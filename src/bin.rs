extern crate trie;

use std::num::NonZeroU32;
use trie::{WordFrequency, WordData, Compiler, Search, mini::{MiniCompiler, MiniSearch}};

fn main() {
    println!("Hello world!");

    {
        let mut compiler = MiniCompiler::<WordFrequency>::new("test.txt");
        for word in ["test", "a", "b", "other"].iter() {
            compiler.add(word.as_bytes(), NonZeroU32::new(1).unwrap());
        }
    }

    let mut trie = MiniSearch::<WordFrequency>::load("test.txt").unwrap();
    for distance in [0, 1, 2, 3, 4].iter() {
        for word in ["test", "a", "b", "other"].iter() {
            println!("Searching {}, distance {}", word, distance);
            for word_data in trie.search(word.as_bytes(), *distance) {
                println!("word: {}, frequency: {}, distance: {}",
                    String::from_utf8_lossy(&word_data.word),
                    word_data.data,
                    word_data.distance
                );
            }

            println!("");
        }
    }
}