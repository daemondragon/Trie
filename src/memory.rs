use core::ops::{Deref, DerefMut};

/// Nice interface so that a dynamic sized array of T
/// can be stored in a file without the implementation
/// of search structure having to worries about file access
/// and saving.
/// this interface allows for data structure implementation
/// to create the structure the same ways, while the
/// ways that the information is stocked can change.
#[derive(Debug, Clone)]
pub struct Memory<T: Sized> {
    data: Vec<T>
}

impl <T: Sized> Memory<T> {
    /// Load a memory from the given filename.
    pub fn open(filename: &str) -> Result<Self, String> {
        Ok(Memory {
            data: Vec::new()
        })
    }

    /// Create a new memory that will be saved
    /// to the given filename.
    pub fn new(filename: &str) -> Self {
        Memory {
            data: Vec::new()
        }
    }

    /// Add a new value to the end of the memory
    /// Note that the insertion can fail in case
    /// the object could not be added at the end of the file.
    pub fn push(&mut self, value: T) -> Result<(), T> {
        self.data.push(value);
        Ok(())
    }

    /// Remove the last value at the end of the memory.
    /// The deletion can fail if the file could not be accessed.
    pub fn pop(&mut self) -> Result<Option<T>, String> {
        Ok(self.data.pop())
    }
}

impl <T: Sized> Deref for Memory<T> {
    type Target = [T];

    fn deref(&self) -> &Self::Target {
        &self.data
    }
}

impl <T: Sized> DerefMut for Memory<T> {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.data
    }
}