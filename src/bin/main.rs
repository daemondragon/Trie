extern crate trie;

use std::num::NonZeroU32;

use trie::{Compiler, Search, mini::{MiniCompiler, MiniSearch}};

fn basic_test() {
    {
        let mut compiler = MiniCompiler::new("test.txt");
        for word in ["test", "a", "b", "other"].iter() {
            compiler.add(word.as_bytes(), NonZeroU32::new(1).unwrap());
        }
    }

    let trie = MiniSearch::load("test.txt").unwrap();
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

fn main() {
    if let Some(arg) = std::env::args().nth(1) {
        match &*arg {
            "graph" => {
                let trie = MiniSearch::load("test.txt").unwrap();
                println!("{}", trie);
            },
            _ => { println!("Hello world!") },
        }
    } else {
        basic_test()
    }
}