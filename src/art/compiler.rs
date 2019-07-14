use crate::{Compiler, WordFrequency};
use crate::memory::{DiskMemory, MemoryAccess};
use crate::flags::Flags;

use super::{NodeKind, NodeHeader, NodeOffset, Node0, Node4, Node16, Node48, Node256, get, get_mut};

use core::mem::size_of;
use std::os::unix::io::AsRawFd;

extern {
    fn ftruncate(fd: i32, length: isize) -> i32;
}

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
}

/// All further optimisation that can be made on the ART
/// to increase it's speed.
impl ArtCompiler {
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
                self.path_compression(child_index.get(), deleted_nodes);
            }
        }
    }

    // Compact all nodes so that they can use the more efficient
    // memory layout adapted to their children count.
    // Note that aftee this operation, there is empty space between
    // nodes if they use smaller node version, they are not compacted together.
    //
    // Compacting all nodes allows for after to compact the file
    // so that more nodes can be placed in the same memory space,
    // allowing for another speed boost.
    fn node_compression(&mut self, index: usize, deleted_nodes: &Flags) {
        // The current node doesn't have been deleted
        debug_assert!(false == deleted_nodes.get(index / size_of::<Node256>()));

        // Perform the change for the children before as after the memory layout
        // will be changed. Doing this allows to not have one loop per node kind.
        for child_index in 0..self.get_256(index).pointers.len() {
            if let Some(child_index) = self.get_256(index).pointers[child_index].clone() {
                self.node_compression(child_index.get(), deleted_nodes);
            }
        }

        match self.get_256(index).header.nb_children {
            0 => {
                let mut node = self.get_256_mut(index);
                node.header.kind = NodeKind::Node0;
            },
            1...4 => {
                let mut node = self.get_256_mut(index);
                node.header.kind = NodeKind::Node4;

                // Change the layout
                let mut keys: [u8; 4] = [0; 4];
                let mut ptrs: [Option<NodeOffset>; 4] = [None; 4];
                let mut insert_index = 0;

                for (value, pointer) in node.pointers
                                            .iter()
                                            .enumerate()
                                            .filter(|(_, ptr)| ptr.is_some()) {

                    debug_assert!(insert_index < 4);

                    keys[insert_index] = value as u8;
                    ptrs[insert_index] = *pointer;
                    insert_index += 1;
                }

                let mut node = unsafe { get_mut::<Node4>(&mut self.memory, index) };
                node.keys = keys;
                node.pointers = ptrs;
            },
            5...16 => {
                let mut node = self.get_256_mut(index);
                node.header.kind = NodeKind::Node16;


                // Change the layout
                let mut keys: [u8; 16] = [0; 16];
                let mut ptrs: [Option<NodeOffset>; 16] = [None; 16];
                let mut insert_index = 0;

                for (value, pointer) in node.pointers
                                            .iter()
                                            .enumerate()
                                            .filter(|(_, ptr)| ptr.is_some()) {

                    debug_assert!(insert_index < 16);

                    keys[insert_index] = value as u8;
                    ptrs[insert_index] = *pointer;
                    insert_index += 1;
                }

                let mut node = unsafe { get_mut::<Node16>(&mut self.memory, index) };
                node.keys = keys;
                node.pointers = ptrs;
            },
            17...48 => {
                let mut node = self.get_256_mut(index);
                node.header.kind = NodeKind::Node48;

                                // Change the layout
                let mut keys: [u8; 256] = [core::u8::MAX; 256];
                let mut ptrs: [Option<NodeOffset>; 48] = [None; 48];
                let mut insert_index = 0;

                for (value, pointer) in node.pointers
                                            .iter()
                                            .enumerate()
                                            .filter(|(_, ptr)| ptr.is_some()) {

                    debug_assert!(insert_index < 48);

                    keys[value] = insert_index as u8;
                    ptrs[insert_index] = *pointer;
                    insert_index += 1;
                }

                let mut node = unsafe { get_mut::<Node48>(&mut self.memory, index) };
                node.keys = keys;
                node.pointers = ptrs;
            },
            _ => { /* Nothing to do, as it's the big version that need to be used */ }
        }
    }

    fn memory_compression_mapping(&self, deleted_nodes: &Flags) -> Vec<(usize, usize)> {
        let mut mapping: Vec<(usize, usize)> = (0..self.nb_nodes)
            .filter(|index| !deleted_nodes.get(*index))
            .map(|index| index * size_of::<Node256>())
            .map(|index| (index, index))
            .collect();

        assert!(mapping.len() > 0);
        assert_eq!(0, mapping[0].0);
        assert_eq!(0, mapping[0].1);

        let mut next_position = 0;
        mapping
            .iter_mut()
            .for_each(|it| {
                debug_assert!(next_position <= it.0);

                it.1 = next_position;
                let header = unsafe { get::<NodeHeader>(&self.memory, it.0) };

                next_position += match header.kind {
                    NodeKind::Node0 => size_of::<Node0>(),
                    NodeKind::Node4 => size_of::<Node4>(),
                    NodeKind::Node16 => size_of::<Node16>(),
                    NodeKind::Node48 => size_of::<Node48>(),
                    NodeKind::Node256 => size_of::<Node256>()
                };
            });

        return mapping;
    }

    fn rewritte_nodes(&mut self, mapping: &Vec<(usize, usize)>) {
        for index in mapping.iter().map(|(_, new)| new) {

            match (unsafe { get::<NodeHeader>(&self.memory, *index) }).kind {
                NodeKind::Node0 => { /* No more thing to do */ },
                NodeKind::Node4 => {
                    let mut node = unsafe { get_mut::<Node4>(&mut self.memory, *index) };

                    for ptr_index in 0..node.header.nb_children {
                        node.pointers[ptr_index as usize] = mapping
                            .iter()
                            .find(|(old, _)| *old == node.pointers[ptr_index as usize].unwrap().get())
                            .map(|(_, new)| NodeOffset::new(*new).unwrap());
                    }
                },
                NodeKind::Node16 => {
                    let mut node = unsafe { get_mut::<Node16>(&mut self.memory, *index) };

                    for ptr_index in 0..node.header.nb_children {
                        node.pointers[ptr_index as usize] = mapping
                            .iter()
                            .find(|(old, _)| *old == node.pointers[ptr_index as usize].unwrap().get())
                            .map(|(_, new)| NodeOffset::new(*new).unwrap());
                    }
                },
                NodeKind::Node48 => {
                    let mut node = unsafe { get_mut::<Node48>(&mut self.memory, *index) };

                    for ptr_index in 0..node.header.nb_children {
                        node.pointers[ptr_index as usize] = mapping
                            .iter()
                            .find(|(old, _)| *old == node.pointers[ptr_index as usize].unwrap().get())
                            .map(|(_, new)| NodeOffset::new(*new).unwrap());
                    }

                },
                NodeKind::Node256 => {
                    let mut node = unsafe { get_mut::<Node256>(&mut self.memory, *index) };

                    for ptr_index in 0..256 {
                        if node.pointers[ptr_index as usize].is_none() {
                            continue;
                        }

                        node.pointers[ptr_index as usize] = mapping
                            .iter()
                            .find(|(old, _)| *old == node.pointers[ptr_index as usize].unwrap().get())
                            .map(|(_, new)| NodeOffset::new(*new).unwrap());
                    }
                }
            }
        }

        // Truncate the file so that it is shorter
        // to remove useless node at the end.
        unsafe {
            let last_index = mapping.last().unwrap().1;
            let header = get::<NodeHeader>(&self.memory, last_index);

            let end = last_index + match header.kind {
                NodeKind::Node0 => size_of::<Node0>(),
                NodeKind::Node4 => size_of::<Node4>(),
                NodeKind::Node16 => size_of::<Node16>(),
                NodeKind::Node48 => size_of::<Node48>(),
                NodeKind::Node256 => size_of::<Node256>()
            };

            ftruncate(self.memory.file().as_raw_fd(), end as isize);
        }
    }

    /// Compress the trie in memory so that less space need to be fetched
    /// while searching for a word.
    fn memory_compression(&mut self, deleted_nodes: &Flags) {
        let mapping = self.memory_compression_mapping(deleted_nodes);

        // Moving all nodes.
        for (old_index, next_index) in mapping.iter() {
            debug_assert!(next_index <= old_index);

            unsafe {
                let size = match get::<NodeHeader>(&self.memory, *old_index).kind {
                    NodeKind::Node0 => size_of::<Node0>(),
                    NodeKind::Node4 => size_of::<Node4>(),
                    NodeKind::Node16 => size_of::<Node16>(),
                    NodeKind::Node48 => size_of::<Node48>(),
                    NodeKind::Node256 => size_of::<Node256>()
                };

                std::ptr::copy(
                    self.memory.data().offset(*old_index as isize) as *const u8,
                    self.memory.data().offset(*next_index as isize) as *mut u8,
                    size
                );
            }
        }

        // Rewritting nodes to match the new mapping
        self.rewritte_nodes(&mapping);
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
        self.node_compression(0, &deleted_nodes);
        self.memory_compression(&deleted_nodes);
    }
}
