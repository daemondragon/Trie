use crate::{Compiler, WordFrequency};
use super::{NodeKind, NodeHeader, NodeOffset, Node0, Node4, Node16, Node48, Node256};

use std::fs::{File, OpenOptions};
use std::io::{Write, Seek, SeekFrom};

use core::num::NonZeroUsize;
use core::mem::size_of;

/// The compiler compile the ART structure into a stored
/// equivalent that can be directly used.
/// This compiler need to have each element added added in order
/// so that only a part on the structure need to be kept in memory
/// while the other part can be directly added to the file
/// and not modified after.
///
/// Each nodes are stored in memory as a Node256 for simplicity,
/// and continuously (each parent of a node is the previous one in the vector)
/// Nodes have their path directly compressed in memory.
///
/// Adding a new word are done like this:
/// - The compiler looks where the node will need to be inserted.
/// - Split and Path merge are done, the new nodes are not yet inserted.
/// - Nodes that where on the old trie path are inserted in the file (last first),
///     and nodes are rewrote so that correct file index are placed for each node
///     Nodes inserted are compressed (Node0, Node4...) at this moment.
/// - The new nodes are added in the in RAM vector and new word can be added next.
///
/// For pointer optimisation reason, the first node in the file is always
/// the root node (even if it will be inserted last), as 0 index means that
/// the node doesn't have a child.
pub struct ArtCompiler {
    /// Where the nodes need to be written.
    file: File,
    /// The nodes kept in memory, represent the full trie path
    /// that is not yet inserted into the file.
    nodes: Vec<RAMNode>,
    /// The current index in the file so that each parent node know
    /// the index of its newly inserted child.
    file_index: usize,
}

/// Node kept in RAM.
/// Each children not yet wrote into the file is not
/// inserted into the node children index as the file
/// index is not known at this point in time.
struct RAMNode {
    /// The Node256 used for that.
    /// Only this kind of node is used to kept the insertion
    /// simpler and prevent mistakes. The node is rewritten to the correct
    /// version when it is inserted into the file.
    node: Node256,
    /// If it as a child, on which node it is.
    /// As the in RAM node only represent one trie path,
    /// it means that each node can only have one child at most,
    /// as having two children means that the first one needs
    /// to be wrote on the file.
    /// If this node is the root one, this value will be ignored.
    child: u8
}

impl ArtCompiler {
    pub fn new(filename: &str) -> Result<Self, String> {

        let mut file = OpenOptions::new()
                        .read(true)
                        .write(true)
                        .create(true)
                        .truncate(true)
                        .open(filename)
                        .map_err(|error| format!("Can't open file \"{}\" (reason: {})", filename, error))?;

        // Reserve the root node.
        let buffer = unsafe {
            let ptr = &core::mem::zeroed::<Node256>() as *const Node256 as *const u8;
            std::slice::from_raw_parts(ptr, size_of::<Node256>())
        };

        file
            .write_all(buffer)
            .map_err(|error| format!("Can't write to file: {}", error))?;

        Ok(ArtCompiler {
            file: file,
            nodes: vec![
                RAMNode {
                    node: Node256 {
                        header: NodeHeader {
                            frequency: None,
                            kind: NodeKind::Node256,
                            nb_children: 0,
                            path_length: 0,
                            path: [0; 7]
                        },
                        pointers: [None; 256]
                    },
                    child: 0
                }
            ],
            file_index: size_of::<Node256>()
        })
    }
}

impl Compiler for ArtCompiler {
    fn add(&mut self, word: &[u8], frequency: WordFrequency) {
        self.add_rec(word, frequency, 0).unwrap();
    }

    fn build(mut self) {
        self.move_to_file(0).unwrap();
    }
}

impl ArtCompiler {
    /// Add the given word recursively.
    /// If a new trie path is created, the new one is written to file first.
    fn add_rec(&mut self, word: &[u8], frequency: WordFrequency, node_index: usize) -> Result<(), String> {
        if !word.is_empty() {
            // In the middle of the word.
            if self.nodes[node_index].node.pointers[word[0] as usize].is_none() {
                // Creating a new path, inserting the old path first
                if node_index + 1 < self.nodes.len() {
                    self.move_to_file(node_index + 1)?;
                }
                debug_assert!(node_index + 1 == self.nodes.len());

                // Increase the children count
                self.nodes[node_index].node.header.nb_children += 1;

                // Telling the father that it's new child is at this character.
                let parent_index = self.nodes.len() - 1;
                self.nodes[parent_index].child = word[0];

                // Node absent, add it and treat it as added.
                self.nodes.push(RAMNode {
                    node: Node256 {
                        header: NodeHeader {
                            frequency: None,
                            kind: NodeKind::Node256,
                            nb_children: 0,
                            path_length: 0,
                            path: [0; 7]
                        },
                        pointers: [None; 256]
                    },

                    child: 0
                });

                self.nodes[node_index].node.pointers[word[0] as usize] = NodeOffset::new(parent_index + 1);
            }

            self.add_rec(&word[1..], frequency, self.nodes[node_index].node.pointers[word[0] as usize].unwrap().get())
        } else {
            self.nodes[node_index].node.header.frequency = Some(frequency);
            Ok(())
        }
    }

