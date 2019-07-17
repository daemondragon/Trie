use std::os::unix::io::AsRawFd;
use std::fs::File;
use std::io::{Seek, SeekFrom};

// All C function that need to be called.
extern {
    #[cfg_attr(all(target_os = "macos", target_arch = "x86"), link_name = "mmap$UNIX2003")]
    fn mmap(addr: *mut i8, len: usize, prot: i32, flags: i32, fd: i32, offset: isize) -> *mut i8;
    #[cfg_attr(all(target_os = "macos", target_arch = "x86"), link_name = "munmap$UNIX2003")]
    fn munmap(addr: *mut i8, len: usize) -> i32;
}

/// A memory using a mmap-ed file directly.
/// Allows to have a memory bigger than the limit
/// imposed by the project without having to do weird
/// trick to load and unload manually.
/// The disk memory is read only to prevent error.
#[derive(Debug)]
pub struct DiskMemory {
    /// The file where the data need to be stored
    file: File,
    /// The mmap-ed file
    data: *const u8,
    /// The length of the array pointed by data
    length: usize,
}

impl DiskMemory {
    /// Open a memory file from the given file.
    /// The file is expected to be created and contains
    /// the correct type.
    pub fn open(filename: &str) -> Result<Self, String> {
        let mut file = File::open(filename)
                                .map_err(|error| format!("Can't create new memory for \"{}\" ({})", filename, error))?;


        let fd = file.as_raw_fd();
        let len = file.seek(SeekFrom::End(0))
                      .map_err(|error| format!("Can't tell file size {}", error))? as usize;

        let ptr: *mut u8;

        if len != 0 {
            ptr = unsafe {
                mmap(std::ptr::null_mut(), len, 1 /*Read Only*/, 0x0002 /* MAP_PRIVATE */, fd, 0) as *mut u8
            };

            if ptr == !0 as *mut u8 {
                return Err(String::from("Could not mmap, need file with same rights as those requested"));
            }
        } else {
            ptr = std::mem::align_of::<u8>() as *mut u8;
        }

        Ok(DiskMemory {
            file: file,
            data: ptr,
            length: len
        })
    }

    pub fn len(&self) -> usize {
        self.length
    }

    pub fn data(&self) -> *const u8 {
        self.data
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