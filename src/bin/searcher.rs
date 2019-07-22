extern crate trie;

use core::str::from_utf8_unchecked;
use std::io::{self, BufRead, StdoutLock, Write};

use trie::distance::{
    DamerauLevenshteinBitDistance, DamerauLevenshteinDistance, IncrementalDistance,
};
use trie::limit::Limit;
use trie::{art::ArtSearch, Search, WordData};

fn write_word_data(stdout: &mut StdoutLock, data: &WordData) {
    let word = unsafe { from_utf8_unchecked(&data.word) };

    write!(
        stdout,
        "{{\"word\":\"{}\",\"freq\":{},\"distance\":{}}}",
        word, data.frequency, data.distance
    )
    .unwrap();
}

fn main() {
    // Add a limit of memory to test in real condition.
    Limit::Memory(512 * 1024 * 1024 /* 512 MB*/).apply();

    let searcher = ArtSearch::load(
        &std::env::args()
            .nth(1)
            .expect("Missing compiled file as argument"),
    )
    .expect("Could not load the compiled structure");

    let stdin = io::stdin();
    let mut stdin = stdin.lock();

    let stdout = io::stdout();
    let mut stdout = stdout.lock();

    let mut line = String::new();
    let mut leveinshtein = DamerauLevenshteinDistance::new(&[]);
    let mut leveinshtein_bit = DamerauLevenshteinBitDistance::new(&[]);

    let mut results = Vec::<WordData>::new();

    while let Ok(nb_read) = stdin.read_line(&mut line) {
        if nb_read == 0 {
            break; // End of file, nothing more to read.
        }

        let mut words = line.split_whitespace();
        let max_distance = str::parse(
            words
                .nth(1)
                .expect("Expected a second argument: the distance"),
        )
        .expect("The distance is not a number");

        let word = words
            .next()
            .expect("Expected the word to search as last argument");

        let word = word.as_bytes();

        // Takes the optmized version of levenshtein if it can.
        if leveinshtein_bit.allows(word, max_distance) {
            leveinshtein_bit.reset(word);
            searcher.search(&mut leveinshtein_bit, max_distance, &mut results);
        } else {
            leveinshtein.reset(word);
            searcher.search(&mut leveinshtein, max_distance, &mut results);
        };

        line.clear(); // To prevent reading the same line again and again
        write!(stdout, "[").unwrap();

        let mut results_iter = results.iter();

        if let Some(result) = results_iter.next() {
            write_word_data(&mut stdout, &result);
        }

        for result in results_iter {
            write!(stdout, ",").unwrap();
            write_word_data(&mut stdout, &result);
        }

        writeln!(stdout, "]").unwrap();
        stdout.flush().unwrap();
        results.clear();
    }
}
