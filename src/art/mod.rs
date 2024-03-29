//! An implementation of the ART search structure
//! based on the original paper.
//! (https://db.in.tum.de/~leis/papers/ART.pdf)

use crate::memory::DiskMemory;
use crate::WordFrequency;

pub mod compiler;
pub mod searcher;

pub use compiler::ArtCompiler;
pub use searcher::ArtSearch;

use core::num::NonZeroUsize;

/// All differents nodes that exist
/// Note that they don't contains the node
/// itself as each node have a different size,
/// and placing them all here will make them have
/// the same size as the largest one, defeating
/// the purpose of having multiple nodes (saving space).
#[derive(PartialEq, Eq)]
enum NodeKind {
    Node0,
    Node4,
    Node16,
    Node48,
    Node256,
}

/// The header present before all nodes
/// as it is a shared information.
#[repr(C)]
struct NodeHeader {
    /// The frequency associated with the formed word
    /// if the node represent a word.
    frequency: Option<WordFrequency>,
    /// The type of the node that just follows this header
    kind: NodeKind,
    /// how many children are stored in the node
    nb_children: u8,
    /// How many bytes are used in the path
    path_length: u8,
    /// The compressed path.
    /// In addition to the bytes stored on link as a trie,
    /// a path having only one node per level can be compressed
    /// to only take one node instead of multiple one.
    /// It is pretty useless near the root of the trie,
    /// but is really handy further down the trie.
    ///
    /// Note that instead of using a pointer to a list of u8
    /// for storing the full compressed path, each node
    /// can only store up to 7 character (u8).
    /// This does allows to not have a pointer indirection
    /// and reduce cache miss
    path: [u8; 7],
}

/// An offset to a node children.
/// Can't be zero as the root node is always the first one
/// and this node can't have a parent.
/// This allows to easily have an Option<NodeOffset>
/// while still using the same space.
pub type NodeOffset = NonZeroUsize;

/// A leaf node containing zero children.
/// This node is usefull as for the provided dictionnary,
/// more than 90% of the words end up in a lead node
/// (very few words are prefix of other words)
/// This node does allows to save a large amount of storage.
#[repr(C)]
struct Node0 {
    /// The header that contains general information about the node.
    /// In this case, header.kind is always NodeKind::Node0
    header: NodeHeader,
}

/// A node that can only contains up to 4 chldren
/// Packed is used so that it remove the trailing space for alignment.
/// It does allows to have a better space usage
#[repr(C, packed)]
struct Node4 {
    /// The header that contains general information about the node.
    /// In this case, header.kind is always NodeKind::Node4
    header: NodeHeader,
    /// An offset where the node's children are located
    pointers: [Option<NodeOffset>; 4],
    /// The key to the next node (the represented character)
    /// For the key[i], the pointed child is located at pointers[i]
    keys: [u8; 4],
}

/// A node that can only contains up to 16 chlidren
#[repr(C)]
struct Node16 {
    /// The header that contains general information about the node.
    /// In this case, header.kind is always NodeKind::Node16
    header: NodeHeader,
    /// The key to the next node (the represented character)
    /// For the key[i], the pointed child is located at pointers[i]
    keys: [u8; 16],
    /// An offset where the node's children are located
    pointers: [Option<NodeOffset>; 16],
}

/// A node that can only contains up to 16 chldren
#[repr(C)]
struct Node48 {
    /// The header that contains general information about the node.
    /// In this case, header.kind is always NodeKind::Node48
    header: NodeHeader,
    /// Instead of being the key to the next node (the represented character)
    /// it represent the index where the associated pointer is located
    /// e.g: pointer[keys[key]] if keys[key] is a valid index (the key is present)
    /// If the key is not present, use core::u8::MAX instead.
    keys: [u8; 256],
    /// An offset where the node's children are located
    pointers: [Option<NodeOffset>; 48],
}

/// A node that can only contains up to 16 chldren
#[repr(C)]
struct Node256 {
    /// The header that contains general information about the node.
    /// In this case, header.kind is always NodeKind::Node256
    header: NodeHeader,
    /// An offset where the node's children are located
    pointers: [Option<NodeOffset>; 256],
}

