use crate::{Compiler, WordFrequency};
use crate::memory::{DiskMemory, MemoryAccess};

use super::{NodeKind, NodeHeader, Node0, Node4, Node16, Node48, Node256};

use core::mem::size_of;

/// The compiler compile the ART structure into a stored
/// equivalent that can be directly used.
/// For simplicity, the compiler work in two phase:
/// -   The first phase, where word are added, when a node need
///     to be added, the Node256 is used instead
///     (all other nodes are never used)
/// -   The second phase is the compresion phase, when no more node
///     will be added. At this moment, node are compressed using the correct
///     equivalent and correctly rewritten so that the structure takes less
///     memory space.
pub struct ArtCompiler {
    /// The disk memory that is been used to save all the nodes
    memory: DiskMemory,
    /// How many nodes are really in memory
    /// (nodes are added in bulk to prevent mapping the file too many times.
    nb_nodes: usize,
}

impl ArtCompiler {
    fn new(filename: &str) -> Result<Self, String> {
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

    fn get(&self, node_index: usize) -> &Node256 {
        debug_assert!(node_index < self.nb_nodes);

        unsafe {
            &*(self.memory.data() as *mut Node256).offset(node_index as isize)
        }
    }

    fn get_mut(&self, node_index: usize) -> &mut Node256 {
        debug_assert!(node_index < self.nb_nodes);

        unsafe {
            &mut *(self.memory.data() as *mut Node256).offset(node_index as isize)
        }
    }

    fn add_rec(&mut self, word: &[u8], frequency: WordFrequency, node_index: usize) {
        if !word.is_empty() {
            // In the middle of the word.
            /*
            if self.nodes[node_index].children[word[0] as usize].is_none() {
                // Node absent, add it and treat it as added.
                self.nodes.push(MiniNode {
                    frequency: None,
                    children: [None; 256]
                }).unwrap();

                self.nodes[node_index].children[word[0] as usize] = NonZeroUsize::new(self.nodes.len() - 1);
            }

            self.add_rec(self.nodes[node_index].children[word[0] as usize].unwrap().get(), &word[1..], frequency);
            */
        } else {
            self.get_mut(node_index).header.frequency = Some(frequency);
        }
    }
}

impl Compiler for ArtCompiler {
    fn add(&mut self, word: &[u8], frequency: WordFrequency)
    {
        self.add_rec(word, frequency, 0);
    }

    fn build(self) {
        // TODO: compact the structure.
    }
}
