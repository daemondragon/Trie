use crate::{Search, Information, WordFrequency, WordData};
use crate::memory::{DiskMemory, MemoryAccess};
use crate::distance::IncrementalDistance;

use super::{NodeKind, NodeHeader, NodeOffset, Node0, Node4, Node16, Node48, Node256, get, get_mut};

pub struct ArtSearch {
    /// The disk memory that is been used to save all the nodes
    memory: DiskMemory,
}

impl ArtSearch {
    pub fn load(filename: &str) -> Result<Self, String> {
        Ok(ArtSearch {
            memory: DiskMemory::open(filename, MemoryAccess::ReadOnly)?
        })
    }
}

impl Search for ArtSearch {
    fn search(&self, distance: &mut IncrementalDistance, max_distance: usize) -> Box<dyn Iterator<Item=WordData>> {
        Box::new(std::iter::empty())
    }
}

impl Information for ArtSearch {
    fn words(&self) -> usize {
        self.words_rec(0)
    }

    fn nodes(&self) -> usize {
        self.nodes_rec(0)
    }

    fn height(&self) -> usize {
        self.height_rec(0)
    }

    fn max_lenght(&self) -> usize {
        self.max_lenght_rec(0)
    }

    fn graph(&self) {
        println!("digraph G {{");

        self.graph_rec(0);

        println!("}}");
    }
}

impl ArtSearch {
    fn words_rec(&self, index: usize) -> usize {
        match (unsafe { get::<NodeHeader>(&self.memory, index) }).unwrap().kind {
            NodeKind::Node0 => {
                let node = unsafe { get::<Node0>(&self.memory, index) }.unwrap();

                if unsafe { &node.header.frequency }.is_some() { 1 } else { 0 }
            },
            NodeKind::Node4 => {
                let node = unsafe { get::<Node4>(&self.memory, index) }.unwrap();

                let count: usize = if unsafe { &node.header.frequency }.is_some() { 1 } else { 0 };
                let children_count: usize = (0..node.header.nb_children)
                        .map(|index| self.words_rec(node.pointers[index as usize].unwrap().get()))
                        .sum();

                count + children_count
            },
            NodeKind::Node16 => {
                let node = unsafe { get::<Node16>(&self.memory, index) }.unwrap();

                let count: usize = if unsafe { &node.header.frequency }.is_some() { 1 } else { 0 };
                let children_count: usize = (0..node.header.nb_children)
                        .map(|index| self.words_rec(node.pointers[index as usize].unwrap().get()))
                        .sum();

                count + children_count
            },
            NodeKind::Node48 => {
                let node = unsafe { get::<Node48>(&self.memory, index) }.unwrap();

                let count: usize = if unsafe { &node.header.frequency }.is_some() { 1 } else { 0 };
                let children_count: usize = node.keys
                        .iter()
                        .filter(|index| **index != core::u8::MAX)
                        .map(|index| self.words_rec(node.pointers[*index as usize].unwrap().get()))
                        .sum();

                count + children_count
            },
            NodeKind::Node256 => {
                let node = unsafe { get::<Node256>(&self.memory, index) }.unwrap();

                let count: usize = if unsafe { &node.header.frequency }.is_some() { 1 } else { 0 };
                let children_count: usize = node.pointers
                        .iter()
                        .filter(|index| index.is_some())
                        .map(|index| self.words_rec(index.unwrap().get()))
                        .sum();

                count + children_count
            }
        }
    }

    fn nodes_rec(&self, index: usize) -> usize {
        match (unsafe { get::<NodeHeader>(&self.memory, index) }).unwrap().kind {
            NodeKind::Node0 => { 1 },
            NodeKind::Node4 => {
                let node = unsafe { get::<Node4>(&self.memory, index) }.unwrap();

                let children_count: usize = (0..node.header.nb_children)
                        .map(|index| self.nodes_rec(node.pointers[index as usize].unwrap().get()))
                        .sum();

                1 + children_count
            },
            NodeKind::Node16 => {
                let node = unsafe { get::<Node16>(&self.memory, index) }.unwrap();

                let children_count: usize = (0..node.header.nb_children)
                        .map(|index| self.nodes_rec(node.pointers[index as usize].unwrap().get()))
                        .sum();

                1 + children_count
            },
            NodeKind::Node48 => {
                let node = unsafe { get::<Node48>(&self.memory, index) }.unwrap();

                let children_count: usize = node.keys
                        .iter()
                        .filter(|index| **index != core::u8::MAX)
                        .map(|index| self.nodes_rec(node.pointers[*index as usize].unwrap().get()))
                        .sum();

                1 + children_count
            },
            NodeKind::Node256 => {
                let node = unsafe { get::<Node256>(&self.memory, index) }.unwrap();

                let children_count: usize = node.pointers
                        .iter()
                        .filter(|index| index.is_some())
                        .map(|index| self.nodes_rec(index.unwrap().get()))
                        .sum();

                1 + children_count
            }
        }
    }

