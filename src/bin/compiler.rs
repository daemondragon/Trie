extern crate trie;

use trie::{Compiler, art::ArtCompiler};
use trie::dictionary::{Dictionary, DictionaryLine};
use trie::limit::Limit;

fn main() {
    // Add a limit of memory to test in real condition.
    Limit::Memory(512 * 1024 * 1024/* 512 MB*/).apply();

    let dictionary = Dictionary::new(
        &std::env::args().nth(1).expect("Missing dictionary filename as first argument")
    ).expect("Could not load dictionary file");

    let mut compiler = ArtCompiler::new(
        &std::env::args().nth(2).expect("Missing compiled file as second argument")
    ).unwrap();

    // Sort lines so that the compiler can be smarter.
    let mut lines: Vec<DictionaryLine> = dictionary.into_iter().collect();
    lines.sort();

    for line in lines {
        compiler.add(line.word.as_bytes(), line.frequency);
    }

    compiler.build();
}