use crate::{Search, Information, WordData};
use crate::memory::DiskMemory;
use crate::distance::IncrementalDistance;

use super::{Entry, get, get_flag, get_char, get_data};

use core::num::NonZeroU32;

pub struct TrieSearch {
    /// The disk memory that is been used to save all the nodes
    memory: DiskMemory,
}

impl TrieSearch {
    pub fn load(filename: &str) -> Result<Self, String> {
        Ok(TrieSearch {
            memory: DiskMemory::open(filename)?
        })
    }
}

impl Search for TrieSearch {
    fn search(&self, distance: &mut IncrementalDistance, max_distance: usize) -> Box<dyn Iterator<Item=WordData>> {
        if max_distance == 0 {
            Box::new(self.exact_search(
                2/* Skip length field and root node */,
                distance.word(),
                distance.word()
            ).into_iter())
        } else {
            let mut result = Vec::new();

            self.distance_search(2/* Skip length field and root node */, distance, max_distance, &mut result);

            result.sort();
            Box::new(result.into_iter())
        }
    }
}

impl TrieSearch {
    fn exact_search(&self, index: usize, word: &[u8], full_word: &[u8]) -> Option<WordData> {
        if word.is_empty() {
            // Need to search for data
            for offset in 0..256 {
                let entry: Entry = *unsafe { get(&self.memory, index + offset) };

                if get_char(entry) == 0 {
                    // Found the data node
                    return Some(WordData {
                        word: full_word.into(),
                        frequency: NonZeroU32::new(get_data(entry)).unwrap(),
                        distance: 0
                    });
                }

                if get_flag(entry) {
                    break;
                }
            }
        } else {
            // Need to search first char and recurse
            for offset in 0..256 {
                let current_index = index + offset;
                let entry: Entry = *unsafe { get(&self.memory, current_index) };

                if get_char(entry) == word[0] {
                    return self.exact_search(current_index + get_data(entry) as usize, &word[1..], full_word);
                }

                if get_flag(entry) {
                    break;
                }
            }
        }

        // Wanted word not found
        None
    }

    fn distance_search(&self, index: usize, distance: &mut IncrementalDistance,
                       max_distance: usize, result: &mut Vec<WordData>) {
        // Need to search for data
        for offset in 0..256 {
            let current_index = index + offset;
            let entry: Entry = *unsafe { get(&self.memory, current_index) };

            let current_c = get_char(entry);
            if current_c == 0 {
                // Data entry
                let current_distance = distance.distance();
                if current_distance <= max_distance {
                    result.push(WordData {
                        word: distance.current().into(),
                        frequency: NonZeroU32::new(get_data(entry)).unwrap(),
                        distance: current_distance
                    });
                }
            } else {
                // Link entry
                distance.push(current_c);
                if distance.can_continue(max_distance) {
                    self.distance_search(current_index + get_data(entry) as usize, distance, max_distance, result);
                }
                distance.pop();
            }

            if get_flag(entry) {
                break;
            }
        }
    }
}