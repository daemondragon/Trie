/// Represent the presence or absence of a list of flags.
/// Flags are compacted to used the least amount of memory as possible
/// so that it can be used and not take the whole memory budget.
pub struct Flags {
    /// How many flags are present.
    /// Don't use flags.len() as it represent
    /// the number of flags rounded up to the next multiple of 8.
    length: usize,
    /// A vec of flags.
    /// Each flags can be accessed by its index (starting from 0).
    flags: Vec<u8>
}

impl Flags {
    /// Create a list of flags where all flags are absents.
    pub fn new(size: usize) -> Self {
        Flags {
            length: size,
            flags: vec![0; size / 8 + if size % 8 != 0 { 1 } else { 0 }]
        }
    }

    /// Get if the flags is present at the given index
    pub fn get(&self, index: usize) -> bool {
        debug_assert!(index < self.length);

        (self.flags[index / 8] & (1 << (index % 8))) != 0
    }

    /// Set the flags presence at the given index
    pub fn set(&mut self, index: usize, value: bool) {
        debug_assert!(index < self.length);

        if value {
            self.flags[index / 8] |= 1 << (index % 8);
        } else {
            self.flags[index / 8] &= !(1 << (index % 8));
        }
    }
}

#[cfg(test)]
mod tests {
    use super::Flags;

    #[test]
    fn create() {
        // Compact correctly
        assert_eq!(1, Flags::new(1).flags.len());
        // Doesn't take one byte too much of data
        assert_eq!(1, Flags::new(8).flags.len());
        // Overflow correctly
        assert_eq!(2, Flags::new(9).flags.len());
        assert_eq!(2, Flags::new(10).flags.len());
    }

    #[test]
    fn get() {
        let flags = Flags::new(23);

        for index in 0..23 {
            assert_eq!(false, flags.get(index));
        }
    }

    #[test]
    fn set() {
        let mut flags = Flags::new(13);

        flags.set(1, true);
        // Set the correct bit
        assert_eq!(true, flags.get(1));
        // Doesn't modify next bit by error
        assert_eq!(false, flags.get(0));
        assert_eq!(false, flags.get(2));

        flags.set(2, false);
        assert_eq!(false, flags.get(2));

        flags.set(2, true);
        assert_eq!(true, flags.get(2));
    }
}