use core::marker::PhantomData;
use core::cmp::max;


use fasthash::{sea, xx, metro, murmur3};

/// A struct is hashable if it can be converted to a bytes array
/// so that all hash function of the BloomFilter can be used on.
pub trait Hashable {
    /// Get the bytes that will be hashed from the
    /// implementing struct.
    /// The function must always return the same things
    /// as it will be used for retrieval.
    /// The returned value can be cached from the calling site if needed.
    fn bytes(&self) -> &[u8];
}

/// A bloom filter to fastly check if the element
/// is not present in a collection.
#[derive(Debug, Clone)]
pub struct BloomFilter<T: Hashable> {
    /// The number of bits in the bloom
    /// filter is always a multiple of 8
    /// so that there is no need to keep more information
    /// as the len of the vector is already known.
    bytes: Vec<u8>,

    /// As the type of the data is not used in the struture.
    _phantom: PhantomData<T>
}

impl <T: Hashable> BloomFilter<T> {
    /// Creates a bloom filter with an array of nb_bytes size.
    /// All bloom filters use a bits aray that is a multiple of 8
    /// so that bytes can be used instead of bits, allowing for bigger
    /// if needed.
    pub fn new(nb_bytes: usize) -> Self {
        debug_assert!(nb_bytes > 0);

        BloomFilter {
            bytes: vec![0; nb_bytes],
            _phantom: PhantomData
        }
    }

    /// Creates a bloom filter of the needed size so that
    /// the filter can hold the expected number of elements
    /// with a false positivie rate of rate (lower is better).
    pub fn with(expected_elements: u64, rate: f64) -> Self {
        debug_assert!(expected_elements > 0);
        debug_assert!(rate > 0.0);
        debug_assert!(rate < 1.0);

        let ln_2_squared = 2.0_f64.ln().powi(2);

        let nb_bits_required = (((-(expected_elements as f64) * rate.ln()) / ln_2_squared).round()) as usize;

        // Need to convert from bits to bytes.
        BloomFilter::new(max(2, nb_bits_required / 8))
    }

    /// Get the buffer of bytes currently
    /// used by the bloom filter. This methods allows
    /// for easy serialisation if needed.
    pub fn bytes(&self) -> &[u8] {
        &self.bytes
    }

    /// Add the given value in the bloom filter
    /// so that it known that the value have been added
    /// to the collection.
    pub fn add(&mut self, value: &T) {
        let bytes_len = self.bytes.len();

        BloomFilter::hash(value)
            .iter()
            // Mark the bits as present
            .for_each(|index| self.bytes[(index / 8) % bytes_len] |= 1 << (index % 8))
    }

    /// Does the bloom filter think that the element is present.
    /// Note that if true is returned, the element might not be present,
    /// but if false is returned, the element is never present.
    pub fn contains(&self, value: &T) -> bool {
        let bytes_len = self.bytes.len();

        BloomFilter::hash(value)
            .iter()
            .all(|index| (self.bytes[(index / 8) % bytes_len] & (1 << (index % 8))) != 0)
    }

    fn hash(value: &T) -> [usize; 4] {
        // Caching the value as it could be computed
        // each times the function is called.
        let bytes = value.bytes();

        [
            sea::hash64(bytes) as usize,
            xx::hash64(bytes) as usize,
            metro::hash64(bytes) as usize,
            murmur3::hash32(bytes) as usize
        ]
    }
}

/// Create a new bloom filter from the given bytes.
/// The given bytes are copied to a new vector.
/// Implementing this allows for deserialisation of the BloomFilter
/// after it as been serialised with bytes().
impl <T: Hashable> From<&[u8]> for BloomFilter<T> {
    fn from(bytes: &[u8]) -> Self {
        BloomFilter {
            bytes: bytes.into(),
            _phantom: PhantomData
        }
    }
}

#[cfg(test)]
mod tests {
    use super::{BloomFilter, Hashable};

    #[derive(Debug)]
    struct PackedStruct {
        value: u8
    }

    impl Hashable for PackedStruct {
        fn bytes(&self) -> &[u8] {
            unsafe {
                ::std::slice::from_raw_parts(&self.value as *const u8, ::std::mem::size_of::<u8>())
            }
        }
    }

    #[test]
    fn basic_creation() {
        let bloom: BloomFilter<PackedStruct> = BloomFilter::new(16);
        // Correct size of the bloom filter.
        assert_eq!(16, bloom.bytes.len());
        // Everything is set to 0.
        assert!(bloom.bytes.iter().all(|value| *value == 0));
    }

    #[test]
    fn accurate_creation() {
        let bloom: BloomFilter<PackedStruct> = BloomFilter::with(216553, 0.01);
        // The expected number of bytes that the bloom filter must takes is 259461.
        // However, beacuse of rounding error, an interval of +- 10 is used
        assert!((259461 - bloom.bytes.len() as isize).abs() < 10);
    }

    #[test]
    fn add() {
        let mut bloom: BloomFilter<PackedStruct> = BloomFilter::new(16);

        let value = PackedStruct { value: 3 };

        assert!(!bloom.contains(&value));

        bloom.add(&value);

        assert!(bloom.bytes.iter().any(|value| *value != 0));
        assert!(bloom.contains(&value));
    }
}