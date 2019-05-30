extern crate trie;

use trie::{Compiler, mini::MiniCompiler};
use trie::dictionary::Dictionary;

fn main() {
    let dictionary = Dictionary::new(
        &std::env::args().nth(1).expect("Missing dictionary filename as first argument")
    ).expect("Could not load dictionary file");

    let mut compiler = MiniCompiler::new(
        &std::env::args().nth(2).expect("Missing compiled file as second argument")
    );

    for line in dictionary {
        compiler.add(line.word.as_bytes(), line.frequency);
    }
}