//! A basic trie that is used as the reference
//! for all other search structure.
//! The baseline that this trie offer
//! is to be sure that other structure
//! don't perform worse than this structure.

use core::num::NonZeroUsize;

use crate::{Compiler, Search, Information, WordData, WordFrequency};
use crate::distance::IncrementalDistance;
use crate::memory::{Memory, MemoryAccess};

/// A very basic node of the trie.
#[repr(C)]
struct MiniNode {
    /// The associated frequency of the word,
    /// if the node represent a valid word.
    frequency: Option<WordFrequency>,

    /// The index of the node children if they exist.
    /// The index is non-zero as the root node can't be referenced.
    children: [Option<NonZeroUsize>; 256]
}

/// Create a basic trie and write
/// all the nodes to the files when it's done.
pub struct MiniCompiler {
    nodes: Memory<MiniNode>
}

impl MiniCompiler {
    pub fn new(filename: &str) -> Self {
        let mut memory = Memory::new(filename, MemoryAccess::ReadWrite).expect("Can't create file based memory");

        memory.push(MiniNode {
            frequency: None,
            children: [None; 256]
        }).unwrap();

        MiniCompiler {
            nodes: memory
        }
    }

    fn add_rec<'a>(&mut self, node_index: usize, word: &[u8], frequency: WordFrequency) {
        if !word.is_empty() {
            // In the middle of the word.
            if self.nodes[node_index].children[word[0] as usize].is_none() {
                // Node absent, add it and treat it as added.
                self.nodes.push(MiniNode {
                    frequency: None,
                    children: [None; 256]
                }).unwrap();

                self.nodes[node_index].children[word[0] as usize] = NonZeroUsize::new(self.nodes.len() - 1);
            }

            self.add_rec(self.nodes[node_index].children[word[0] as usize].unwrap().get(), &word[1..], frequency);
        } else {
            self.nodes[node_index].frequency = Some(frequency);
        }
    }
}

impl Compiler for MiniCompiler {
    fn add<'a>(&mut self, word: &[u8], frequency: WordFrequency) {
        self.add_rec(0, word, frequency);
    }

    fn build(self) {
        // Nothing need to be done as all the node are already written to the disk.
    }
}

pub struct MiniSearch {
    memory: Memory<MiniNode>
}

impl MiniSearch {
    pub fn load(filename: &str) -> Result<Self, String> {
        Ok(MiniSearch {
            memory: Memory::open(filename, MemoryAccess::ReadOnly)?
        })
    }

    fn exact_search(&self, node_index: usize, word: &[u8], full_word: &[u8]) -> Option<WordData> {
        if let Some(children_node_index) = self.memory[node_index].children[word[0] as usize] {
            if word.len() <= 1 {
                if let Some(frequency) = self.memory[children_node_index.get()].frequency {
                    Some(WordData {
                        word: full_word.into(),
                        frequency: frequency,
                        distance: 0
                    })
                } else {
                    // No frequency (word not present)
                    None
                }
            } else {
                // Need to keep searching deeper
                self.exact_search(children_node_index.get(), &word[1..], full_word)
            }
        } else {
            None// No children leading to the wanted node
        }
    }
}

impl Search for MiniSearch {
    fn search(&self, distance: &mut IncrementalDistance, max_distance: usize) -> Box<dyn Iterator<Item=WordData>> {
        if max_distance == 0 {
            Box::new(self.exact_search(0, distance.word(), distance.word()).into_iter())
        } else {
            let mini_search = MiniSearchIterator {
                memory: &self.memory,
                parents: vec![
                    MiniSearchIteratorIndex {
                        node_index: 0,
                        next_word_index: 0,
                        distance: distance.word().len()
                    }
                ],
                distance_calculator: distance,
                max_distance: max_distance
            };

            let mut result = mini_search.collect::<Vec<WordData>>();
            result.sort();
            Box::new(result.into_iter())
        }
    }
}

#[derive(Debug)]
struct MiniSearchIteratorIndex {
    node_index: usize,
    next_word_index: usize,
    // What was the distance before for early stopping.
    distance: usize
}

struct MiniSearchIterator<'a> {
    memory: &'a Memory<MiniNode>,
    parents: Vec<MiniSearchIteratorIndex>,
    distance_calculator: &'a mut IncrementalDistance,
    max_distance: usize
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


            // Distance is too big, retrying with the next node (and distance is increasing, not decreasing).
            if calculated_distance > self.max_distance && calculated_distance > self.parents.last()?.distance {
                self.distance_calculator.pop();
                continue;
            }

            // Go to the next node.
            let children_node_index = node.children[self.parents.last()?.next_word_index - 1].unwrap().get();
            let children_node = &self.memory[children_node_index];

            self.parents.push(MiniSearchIteratorIndex {
                node_index: children_node_index,
                next_word_index: 0,
                distance: calculated_distance,
            });

            // This is a valid node, return it.
            if calculated_distance <= self.max_distance {
                if let Some(frequency) = &children_node.frequency {
                    return Some(WordData {
                        word: self.distance_calculator.current().to_vec(),
                        frequency: *frequency,
                        distance: calculated_distance
                    });
                }
            }
        }

        // End of iterator, nothing more to do.
        None
    }
}

impl Information for MiniSearch {
    fn words(&self) -> usize {
        self.words_rec(0)
    }

    fn nodes(&self) -> usize {
        self.memory.len()
    }

    fn height(&self) -> usize {
        self.height_rec(0)
    }

    fn max_lenght(&self) -> usize {
        // Node don't compress path, so the longest word's length
        // is the height of the trie
        self.height()
    }

    fn graph(&self) {
        println!("digraph G {{");

        self.graph_rec(0);

        println!("}}");
    }
}

impl MiniSearch {
    fn words_rec(&self, node_index: usize) -> usize {
        let count: usize = if self.memory[node_index].frequency.is_some() { 1 } else { 0 };

        let children_count: usize = self.memory[node_index]
                                        .children
                                        .iter()
                                        .filter(|child| child.is_some())
                                        .map(|child| self.words_rec(child.unwrap().get()))
                                        .sum();

        count + children_count
    }

    fn height_rec(&self, node_index: usize) -> usize {
        1 + self.memory[node_index].children
                .iter()
                .filter(|child| child.is_some())
                .map(|child| self.height_rec(child.unwrap().get()))
                .max()
                .unwrap_or(0)
    }

    fn graph_rec(&self, node_index: usize) {
        print!("{} [", node_index);

        if let Some(frequency) = self.memory[node_index].frequency {
            print!("label=\"{}\", color=green, style=filled", frequency.get());
        } else {
            print!("label=\"\"");
        }

        println!("];");

        for index in 0..256 {
            if let Some(children_node_index) = self.memory[node_index].children[index] {
                println!("{} -> {} [label=\"{}\"];", node_index, children_node_index, index as u8 as char);
                self.graph_rec(children_node_index.get());
            }
        }
    }
}