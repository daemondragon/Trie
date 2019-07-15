use core::cmp::min;
use core::mem::size_of;

/// Common interface for all distances that can be used on a Trie.
/// The construction of a trie means that the distance can be calculated
/// incrementaly and can be reused over multiple word to prevent computation
/// of the same values multiple times.
/// For instance, for the words ["test", "tests"], the distance of the word "test"
/// can be reused to calculate the distance of the word "tests".
///
/// In this trait, words can be seen as a stack of character that can freely
/// be added and removed.
pub trait IncrementalDistance: core::fmt::Debug {
    /// Add a new character to the previous word
    /// and return the computed distance.
    fn push(&mut self, value: u8) -> usize;

    /// Remove the previously added character.
    /// return true if the character could be removed,
    /// or false if there no more character to pop.
    fn pop(&mut self) -> bool;

    /// Reset the distance to a clean state to calculate the distance
    /// with the given word. This is used to allows reusing the buffer
    /// for multiple iterations.
    fn reset(&mut self, word: &[u8]);

    /// Get the word that is being matched against all the other one.
    fn word(&self) -> &[u8];

    /// Get the current word that is being matched against the word.
    fn current(&self) -> &[u8];

    /// Get the current distance with the given word.
    fn distance(&self) -> usize;

    /// Does calling push more times will keep the distance
    /// under the given threshold
    fn can_continue(&self, max_distance: usize) -> bool;
}

/// Calculate the distance between a word and all words present in a trie.
/// The distance used is the Damerau-Levenshtein distance.
/// This distance use character deletion, insertion, mismatch or transposition.
/// Transposition (switch between to character) is what is added to the
/// Levenshtein distance to get this distance.
///
/// This implementation use the dynamic programming version as a lots
/// of distance will be calculated and this version allows to efficiently
/// cache the computation when used in a trie.
#[derive(Debug, Clone)]
pub struct DamerauLevenshteinDistance {
    /// The word that need to be matched against all the other one.
    word: Vec<u8>,
    /// All the characters that have been previously added and not popped.
    /// They are needed for the transposition part of the algorithm.
    current: Vec<u8>,
    /// The matrix used by the distance to compute the distance between all words.
    /// distances.len() is always >= level * word.len()
    /// It can be greater as the Vec is not resized down when a pop have been done
    /// as it allows to reuse the part when multiple push have been done
    /// without having to resize again the Vec.
    distances: Vec<usize>,
    /// For each rows, was it the minimum in it ?
    /// Used for early stopping to prevent going to far.
    min_distances: Vec<usize>
}

impl DamerauLevenshteinDistance {
    /// Create a new distance calculator for the given word.
    pub fn new(word: &[u8]) -> Self {
        DamerauLevenshteinDistance::new_with_words_len(word, word.len())
    }

    /// Create a new distance calculator for the given word.
    /// The max_words_len parameter allows to specify the length
    /// of the longest word that is expected to be used to calculate the
    /// distance with the original word.
    /// Doing so allows to pre-reserve the capacity of the distance matrix
    /// so that no other resize is needed.
    pub fn new_with_words_len(word: &[u8], max_words_len: usize) -> Self {
        let mut matrix = Vec::with_capacity((word.len() + 1) * (max_words_len + 1));
        (0..=word.len())
            .for_each(|value| matrix.push(value));

        let mut min_distances = Vec::with_capacity(max_words_len + 1);
        min_distances.push(0);//The minimum distances in the first line is 0.

        DamerauLevenshteinDistance {
            word: word.into(),
            current: Vec::with_capacity(max_words_len),
            distances: matrix,
            min_distances: min_distances
        }
    }
}

impl IncrementalDistance for DamerauLevenshteinDistance {

