//! Implementation of the trie used in the reference binary.
//! As the reference currectly outperform the current ART
//! implementation, the goal is to try to rewritte this one
//! to have better performance than the current implementation.
//!
//! The reference trie is made of entries, each one saved on a u32.
//! An entry is a link from a node (a prefix) to another one.
//! All link going out of a nodes are stored consecutively.
//!
//! An entries contains three different data:
//! - The character represented by the link.
//!   If zero, it's the entry contains the data associated with the current word.
//! - A boolean to know if the entries is the last one of the node.
//! - Either the child relative offset of the frequency associated with the word.
//!   (it depends on the value of the character).

use crate::memory::DiskMemory;

pub mod searcher;

pub use searcher::TrieSearch;

type Entry = u32;

fn get_flag(entry: Entry) -> bool {
    (entry & 0x80_00_00_00) != 0
}

fn get_char(entry: Entry) -> u8 {
    ((entry & 0x7F_00_00_00) >> 24) as u8
}

fn get_data(entry: Entry) -> u32 {
    //let data =
    //(*entry & 0x00_FF_00_00) >> 16 | (*entry & 0x00_00_FF_00) | (*entry & 0x00_00_00_FF) << 16
    entry & 0x00_FF_FF_FF
    //; eprintln!("{:x} {:x}",data, *entry); data
}

unsafe fn get(memory: &DiskMemory, offset: usize) -> &Entry {
    debug_assert!((offset + 1) * core::mem::size_of::<Entry>() <= memory.len());

    &*(memory.data() as *const Entry).offset(offset as isize)
}