    /// Moves all nodes from the given start index to the end of self.nodes
    /// to the file after doing some optimisation on it.
    fn move_to_file(&mut self, start_index: usize) -> Result<(), String> {
        self.path_compression(start_index);
        return self.write_to_file(start_index)
    }

    fn path_compression(&mut self, start_index: usize) {
        let mut index = start_index;

        while index + 1 < self.nodes.len() {
            let header = &self.nodes[index].node.header;

            if header.nb_children != 1// Too much children
            || (header.path_length as usize) >= header.path.len()// Path compression full
            || header.frequency.is_some() {// Have data

                // Try to compress the next nodes
                index += 1;
                continue;
            }

            let ram_node = self.nodes.remove(index);
            // At this point, index point on the child node, not the previous one.
            let mut header = &mut self.nodes[index].node.header;

            // Add the value to the compressed path (the child node have not yet being compressed)
            header.path = ram_node.node.header.path;
            header.path[ram_node.node.header.path_length as usize] = ram_node.child;
            header.path_length = ram_node.node.header.path_length + 1;

            // Don't increase the index as the nodes list have been reduced by one.
        }
    }

    /// Write to file all nodes from the end of self.nodes to the given starting index
    /// They are transformed to the correct version before being inserted
    /// so that the least amount of memory possible is used.
    fn write_to_file(&mut self, start_index: usize) -> Result<(), String> {
        // Inserting children first to have correct inserted index
        while start_index < self.nodes.len() {

            let is_root = self.nodes.len() <= 1;

            // Modifying the parent to add the newly node that will be inserted.
            if !is_root {
                let parent_index = self.nodes.len() - 2;
                let parent = &mut self.nodes[parent_index];
                parent.node.pointers[parent.child as usize] = NonZeroUsize::new(self.file_index);
            } else {
                // The node have no parent, it's the root node, inserting it at the start of the file.
                self.file
                    .seek(SeekFrom::Start(0))
                    .map_err(|error| format!("Can't go to the end of the file: {}", error))?;
            }

            // To what kind of node the newly node need to be transformed to ?
            let node = &self.nodes.last().unwrap().node;
            let new_type = if is_root {
                // Don't compact the root node as the place is taken anyways.
                NodeKind::Node256
            } else {
                match node.header.nb_children {
                    0       => NodeKind::Node0,
                    1...4   => NodeKind::Node4,
                    5...16  => NodeKind::Node16,
                    17...48 => NodeKind::Node48,
                    _       => NodeKind::Node256,
                }
            };

            // Advancing the file_index
            self.file_index += match new_type {
                NodeKind::Node0 => size_of::<Node0>(),
                NodeKind::Node4 => size_of::<Node4>(),
                NodeKind::Node16 => size_of::<Node16>(),
                NodeKind::Node48 => size_of::<Node48>(),
                NodeKind::Node256 => size_of::<Node256>()
            };

            fn write_node_to_file<T: Sized>(file: &mut File, node: T) -> Result<(), String> {
                let buffer = unsafe {
                    let ptr = &node as *const T as *const u8;
                    std::slice::from_raw_parts(ptr, size_of::<T>())
                };

                file
                    .write_all(buffer)
                    .map_err(|error| format!("Can't write to file: {}", error))?;

                Ok(())
            }

            // Transforming the node and inserting it in the file.
            let node = self.nodes.pop().unwrap().node;
            match new_type {
                NodeKind::Node0 => write_node_to_file::<Node0>(&mut self.file, node.into())?,
                NodeKind::Node4 => write_node_to_file::<Node4>(&mut self.file, node.into())?,
                NodeKind::Node16 => write_node_to_file::<Node16>(&mut self.file, node.into())?,
                NodeKind::Node48 => write_node_to_file::<Node48>(&mut self.file, node.into())?,
                NodeKind::Node256 => write_node_to_file::<Node256>(&mut self.file, node.into())?,
            }
        }

        Ok(())
    }
}