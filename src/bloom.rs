use std::marker::PhantomData;

/// A bloom filter to fastly check if the element
/// is not present in a collection.
#[derive(Debug, Clone)]
pub struct BloomFilter<T> {
    /// The number of bits in the bloom
    /// filter is always a multiple of 8
    /// so that there is no need to keep more information
    /// as the len of the vector is already known.
    bits: Vec<u8>,

    /// As the type of the data is not used
    /// in the struture.
    _phantom: PhantomData<T>
}

impl <T> BloomFilter<T> {
    /// Create a bloom filter with an array of nb_bytes size.
    pub fn new(nb_bytes: usize) -> Self {
        debug_assert!(nb_bytes > 0);

        BloomFilter {
            bits: vec![0; nb_bytes],
            _phantom: PhantomData
        }
    }

    /// Add the given value in the bloom filter
    /// so that it known that the value have been added
    /// to the collection.
    pub fn add(&mut self, value: &T) {
        let bits_len = self.bits.len();

        BloomFilter::hash(value)
            .iter()
            // Mark the bits as present
            .for_each(|index| self.bits[(index / 8) % bits_len] |= 1 << (index % 8))
    }

    /// Does the bloom filter think that the element is present.
    /// Note that if true is returned, the element might not be present,
    /// but if false is returned, the element is never present.
    pub fn contains(&self, value: &T) -> bool {
        let bits_len = self.bits.len();

        BloomFilter::hash(value)
            .iter()
            .all(|index| (self.bits[(index / 8) % bits_len] & (1 << (index % 8))) != 0)
    }

    fn hash(_value: &T) -> [usize; 8] {
        // TODO: hash value with multiple hash function
        [1, 2, 3, 4, 5, 6, 7, 8]
    }
}

#[cfg(test)]
mod tests {
    use super::BloomFilter;

    #[test]
    fn creation() {
        let bloom: BloomFilter<u8> = BloomFilter::new(16);
        // Use bytes instead of bits.
        assert_eq!(16, bloom.bits.len());
        // Everything is set to 0.
        assert!(bloom.bits.iter().all(|value| *value == 0));
    }

    #[test]
    fn add() {
        let mut bloom: BloomFilter<u8> = BloomFilter::new(16);

        assert!(!bloom.contains(&3));

        bloom.add(&3);

        assert!(bloom.bits.iter().any(|value| *value != 0));
        assert!(bloom.contains(&3));
    }


}