    fn height_rec(&self, index: usize) -> usize {
        match (unsafe { get::<NodeHeader>(&self.memory, index) }).unwrap().kind {
            NodeKind::Node0 => { 0 },
            NodeKind::Node4 => {
                let node = unsafe { get::<Node4>(&self.memory, index) }.unwrap();

                let children_count: usize = (0..node.header.nb_children)
                        .map(|index| self.height_rec(node.pointers[index as usize].unwrap().get()))
                        .max()
                        .unwrap_or(0);

                1 + children_count
            },
            NodeKind::Node16 => {
                let node = unsafe { get::<Node16>(&self.memory, index) }.unwrap();

                let children_count: usize = (0..node.header.nb_children)
                        .map(|index| self.height_rec(node.pointers[index as usize].unwrap().get()))
                        .max()
                        .unwrap_or(0);

                1 + children_count
            },
            NodeKind::Node48 => {
                let node = unsafe { get::<Node48>(&self.memory, index) }.unwrap();

                let children_count: usize = node.keys
                        .iter()
                        .filter(|index| **index != core::u8::MAX)
                        .map(|index| self.height_rec(node.pointers[*index as usize].unwrap().get()))
                        .max()
                        .unwrap_or(0);

                1 + children_count
            },
            NodeKind::Node256 => {
                let node = unsafe { get::<Node256>(&self.memory, index) }.unwrap();

                let children_count: usize = node.pointers
                        .iter()
                        .filter(|index| index.is_some())
                        .map(|index| self.height_rec(index.unwrap().get()))
                        .max()
                        .unwrap_or(0);

                1 + children_count
            }
        }
    }

    fn max_lenght_rec(&self, index: usize) -> usize {
        match (unsafe { get::<NodeHeader>(&self.memory, index) }).unwrap().kind {
            NodeKind::Node0 => {
                let node = unsafe { get::<Node0>(&self.memory, index) }.unwrap();

                1 + node.header.path_length as usize
            },
            NodeKind::Node4 => {
                let node = unsafe { get::<Node4>(&self.memory, index) }.unwrap();

                let children_count: usize = (0..node.header.nb_children)
                        .map(|index| self.max_lenght_rec(node.pointers[index as usize].unwrap().get()))
                        .max()
                        .unwrap_or(0);

                1 + node.header.path_length as usize + children_count
            },
            NodeKind::Node16 => {
                let node = unsafe { get::<Node16>(&self.memory, index) }.unwrap();

                let children_count: usize = (0..node.header.nb_children)
                        .map(|index| self.max_lenght_rec(node.pointers[index as usize].unwrap().get()))
                        .max()
                        .unwrap_or(0);

                1 + node.header.path_length as usize + children_count
            },
            NodeKind::Node48 => {
                let node = unsafe { get::<Node48>(&self.memory, index) }.unwrap();

                let children_count: usize = node.keys
                        .iter()
                        .filter(|index| **index != core::u8::MAX)
                        .map(|index| self.max_lenght_rec(node.pointers[*index as usize].unwrap().get()))
                        .max()
                        .unwrap_or(0);

                1 + node.header.path_length as usize + children_count
            },
            NodeKind::Node256 => {
                let node = unsafe { get::<Node256>(&self.memory, index) }.unwrap();

                let children_count: usize = node.pointers
                        .iter()
                        .filter(|index| index.is_some())
                        .map(|index| self.max_lenght_rec(index.unwrap().get()))
                        .max()
                        .unwrap_or(0);

                1 + node.header.path_length as usize + children_count
            }
        }
    }

    fn graph_rec_display_link(&self, index: usize, child_index: usize, value: char) {
        let child_header = unsafe { get::<NodeHeader>(&self.memory, child_index) }.unwrap();

        println!("{} -> {} [label=\"{}{}\"];",
            index, child_index, value,
            unsafe { std::str::from_utf8_unchecked(&child_header.path[0..(child_header.path_length as usize)]) }
        );
    }

    fn graph_rec(&self, index: usize) {
        let header = unsafe { get::<NodeHeader>(&self.memory, index) }.unwrap();

        print!("{} [", index);

        if let Some(frequency) = header.frequency {
            print!("label=\"{}\", color=green, style=filled", frequency.get());
        } else {
            print!("label=\"\"");
        }

        print!(", shape={}", match header.kind {
            NodeKind::Node0 => "circle",
            NodeKind::Node4 => "triangle",
            NodeKind::Node16 => "box",
            NodeKind::Node48 => "pentagon",
            NodeKind::Node256 => "hexagon",
        });

        println!("];");

        match (unsafe { get::<NodeHeader>(&self.memory, index) }).unwrap().kind {
            NodeKind::Node0 => { /* No more thing to do */ },
            NodeKind::Node4 => {
                let node = unsafe { get::<Node4>(&self.memory, index) }.unwrap();

                for (value, child_index) in (0..node.header.nb_children)
                                                .map(|index| (index as u8 as char, node.pointers[index as usize].unwrap())) {

                    self.graph_rec_display_link(index, child_index.get(), value);
                    self.graph_rec(child_index.get());
                }
            },
            NodeKind::Node16 => {
                let node = unsafe { get::<Node16>(&self.memory, index) }.unwrap();

                for (value, child_index) in (0..node.header.nb_children)
                                                .map(|index| (index as u8 as char, node.pointers[index as usize].unwrap())) {

                    self.graph_rec_display_link(index, child_index.get(), value);
                    self.graph_rec(child_index.get());
                }

            },
            NodeKind::Node48 => {
                let node = unsafe { get::<Node48>(&self.memory, index) }.unwrap();

                for (value, child_index) in node.keys
                                                .iter()
                                                .filter(|index| **index != core::u8::MAX)
                                                .map(|index| (*index as char, node.pointers[*index as usize].unwrap())) {

                    self.graph_rec_display_link(index, child_index.get(), value);
                    self.graph_rec(child_index.get());
                }
            },
            NodeKind::Node256 => {
                let node = unsafe { get::<Node256>(&self.memory, index) }.unwrap();

                for (value, child_index) in node.pointers
                                       .iter()
                                       .enumerate()
                                       .filter(|(_, index)| index.is_some())
                                       .map(|(value, index)| (value as u8 as char, index.unwrap())) {

                    self.graph_rec_display_link(index, child_index.get(), value);
                    self.graph_rec(child_index.get());
                }
            }
        }
    }
}