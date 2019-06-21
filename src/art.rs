//! An implementation of the ART search structure
//! based on the original paper.
//! (https://db.in.tum.de/~leis/papers/ART.pdf)

use crate::WordFrequency;

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
    /// How many bytes are used in the path
    path_length: u8,
    /// The compressed path.
    /// In addition to the bytes stored on link as a trie,
    /// a path having only one node per level can be compressed
    /// to only take one node instead of multiple one.
    /// It is pretty useless near the root of the trie,
    /// but is really handy further down the trie.
    path: [u8; 8]
}

mod tests {
    use super::*;
    use std::mem::size_of;

    #[test]
    fn size() {
        // Check the size of all structures
        // to be sure that what is expected
        assert_eq!(size_of::<NodeKind>(), 1);
        assert_eq!(size_of::<NodeHeader>(), 16);
    }
}