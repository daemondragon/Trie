extern crate trie;

use std::num::NonZeroU32;
use std::time::Instant;
use core::cmp::max;

use trie::{Compiler, Search, Information, trie::{MiniCompiler, MiniSearch}};
use trie::distance::{IncrementalDistance, DamerauLevenshteinDistance};
use trie::dictionary::{Dictionary, DictionaryLine};

fn basic_test() {
    let mut compiler = MiniCompiler::new("test.txt");
    for word in ["test", "a", "b", "other"].iter() {
        compiler.add(word.as_bytes(), NonZeroU32::new(1).unwrap());
    }

    compiler.build();

    let trie = MiniSearch::load("test.txt").unwrap();
    let mut levenshtein = DamerauLevenshteinDistance::new(&[]);

    for distance in [0, 1, 2, 3, 4].iter() {
        for word in ["test", "a", "b", "other"].iter() {
            println!("Searching {}, distance {}", word, distance);

            levenshtein.reset(word.as_bytes());

            for word_data in trie.search(&mut levenshtein, *distance) {
                println!("word: {}, frequency: {}, distance: {}",
                    String::from_utf8_lossy(&word_data.word),
                    word_data.frequency,
                    word_data.distance
                );
            }

            println!("");
        }
    }
}

fn bench() {
    for amount in [1_000, 10_000, 100_000].iter() {
        let trie_filename = format!("words_{}.bin", amount);
        println!("Testing trie \"{}\"", trie_filename);
        let trie = MiniSearch::load(&trie_filename).unwrap();

        // Starting by the good query first as they are more representative
        // of the real performance of the algorithm.
        for good_query_ratio in [100/*, 90, 75, 50, 25, 10, 0*/].iter() {
            let query_filename = format!("../split/query_{}_{}_{}.txt", amount, good_query_ratio, 100 - good_query_ratio);
            println!("Testing query file \"{}\"", query_filename);

            // Loading all the query once to prevent loading this impacting the result.
            let lines: Vec<DictionaryLine> = Dictionary::new(&query_filename).unwrap().into_iter().collect();
            let mut levenshtein = DamerauLevenshteinDistance::new(&[]);

            for distance in [0, 1, 2].iter() {
                let mut times: Vec<u128> = (0..10).map(|_| {
                    let start = Instant::now();

                    for line in lines.iter() {
                        levenshtein.reset(line.word.as_bytes());
                        let count = trie.search(&mut levenshtein, *distance).count();

                        assert!((*good_query_ratio != 100 || *distance != 0) || count == 1, "Expected to have found a word");
                    }

                    start.elapsed().as_millis()
                }).collect();

                times.sort();
                // Removing outlier.
                let times = &times[2..times.len() - 2];

                let median = times[times.len() / 2];
                let min_time = times[0];
                let max_time = times[times.len() - 1];

                println!("distance: {}, time: {} ms (+- {} ms) => {} query/sec",
                    distance,
                    median,
                    max(median - min_time, max_time - median),
                    lines.len() as u128 * 1000 / max(1, median)
                );
            }
        }
    }
}

fn main() {
    if let Some(arg) = std::env::args().nth(1) {
        let trie = MiniSearch::load("words_1000.bin").unwrap();

        match &*arg {
            "graph" => trie.graph(),
            "info" => {
                println!("words: {}", trie.words());
                println!("nodes: {}", trie.nodes());
                println!("height: {}", trie.height());
                println!("max_lenght: {}", trie.max_lenght());
            }
            "bench" => bench(),

            _ => { println!("Hello world!") },
        }
    } else {
        basic_test()
    }
}