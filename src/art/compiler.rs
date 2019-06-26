use crate::{Compiler, WordFrequency};
use crate::memory::{DiskMemory, MemoryAccess};

use super::{NodeKind, NodeHeader, NodeOffset, Node0, Node4, Node16, Node48, Node256};

use core::mem::size_of;

/// The compiler compile the ART structure into a stored
/// equivalent that can be directly used.
/// For simplicity, the compiler work in three phases:
/// -   The first phase, where word are added, when a node need
///     to be added, the Node256 is used instead
///     (all other nodes are never used)
/// -   The second phase, where path are compressed
///     to reduce the number of nodes used
/// -   The last phase is the node compression phase.
///     Node are compressed using the correct equivalent and rewritten
///     so that the structure takes less memory space.
pub struct ArtCompiler {
    /// The disk memory that is been used to save all the nodes
    memory: DiskMemory,
    /// How many nodes are really in memory
    /// (nodes are added in bulk to prevent mapping the file too many times.
    nb_nodes: usize,
}

impl ArtCompiler {
    pub fn new(filename: &str) -> Result<Self, String> {
        let mut compiler = ArtCompiler {
            memory: DiskMemory::new(filename, MemoryAccess::ReadWrite)?,
            nb_nodes: 0
        };

        compiler.add_node(Node256 {
            header: NodeHeader {
                frequency: None,
                kind: NodeKind::Node256,
                nb_children: 0,
                path_length: 0,
                path: [0; 7]
            },
            pointers: [None; 256]
        })?;

        Ok(compiler)
    }

    fn add_node(&mut self, node: Node256) -> Result<usize, String> {
        let index = self.nb_nodes * size_of::<Node256>();

        if self.memory.len() / size_of::<Node256>() == self.nb_nodes {
            // Need to double the vector size
            // So that not too much mmap are done per insertion
            // The node is duplicated, but it doesn't matter as they will be overwritten
            // by later use
            self.memory.push(node, if self.nb_nodes == 0 { 8 } else { self.nb_nodes })?;
        } else {
            // Enough memory, write the node
            unsafe {
                std::ptr::copy(
                    &node as *const Node256,
                    (self.memory.data_mut() as *mut Node256).offset(self.nb_nodes as isize),
                    1
                );
            }
        }

        self.nb_nodes += 1;
        Ok(index)
    }

    fn get_256(&self, node_index: usize) -> &Node256 {
        debug_assert!(node_index < self.nb_nodes * size_of::<Node256>());

        unsafe {
            &*(self.memory.data().offset(node_index as isize) as *const Node256)
        }
    }

    fn get_256_mut(&self, node_index: usize) -> &mut Node256 {
        debug_assert!(node_index < self.nb_nodes * size_of::<Node256>());

        unsafe {
            &mut *(self.memory.data().offset(node_index as isize) as *mut Node256)
        }
    }

    fn add_rec(&mut self, word: &[u8], frequency: WordFrequency, node_index: usize) {
        if !word.is_empty() {
            // In the middle of the word.
            if self.get_256(node_index).pointers[word[0] as usize].is_none() {
                // Node absent, add it and treat it as added.
                let child_index = self.add_node(Node256 {
                    header: NodeHeader {
                        frequency: None,
                        kind: NodeKind::Node256,
                        nb_children: 0,
                        path_length: 0,
                        path: [0; 7]
                    },
                    pointers: [None; 256]
                }).unwrap();

                self.get_256_mut(node_index).pointers[word[0] as usize] = NodeOffset::new(child_index);
            }

            self.add_rec(&word[1..], frequency, self.get_256(node_index).pointers[word[0] as usize].unwrap().get());
        } else {
            self.get_256_mut(node_index).header.frequency = Some(frequency);
        }
    }
}

impl Compiler for ArtCompiler {
    fn add(&mut self, word: &[u8], frequency: WordFrequency)
    {
        self.add_rec(word, frequency, 0);
    }

    fn build(self) {
        // TODO: compact path
        // TODO: compact the structure.
    }
}
