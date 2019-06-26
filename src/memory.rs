use core::ops::{Deref, DerefMut};

use std::os::unix::io::AsRawFd;
use std::mem::size_of;
use std::fs::{File, OpenOptions};
use std::io::{Write, Seek, SeekFrom};

// All C function that need to be called.
extern {
    #[cfg_attr(all(target_os = "macos", target_arch = "x86"), link_name = "mmap$UNIX2003")]
    fn mmap(addr: *mut i8, len: usize, prot: i32, flags: i32, fd: i32, offset: isize) -> *mut i8;
    #[cfg_attr(all(target_os = "macos", target_arch = "x86"), link_name = "munmap$UNIX2003")]
    fn munmap(addr: *mut i8, len: usize) -> i32;
}

/// The memory access that is needed
/// for the memory to work. Using a memory access
//// of ReadOnly and dereferencing with mutbility will panic.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum MemoryAccess {
    ReadOnly,
    ReadWrite
}

/// A memory using a mmap-ed file directly.
/// Allows to have a memory bigger than the limit
/// imposed by the project without having to do weird
/// trick to load and unload manually.
#[derive(Debug)]
pub struct DiskMemory {
    /// The file where the data need to be stored
    file: File,
    /// How the file memory can be accessed
    access: MemoryAccess,
    /// The mmap-ed file
    data: *mut u8,
    /// The length of the array pointed by data
    length: usize,
}

impl DiskMemory {
    /// Create a new memory that will be saved to the given filename.
    pub fn new(filename: &str, access: MemoryAccess) -> Result<Self, String> {
        let mut memory = DiskMemory {
            file: DiskMemory::get_file(filename, &access, true)?,
            access: access,
            data: std::ptr::null_mut(),
            length: 0
        };

        memory.map_file()?;
        Ok(memory)
    }

    /// Open a memory file from the given file.
    /// The file is expected to be created and contains
    /// the correct type.
    pub fn open(filename: &str, access: MemoryAccess) -> Result<Self, String> {
        let mut memory = DiskMemory {
            file: DiskMemory::get_file(filename, &access, false)?,
            access: access,
            data: std::ptr::null_mut(),
            length: 0
        };

        memory.map_file()?;
        Ok(memory)
    }

    pub fn len(&self) -> usize {
        self.length
    }

    pub fn file(&self) -> &File {
        &self.file
    }

    pub fn data(&self) -> *const u8 {
        self.data
    }

    pub fn data_mut(&self) -> *mut u8 {
        self.data
    }

    /// Add a new value to the end of the memory
    /// Note that the insertion can fail in case
    /// the object could not be added at the end of the file.
    /// The object can be added multiple times at the end of the buffer
    /// inc ase the user want to prevent multiple memory mapping.
    pub fn push<T: Sized>(&mut self, value: T, amount: usize) -> Result<(), String> {
        debug_assert!(self.access == MemoryAccess::ReadWrite);

        // Update the file
        self.file.seek(SeekFrom::End(0)).expect("Can't go to the end of file for appending");

        unsafe {
            let ptr = &value as *const T as *const u8;
            let buffer = std::slice::from_raw_parts(ptr, size_of::<T>());

            for _ in 0..amount {
                self.file.write_all(buffer).map_err(|error| format!("Can't write to file {}", error))?;
            }
        }

        // Re-update the mmapped version
        self.map_file()
    }

    fn get_file(filename: &str, access: &MemoryAccess, allow_create: bool) -> Result<File, String> {
        OpenOptions::new()
            .read(true)
            .write(*access != MemoryAccess::ReadOnly)
            .create(allow_create)
            .open(filename)
            .map_err(|error| format!("Can't create new memory for \"{}\" ({})", filename, error))
    }

    fn map_file(&mut self) -> Result<(), String> {
        let fd = self.file.as_raw_fd();
        let len = self.file.seek(SeekFrom::End(0))
                      .map_err(|error| format!("Can't tell file size {}", error))? as usize;

        let ptr: *mut u8;

        if len != 0 {
            let protection = DiskMemory::get_protection(&self.access);

            ptr = unsafe {
                // Shared as the change need to be reflected on the disk
                mmap(std::ptr::null_mut(), len, protection, 0x0001 /* MAP_SHARED */, fd, 0) as *mut u8
            };

            if ptr == !0 as *mut u8 {
                return Err(String::from("Could not mmap, need file with same rights as those requested"));
            }
        } else {
            ptr = std::mem::align_of::<u8>() as *mut u8;
        }

        if self.length != 0 {
            unsafe {
                munmap(self.data as *mut i8, self.length);
            }
        }

        self.data = ptr;
        self.length = len;

        Ok(())
    }

    fn get_protection(access: &MemoryAccess) -> i32 {
        match access {
            MemoryAccess::ReadOnly => 1,
            MemoryAccess::ReadWrite => 3
        }
    }
}

impl Drop for DiskMemory {
    fn drop(&mut self) {
        if self.length != 0 {
            unsafe {
                munmap(self.data as *mut i8, self.length);
            }
        }
    }
}

/// Nice interface so that a dynamic sized array of T
/// can be stored in a file without the implementation
/// of search structure having to worries about file access
/// and saving.
/// this interface allows for data structure implementation
/// to create the structure the same ways, while the
/// ways that the information is stocked can change.
#[derive(Debug)]
pub struct Memory<T: Sized> {
    memory: DiskMemory,

    _phantom: std::marker::PhantomData<T>
}

impl <T: Sized> Memory<T> {
    /// Create a new memory that will be saved to the given filename.
    pub fn new(filename: &str, access: MemoryAccess) -> Result<Self, String> {
        let memory = DiskMemory::new(filename, access)?;

        if (memory.length % size_of::<T>()) != 0 {
            return Err(format!("Invalid file size ({}), not a multiple of {}", memory.length, size_of::<T>()));
        }

        Ok(Memory {
            memory: memory,
            _phantom: std::marker::PhantomData
        })
    }

    /// Open a memory file from the given file.
    /// The file is expected to be created and contains
    /// the correct type.
    pub fn open(filename: &str, access: MemoryAccess) -> Result<Self, String> {
        let memory = DiskMemory::open(filename, access)?;

        if (memory.length % size_of::<T>()) != 0 {
            return Err(format!("Invalid file size ({}), not a multiple of {}", memory.length, size_of::<T>()));
        }

        Ok(Memory {
            memory: memory,
            _phantom: std::marker::PhantomData
        })
    }

    /// Add a new value to the end of the memory
    /// Note that the insertion can fail in case
    /// the object could not be added at the end of the file.
    pub fn push(&mut self, value: T) -> Result<(), String> {
        self.memory.push(value, 1)?;

        if (self.memory.length % size_of::<T>()) != 0 {
            Err(format!("Invalid file size ({}), not a multiple of {}", self.memory.length, size_of::<T>()))
        } else {
            Ok(())
        }
    }
}

impl <T: Sized> Deref for Memory<T> {
    type Target = [T];

    fn deref(&self) -> &Self::Target {
        unsafe {
            std::slice::from_raw_parts(self.memory.data as *const T, self.memory.length / size_of::<T>())
        }
    }
}

impl <T: Sized> DerefMut for Memory<T> {
    fn deref_mut(&mut self) -> &mut Self::Target {
        debug_assert!(self.memory.access != MemoryAccess::ReadOnly);

        unsafe {
            std::slice::from_raw_parts_mut(self.memory.data as *mut T, self.memory.length / size_of::<T>())
        }
    }
}