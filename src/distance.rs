use std::cmp::min;

/// Common interface for all distances that can be used on a Trie.
/// The construction of a trie means that the distance can be calculated
/// incrementaly and can be reused over multiple word to prevent computation
/// of the same values multiple times.
/// For instance, for the words ["test", "tests"], the distance of the word "test"
/// can be reused to calculate the distance of the word "tests".
///
/// In this trait, words can be seen as a stack of character that can freely
/// be added and removed.
trait IncrementalDistance {
    /// Add a new character to the previous word
    /// and return the computed distance.
    fn push(&mut self, value: u8) -> usize;

    /// Remove the previously added character.
    /// return true if the character could be removed,
    /// or false if there no more character to pop.
    fn pop(&mut self) -> bool;
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
struct DamerauLevenshteinDistance<'a> {
    /// The word that need to be matched against all the other one.
    word: &'a[u8],
    /// All the characters that have been previously added and not popped.
    /// They are needed for the transposition part of the algorithm.
    current: Vec<u8>,
    /// The matrix used by the distance to compute the distance between all words.
    /// distances.len() is always >= level * word.len()
    /// It can be greater as the Vec is not resized down when a pop have been done
    /// as it allows to reuse the part when multiple push have been done
    /// without having to resize again the Vec.
    distances: Vec<usize>
}

impl <'a> DamerauLevenshteinDistance<'a> {
    /// Create a new distance calculator for the given word.
    pub fn new(word: &'a[u8]) -> Self {
        DamerauLevenshteinDistance::new_with_words_len(word, word.len())
    }

    /// Create a new distance calculator for the given word.
    /// The max_words_len parameter allows to specify the length
    /// of the longest word that is expected to be used to calculate the
    /// distance with the original word.
    /// Doing so allows to pre-reserve the capacity of the distance matrix
    /// so that no other resize is needed.
    pub fn new_with_words_len(word: &'a[u8], max_words_len: usize) -> Self {
        let mut matrix = Vec::with_capacity((word.len() + 1) * (max_words_len + 1));
        (0..=word.len())
            .for_each(|value| matrix.push(value));

        DamerauLevenshteinDistance {
            word: word,
            current: Vec::with_capacity(max_words_len),
            distances: matrix
        }
    }
}

impl <'a> IncrementalDistance for DamerauLevenshteinDistance<'a> {

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
        }

        self.distances[offset] = self.current.len();
        for index in 1..matrix_width {
            let cost = (self.word[index - 1] != value) as usize;

            let deletion = self.distances[offset + index - 1] + 1;
            let insertion = self.distances[previous_offset + index] + 1;
            let substitution = self.distances[previous_offset + index - 1] + cost;
            let transposition = if index >= 2 && self.current.len() >= 2 &&
                self.word[index - 2] == value && self.word[index - 1] == self.current[self.current.len() - 2] {

                self.distances[previous_previous_offset + index - 2] + cost
            } else {
                // Create a big enought value so that only 3 min are needed
                // instead of 4. Reduce computation needed.
                self.distances.len()
            };

            // Compute the new distance in the matrix.
            self.distances[offset + index] = min(min(deletion, insertion), min(substitution, transposition));
        }

        // Get the calculated distances of the new words.
        self.distances[offset + matrix_width - 1]
    }

    fn pop(&mut self) -> bool {
        self.current.pop().is_some()
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
            ("Pomatomus", "Pomatomus", 0)
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
        }
    }
}