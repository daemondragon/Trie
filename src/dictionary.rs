use std::io::{BufReader, BufRead};
use std::fs::File;
use std::num::NonZeroU32;

/// Load a dictionary file containing word
/// and their associated frequency and return
/// an iterator over them.
///
/// A dictionary file contains for each lines (in this order):
/// - a word (without space)
/// - a tabulation (\t)
/// - the frequency associated with the word
pub struct Dictionary {
    buffer: BufReader<File>
}

/// Represent a line in the dictionary file.
/// It is what the dictionary iterate over.
#[derive(PartialEq, Eq, PartialOrd, Ord)]
pub struct DictionaryLine {
    pub word: String,
    pub frequency: NonZeroU32
}

impl Dictionary {
    /// Create a dictionary for the given file.
    pub fn new(filename: &str) -> Result<Self, String> {
        let file = File::open(filename).map_err(|error| format!("Can't read dictionary file {} ({})", filename, error))?;

        Ok(Dictionary {
            buffer: BufReader::new(file)
        })
    }
}

impl IntoIterator for Dictionary {
    type Item = DictionaryLine;
    type IntoIter = DictionaryIterator;

    /// Transform the dictionary into an iterator.
    fn into_iter(self) -> Self::IntoIter {
        DictionaryIterator {
            iter: self.buffer.lines()
        }
    }
}

/// An iterator over the dictionary line.
pub struct DictionaryIterator {
    iter: std::io::Lines<std::io::BufReader<std::fs::File>>
}

impl Iterator for DictionaryIterator {
    type Item = DictionaryLine;

    fn next(&mut self) -> Option<Self::Item> {
        self.iter
            .next()
            .map(|line| {
                let line = line.expect("Could not read line");
                let mut splitted = line.split_whitespace();

                DictionaryLine {
                    word: splitted.next().expect(&format!("No word in the line: \"{}\"", line)).into(),
                    frequency: NonZeroU32::new(
                        str::parse(
                            splitted.next().expect(&format!("No frequency in the line: \"{}\"", line))
                        ).expect("Second word is not a number")
                    ).expect("Frequency is not non zero")
                }
            })
    }
}