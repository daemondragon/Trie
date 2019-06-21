//! An implementation of the ART search structure
//! based on the original paper.
//! (https://db.in.tum.de/~leis/papers/ART.pdf)

use crate::WordFrequency;

use core::num::NonZeroUsize;

/// All differents nodes that exist
/// Note that they don't contains the node
/// itself as each node have a different size,
/// and placing them all here will make them have
/// the same size as the largest one, defeating
/// the purpose of having multiple nodes (saving space).
#[derive(PartialEq, Eq)]
enum NodeKind {
    Node4,
    Node16,
    Node48,
    Node256
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
    path: [u8; 7]
}

/// An offset to a node children.
/// Can't be zero as the root node is always the first one
/// and this node can't have a parent.
/// This allows to easily have an Option<NodeOffset>
/// while still using the same space.
pub type NodeOffset = NonZeroUsize;

/// A node that can only contains up to 4 chldren
/// This is the smallest node possible.
/// Packed is used so that it remove the trailing space for alignment.
/// It does allows to have a better space usage
#[repr(C, packed)]
struct Node4 {
    /// The header that contains general information about the node.
    /// In this case, header.kind is always NodeKind::Node4
    header: NodeHeader,
    /// The key to the next node (the represented character)
    /// For the key[i], the pointed child is located at pointers[i]
    keys: [u8; 4],
    /// An offset where the node's children are located
    pointers: [Option<NodeOffset>; 4]
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
    pointers: [Option<NodeOffset>; 16]
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
    pointers: [Option<NodeOffset>; 48]
}

/// A node that can only contains up to 16 chldren
#[repr(C)]
struct Node256 {
    /// The header that contains general information about the node.
    /// In this case, header.kind is always NodeKind::Node256
    header: NodeHeader,
    /// An offset where the node's children are located
    pointers: [Option<NodeOffset>; 256]
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

        assert_eq!(size_of::<Node4>(), 52);
        assert_eq!(size_of::<Node16>(), 160);
        assert_eq!(size_of::<Node48>(), 656);
        assert_eq!(size_of::<Node256>(), 2064);
    }
}