    fn push(&mut self, value: u8) -> usize {
        // Calculating all matrix offset at once.
        let matrix_width = self.word.len() + 1;
        let previous_previous_offset = self.current.len().saturating_sub(1) * matrix_width;
        let previous_offset = self.current.len() * matrix_width;
        let offset = self.current.len().saturating_add(1) * matrix_width;

        self.current.push(value);

        if self.distances.len() <= offset {
            // Resizing the distances matrix if needed so that the new element
            // can be correctly inserted without any problem.
            self.distances.resize_with(offset + matrix_width, Default::default);

            // min_distance grows at the same times as the distance matrix
            self.min_distances.resize_with(self.current.len() + 1, Default::default);
        }

        unsafe
        {
            let mut min_distance = self.current.len();
            *self.distances.get_unchecked_mut(offset) = min_distance;

            for index in 1..matrix_width {
                let cost = (*self.word.get_unchecked(index - 1) != value) as usize;

                let deletion = self.distances.get_unchecked(offset + index - 1) + 1;
                let insertion = self.distances.get_unchecked(previous_offset + index) + 1;
                let substitution = self.distances.get_unchecked(previous_offset + index - 1) + cost;
                let transposition = if index >= 2 && self.current.len() >= 2 &&
                    *self.word.get_unchecked(index - 2) == value && self.word.get_unchecked(index - 1) == self.current.get_unchecked(self.current.len() - 2) {

                    self.distances.get_unchecked(previous_previous_offset + index - 2) + cost
                } else {
                    // Create a big enought value so that only 3 min are needed
                    // instead of 4. Reduce computation needed.
                    self.distances.len()
                };

                // Compute the new distance in the matrix.
                let new_distance = min(min(deletion, insertion), min(substitution, transposition));
                *self.distances.get_unchecked_mut(offset + index) = new_distance;
                // Keep track of the minimum distance to add it later
                min_distance = min(min_distance, new_distance);
            }

            // Set the minimun distance of the line.
            *self.min_distances.get_unchecked_mut(self.current.len() + 1) = min_distance;

            // Get the calculated distances of the new words.
            *self.distances.get_unchecked(offset + matrix_width - 1)
        }
    }

    fn pop(&mut self) -> bool {
        self.current.pop().is_some()
    }

    fn reset(&mut self, word: &[u8]) {
        // Clear all buffer
        self.distances.clear();
        self.min_distances.clear();
        self.current.clear();
        self.word.clear();

        // Reset the distance matrix
        (0..=word.len()).for_each(|value| self.distances.push(value));
        // Reset the min_distance matrix
        self.min_distances.push(0);

        // Set the new wanted word
        self.word.extend_from_slice(word);
    }

    fn word(&self) -> &[u8] {
        &self.word
    }

    fn current(&self) -> &[u8] {
        self.current.as_slice()
    }

    fn distance(&self) -> usize {
        self.distances[self.current.len().saturating_add(1) * self.word.len().saturating_add(1) - 1]
    }

    fn can_continue(&self, max_distance: usize) -> bool {
        let width = self.word.len().saturating_add(1);
        let distance_offset = self.current.len().saturating_add(1) * width - 1;

        *unsafe { self.min_distances.get_unchecked(self.current.len()) } <= max_distance ||
            (self.current.len() >= 2 &&
                self.word.len() >= 2 &&
                self.distances[distance_offset - 2 * width - 2] < max_distance)
    }
}

/// Type use for the bit-vectors of the damerau levenshtein distance.
/// Allows to easily change type for testing or performance purpose.
type DamerauLevenshteinBitType = usize;
/// How many bits vectors are stored in the damerau levenshtein distance per row.
const NB_BIT_VECTORS: usize = 5;

/// Calculate the distance between a word and all words present in a trie.
/// The distance used is the Damerau-Levenshtein distance.
/// This distance is exactly the same as the DamerauLevenshteinDistance one,
/// but use bit-vector algorithm instead for faster computation.
///
/// Reference paper here:
/// https://pdfs.semanticscholar.org/813e/26d8920d17c2afac6bf5a15c537b067a128a.pdf
#[derive(Debug, Clone)]
struct DamerauLevenshteinBitDistance {
    /// The word that need to be matched against all the other one.
    word: Vec<u8>,
    /// All the characters that have been previously added and not popped.
    /// They are needed for the transposition part of the algorithm.
    current: Vec<u8>,
    /// All bit-vectors that are needed for the computation to happend.
    /// Bit-vectors are stored consecutively in this order:
    /// D0: D0j[i] = 1 if D[i,j] = D[i-1,j-1]
    /// HP: HPj[i] = 1 if D[i,j]-D[i,j-1] = 1
    /// HN: HNj[i] = 1 if D[i,j]-D[i,j-1] = -1
    /// VP: VPj[i] = 1 if D[i,j]-D[i-1,j] = 1
    /// VN: VNj[i] = 1 if D[i,j]-D[i-1,j] = -1
    /// Each bit-vectors take the same amount of space,
    /// that is word.len() / size_of::<DamerauLevenshteinBitType>(), and are aligned
    /// on the least significant bit.
    bit_vector: Vec<DamerauLevenshteinBitType>,
    /// For each rows, was it the minimum in it ?
    /// Used for early stopping to prevent going to far.
    min_distances: Vec<usize>,
}