unsafe fn get<T: Sized>(memory: &DiskMemory, offset: usize) -> &T {
    debug_assert!(offset + core::mem::size_of::<T>() <= memory.len());

    &*(memory.data().add(offset) as *const T)
}

/// Allows to easily transform a Node256 into a Node0
impl From<Node256> for Node0 {
    fn from(node: Node256) -> Self {
        debug_assert!(node.header.nb_children == 0, "Node must node have children");

        let mut new_node = Node0 {
            header: node.header,
        };
        new_node.header.kind = NodeKind::Node0;

        new_node
    }
}

/// Allows to easily transform a Node256 into a Node4
impl From<Node256> for Node4 {
    fn from(node: Node256) -> Self {
        debug_assert!(
            node.header.nb_children <= 4,
            "Node must node have more than 4 children"
        );

        // Change the layout
        let mut keys: [u8; 4] = [0; 4];
        let mut ptrs: [Option<NodeOffset>; 4] = [None; 4];
        let mut insert_index = 0;

        for (value, pointer) in node
            .pointers
            .iter()
            .enumerate()
            .filter(|(_, ptr)| ptr.is_some())
        {
            debug_assert!(insert_index < 4);

            keys[insert_index] = value as u8;
            ptrs[insert_index] = *pointer;
            insert_index += 1;
        }

        let mut new_node = Node4 {
            header: node.header,
            keys,
            pointers: ptrs,
        };

        new_node.header.kind = NodeKind::Node4;

        new_node
    }
}

/// Allows to easily transform a Node256 into a Node16
impl From<Node256> for Node16 {
    fn from(node: Node256) -> Self {
        debug_assert!(
            node.header.nb_children <= 16,
            "Node must node have more than 16 children"
        );

        // Change the layout
        let mut keys: [u8; 16] = [0; 16];
        let mut ptrs: [Option<NodeOffset>; 16] = [None; 16];
        let mut insert_index = 0;

        for (value, pointer) in node
            .pointers
            .iter()
            .enumerate()
            .filter(|(_, ptr)| ptr.is_some())
        {
            debug_assert!(insert_index < 16);

            keys[insert_index] = value as u8;
            ptrs[insert_index] = *pointer;
            insert_index += 1;
        }

        let mut new_node = Node16 {
            header: node.header,
            keys,
            pointers: ptrs,
        };

        new_node.header.kind = NodeKind::Node16;

        new_node
    }
}

/// Allows to easily transform a Node256 into a Node16
impl From<Node256> for Node48 {
    fn from(node: Node256) -> Self {
        debug_assert!(
            node.header.nb_children <= 48,
            "Node must node have more than 16 children"
        );

        // Change the layout
        let mut keys: [u8; 256] = [core::u8::MAX; 256];
        let mut ptrs: [Option<NodeOffset>; 48] = [None; 48];
        let mut insert_index = 0;

        for (value, pointer) in node
            .pointers
            .iter()
            .enumerate()
            .filter(|(_, ptr)| ptr.is_some())
        {
            debug_assert!(insert_index < 48);

            keys[value] = insert_index as u8;
            ptrs[insert_index] = *pointer;
            insert_index += 1;
        }

        let mut new_node = Node48 {
            header: node.header,
            keys,
            pointers: ptrs,
        };
        new_node.header.kind = NodeKind::Node48;

        new_node
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::mem::size_of;

    #[test]
    fn size() {
        // Check the size of all structures to be sure that what is expected
        // All sizes are taken from the original paper

        assert_eq!(size_of::<NodeKind>(), 1);
        assert_eq!(size_of::<NodeHeader>(), 16);

        assert_eq!(size_of::<Node0>(), 16);
        assert_eq!(size_of::<Node4>(), 52);
        assert_eq!(size_of::<Node16>(), 160);
        assert_eq!(size_of::<Node48>(), 656);
        assert_eq!(size_of::<Node256>(), 2064);
    }
}
