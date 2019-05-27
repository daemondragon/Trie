//! A basic trie that is used as the reference
//! for all other search structure.
//! The baseline that this trie offer
//! is to be sure that other structure
//! don't perform worse than this structure.

use core::num::NonZeroUsize;
use std::fs::{File, OpenOptions};

use super::{Compiler, Search, WordData, WordFrequency};
use crate::distance::{IncrementalDistance, DamerauLevenshteinDistance};
use crate::memory::{Memory, MemoryAccess};

/// A very basic node of the trie.
#[repr(C)]
struct MiniNode {
    /// The associated data of the word,
    /// if the node represent a valid word.
    data: Option<WordFrequency>,

    /// The index of the node children if they exist.
    children: [Option<NonZeroUsize>; 256]
}

/// Create a basic trie and write
/// all the nodes to the files when it's done.
pub struct MiniCompiler {
    nodes: Memory<MiniNode>
}

impl MiniCompiler {
    pub fn new(filename: &str) -> Self {
        let file = OpenOptions::new()
            .read(true)
            .write(true)
            .create(true)
            .open(filename)
            .expect("Can't create file");

        let mut memory = Memory::new(file, MemoryAccess::ReadWrite).expect("Can't create file based memory");

        memory.push(MiniNode {
            data: None,
            children: [None; 256]
        }).unwrap();

        MiniCompiler {
            nodes: memory
        }
    }

    fn add_rec<'a>(&mut self, node_index: usize, word: &[u8], data: WordFrequency) {
        if !word.is_empty() {
            // In the middle of the word.
            if self.nodes[node_index].children[word[0] as usize].is_none() {
                // Node absent, add it and treat it as added.
                self.nodes.push(MiniNode {
                    data: None,
                    children: [None; 256]
                }).unwrap();

                self.nodes[node_index].children[word[0] as usize] = NonZeroUsize::new(self.nodes.len() - 1);
            }

            self.add_rec(self.nodes[node_index].children[word[0] as usize].unwrap().get(), &word[1..], data);
        } else {
            self.nodes[node_index].data = Some(data);
        }
    }
}

impl Compiler for MiniCompiler {
    fn add<'a>(&mut self, word: &[u8], data: WordFrequency) {
        self.add_rec(0, word, data);
    }
}

pub struct MiniSearch {
    memory: Memory<MiniNode>
}

impl MiniSearch {
    pub fn load(filename: &str) -> Result<Self, String> {
        let file = File::open(filename)
            .map_err(|error| format!("Can't open file {}", error))?;

        Ok(MiniSearch {
            memory: Memory::new(file, MemoryAccess::ReadOnly)?
        })
    }
}

impl <'a> Search<'a> for MiniSearch {
    fn search(&'a self, word: &'a [u8], distance: usize) -> Box<dyn Iterator<Item=WordData> + 'a> {
        Box::new(MiniSearchIterator::<'a> {
            memory: &self.memory,
            parents: vec![
                MiniSearchIteratorIndex {
                    node_index: 0,
                    next_word_index: 0
                }
            ],
            distance_calculator: DamerauLevenshteinDistance::new(word),
            distance: distance
        })
    }
}

#[derive(Debug)]
struct MiniSearchIteratorIndex {
    node_index: usize,
    next_word_index: usize
}

struct MiniSearchIterator<'a> {
    memory: &'a Memory<MiniNode>,
    parents: Vec<MiniSearchIteratorIndex>,
    distance_calculator: DamerauLevenshteinDistance<'a>,
    distance: usize
}

impl <'a> Iterator for MiniSearchIterator<'a> {
    type Item=WordData;

    fn next(&mut self) -> Option<Self::Item> {

        while !self.parents.is_empty() {

            // Remove the impossible node
            while self.parents.last()?.next_word_index == 256 {
                self.parents.pop();
                self.distance_calculator.pop();
            }

            // Read node
            let node = &self.memory[self.parents.last()?.node_index];

            // Find the next used node
            while self.parents.last()?.next_word_index < 256 && node.children[self.parents.last()?.next_word_index].is_none() {
                self.parents.last_mut()?.next_word_index += 1;
            }

            // No node have been found in the current node, retrying.
            if self.parents.last()?.next_word_index == 256 {
                continue;
            }

            let calculated_distance = self.distance_calculator.push(self.parents.last()?.next_word_index as u8);
            self.parents.last_mut()?.next_word_index += 1;


            // Distance is too big, retrying with the next node.
            /*
            TODO: fix this for small word.
            if calculated_distance > self.distance {
                self.distance_calculator.pop();
                continue;
            }
            */

            // Go to the next node.
            let children_node_index = node.children[self.parents.last()?.next_word_index - 1].unwrap().get();
            let children_node = &self.memory[children_node_index];

            self.parents.push(MiniSearchIteratorIndex {
                node_index: children_node_index,
                next_word_index: 0
            });

            // This is a valid node, return it.
            if calculated_distance <= self.distance {
                if let Some(data) = &children_node.data {
                    return Some(WordData {
                        word: self.distance_calculator.current().to_vec(),
                        data: *data,
                        distance: calculated_distance
                    });
                }
            }
        }

        // End of iterator, nothing more to do.
        None
    }
}