impl DamerauLevenshteinBitDistance {
    /// Create a new distance calculator for the given word.
    pub fn new(word: &[u8]) -> Self {
        DamerauLevenshteinBitDistance::new_with_words_len(word, word.len())
    }

    /// Create a new distance calculator for the given word.
    /// The max_words_len parameter allows to specify the length
    /// of the longest word that is expected to be used to calculate the
    /// distance with the original word.
    /// Doing so allows to pre-reserve the capacity of the distance matrix
    /// so that no other resize is needed.
    pub fn new_with_words_len(word: &[u8], max_words_len: usize) -> Self {
        DamerauLevenshteinBitDistance {
            word: word.into(),
            current: Vec::with_capacity(max_words_len),
            bit_vector: Vec::with_capacity(
                (word.len().next_power_of_two() / size_of::<DamerauLevenshteinBitType>() * NB_BIT_VECTORS)
                    * max_words_len
            ),
            min_distances: Vec::with_capacity(max_words_len)
        }
    }
}

#[cfg(test)]
mod tests {
    use super::{IncrementalDistance, DamerauLevenshteinDistance};

    #[test]
    fn creation() {
        {
            let word = "test";
            let distance = DamerauLevenshteinDistance::new(word.as_bytes());
            // Compare with the correct word
            assert_eq!(distance.word.len(), word.len());
            assert_eq!(0, distance.current.len());
            // The distance matrix have already done some reservation
            assert!(distance.distances.capacity() != 0);
        }

        {
            let word = "test";
            let distance = DamerauLevenshteinDistance::new_with_words_len(word.as_bytes(), 16);
            // Compare with the correct word
            assert_eq!(word.len(), distance.word.len());
            assert_eq!(0, distance.current.len());
            // The distance matrix have correctly reserved the capacity needed.
            assert_eq!(17 * (word.len() + 1), distance.distances.capacity());
        }
    }

    #[test]
    fn distance() {
        for (word_1, word_2, distance) in [
            ("kitten", "sitting", 3),
            ("Saturday", "Sunday", 3),
            ("gifts", "profit", 5),
            ("Something", "Smoething", 1),
            ("Pomatomus", "Pomatomus", 0),
            ("kynar", "kaynar", 1),
            ("kynar", "kayna", 2),
        ].iter() {
            let mut distance_calculator = DamerauLevenshteinDistance::new_with_words_len(word_1.as_bytes(), word_2.len());
            let calculated_distance = word_2
                .as_bytes()
                .iter()
                .map(|value| distance_calculator.push(*value))
                .last().unwrap_or(word_1.len());// The distance with an empty string is the length of the other string.

            assert_eq!(*distance, calculated_distance,
                "Distance between {} and {} is wrong. Got {}, expected {} ({:?})",
                word_1, word_2,
                calculated_distance, distance,
                distance_calculator
            );

            assert_eq!(*distance, distance_calculator.distance(),
                "Distance between {} and {} is wrong. Got {}, expected {} ({:?})",
                word_1, word_2,
                distance_calculator.distance(), distance,
                distance_calculator
            );
        }
    }

    #[test]
    fn reset() {
        let first_word = "hello";
        let second_word = "world";

        let mut distance_calculator = DamerauLevenshteinDistance::new(first_word.as_bytes());
        let calculated_distance = first_word
                .as_bytes()
                .iter()
                .map(|value| distance_calculator.push(*value))
                .last()
                .unwrap();

        // This is the same word
        assert_eq!(0, calculated_distance);

        // Reseting the distance calculator
        distance_calculator.reset(second_word.as_bytes());

        let calculated_distance = first_word
                .as_bytes()
                .iter()
                .map(|value| distance_calculator.push(*value))
                .last()
                .unwrap();

        // The matching word have been changed meanwhile
        assert_ne!(0, calculated_distance);
    }
}