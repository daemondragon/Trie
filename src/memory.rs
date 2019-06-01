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

/// Nice interface so that a dynamic sized array of T
/// can be stored in a file without the implementation
/// of search structure having to worries about file access
/// and saving.
/// this interface allows for data structure implementation
/// to create the structure the same ways, while the
/// ways that the information is stocked can change.
#[derive(Debug)]
pub struct Memory<T: Sized> {
    /// The file where the data need to be stored.
    file: File,

    /// How the file memory can be accessed.
    access: MemoryAccess,

    /// A mmap data over the filename
    data: *mut T,

    /// The number of value pointed by the pointer
    length: usize
}

impl <T: Sized> Memory<T> {
    /// Create a new memory that will be saved to the given filename.
    pub fn new(filename: &str, access: MemoryAccess) -> Result<Self, String> {
        Memory::map_file(Memory::<T>::get_file(filename, &access, true)?, access)
    }

    /// Open a memory file from the given file.
    /// The file is expected to be created and contains
    /// the correct type.
    pub fn open(filename: &str, access: MemoryAccess) -> Result<Self, String> {
        Memory::map_file(Memory::<T>::get_file(filename, &access, false)?, access)
    }

    fn get_file(filename: &str, access: &MemoryAccess, allow_create: bool) -> Result<File, String> {
        OpenOptions::new()
            .read(true)
            .write(*access != MemoryAccess::ReadOnly)
            .create(allow_create)
            .open(filename)
            .map_err(|error| format!("Can't create new memory for \"{}\" ({})", filename, error))
    }

    fn map_file(mut file: File, access: MemoryAccess) -> Result<Self, String> {
        let fd = file.as_raw_fd();
        let len = file.seek(SeekFrom::End(0))
                      .map_err(|error| format!("Can't tell file size {}", error))? as usize;

        if (len % size_of::<T>()) != 0 {
            return Err(format!("Invalid file size ({}), not a multiple of {}", len, size_of::<T>()));
        }

        if len != 0 {
            let protection = Memory::<T>::get_protection(&access);

            let ptr = unsafe {
                // Shared as the change need to be reflected on the disk
                mmap(std::ptr::null_mut(), len, protection, 0x0001 /* MAP_SHARED */, fd, 0) as *mut T
            };

            if ptr == (!0 as *mut T) {
                return Err(String::from("Could not mmap, need file with same rights as those requested"));
            }

            Ok(Memory {
                file: file,
                access: access,
                data: ptr,
                length: len / size_of::<T>()
            })

        } else {
            Ok(Memory {
                file: file,
                access: access,
                data: std::ptr::null_mut(),
                length: 0
            })
        }
    }

    /// Add a new value to the end of the memory
    /// Note that the insertion can fail in case
    /// the object could not be added at the end of the file.
    pub fn push(&mut self, value: T) -> Result<(), String> {

        // Update the file
        self.file.seek(SeekFrom::End(0)).expect("Can't go to the end of file for appending");

        unsafe {
                let ptr = &value as *const T as *const u8;
                let buffer = std::slice::from_raw_parts(ptr, size_of::<T>());

                self.file.write_all(buffer).map_err(|error| format!("Can't write to file {}", error))?;
        }

        // Re-update the mmapped version
        let fd = self.file.as_raw_fd();
        let len = self.file.seek(SeekFrom::End(0))
                      .map_err(|error| format!("Can't tell file size {}", error))? as usize;

        if len != (self.length + 1) * size_of::<T>() {
            return Err(format!("Invalid file size ({}) != ({} * {})", len, self.length + 1, size_of::<T>()));
        }

        let protection = Memory::<T>::get_protection(&self.access);

        let ptr = unsafe {
            // Shared as the change need to be reflected on the disk
            mmap(std::ptr::null_mut(), (self.length + 1) * size_of::<T>(), protection,  0x0001 /* MAP_SHARED */, fd, 0) as *mut T
        };

        if ptr == (!0 as *mut T) {
            return Err(String::from("Could not mmap, need file with same rights as those requested"));
        }

        if !self.data.is_null() {
            unsafe {
                munmap(self.data as *mut i8, self.length * size_of::<T>());
            }
        }

        // Update the data information of the data buffer
        self.length += 1;
        self.data = ptr;

        Ok(())
    }

    fn get_protection(access: &MemoryAccess) -> i32 {
        match access {
            MemoryAccess::ReadOnly => 1,
            MemoryAccess::ReadWrite => 3
        }
    }
}

impl <T: Sized> Drop for Memory<T> {
    fn drop(&mut self) {
        unsafe {
            munmap(self.data as *mut i8, self.length * size_of::<T>());
        }
    }
}

impl <T: Sized> Deref for Memory<T> {
    type Target = [T];

    fn deref(&self) -> &Self::Target {
        if self.data.is_null() {
            &[]
        } else {
            unsafe {
                std::slice::from_raw_parts(self.data, self.length)
            }
        }
    }
}

impl <T: Sized> DerefMut for Memory<T> {
    fn deref_mut(&mut self) -> &mut Self::Target {
        debug_assert!(self.access != MemoryAccess::ReadOnly);

        if self.data.is_null() {
            &mut []
        } else {
            unsafe {
                std::slice::from_raw_parts_mut(self.data, self.length)
            }
        }
    }
}