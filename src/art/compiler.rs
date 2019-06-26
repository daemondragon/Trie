use crate::{Compiler, WordFrequency};
use crate::memory::{DiskMemory, MemoryAccess};
use crate::flags::Flags;

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
            let node: &Node256 = &*(self.memory.data().offset(node_index as isize) as *const Node256);

            debug_assert!(node.header.kind == NodeKind::Node256);

            node
        }
    }

    fn get_256_mut(&self, node_index: usize) -> &mut Node256 {
        debug_assert!(node_index < self.nb_nodes * size_of::<Node256>());

        unsafe {
            let node: &mut Node256 = &mut *(self.memory.data().offset(node_index as isize) as *mut Node256);

            debug_assert!(node.header.kind == NodeKind::Node256);

            node
        }
    }

    fn add_rec(&mut self, word: &[u8], frequency: WordFrequency, node_index: usize) {
        if !word.is_empty() {
            // In the middle of the word.
            if self.get_256(node_index).pointers[word[0] as usize].is_none() {
                // Increase the children count
                self.get_256_mut(node_index).header.nb_children += 1;

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

    // Compact all path that can be compressed (single children nodes)
    // so that there is less node to iterate over when searching for words.
    //
    // Allows to have some nice speed-up as less memory need to be read for a word search.
    fn path_compression(&mut self, index: usize, deleted_nodes: &mut Flags) {

        loop {
            let mut node = self.get_256_mut(index);

            if node.header.nb_children != 1// Too much children
            || (node.header.path_length as usize) >= node.header.path.len()// Path compression full
            || node.header.frequency.is_some() {// Have data

                break;
            }

            // Try to compact the next node if possible
            // and mark the next one as deleted
            // Correct the pointers index.
            // Break out of the loop if can't compact.
            let (value, next_node_index) = node.pointers
                                                .iter()
                                                .enumerate()
                                                .find(|(_, index)| index.is_some())
                                                .map(|(value, index)| (value as u8, index.unwrap().get()))
                                                .unwrap();//Have at least one, there is one children.

            let next_node = self.get_256(next_node_index);

            // Add the value to the compresse path
            node.header.path[node.header.path_length as usize] = value;
            node.header.path_length += 1;

            // Copy the pointers keys to this one as they are compressed.
            debug_assert!(next_node.header.path_length == 0, "Next node already have a compacted path");

            node.pointers = next_node.pointers;
            node.header.nb_children = next_node.header.nb_children;
            node.header.frequency = next_node.header.frequency;

            // Mark the next node as deleted
            debug_assert!(next_node_index % size_of::<Node256>() == 0);
            deleted_nodes.set(next_node_index / size_of::<Node256>(), true);
        }

        for child_index in 0..self.get_256(index).pointers.len() {
            if let Some(child_index) = self.get_256(index).pointers[child_index].clone() {
                self.path_compaction(child_index.get(), deleted_nodes);
            }
        }
    }
}

impl Compiler for ArtCompiler {
    fn add(&mut self, word: &[u8], frequency: WordFrequency)
    {
        self.add_rec(word, frequency, 0);
    }

    fn build(mut self) {
        let mut deleted_nodes = Flags::new(self.memory.len() / size_of::<Node256>());

        self.path_compression(0, &mut deleted_nodes);
        // TODO: compact the structure.
        // TODO: compact the file.
    }
}
