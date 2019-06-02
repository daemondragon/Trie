extern crate trie;

use std::num::NonZeroU32;
use std::time::Instant;
use core::cmp::max;

use trie::{Compiler, Search, Information, trie::{MiniCompiler, MiniSearch}};
use trie::distance::{IncrementalDistance, DamerauLevenshteinDistance};

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
                    word_data.data,
                    word_data.distance
                );
            }

            println!("");
        }
    }
}

fn bench(trie: &Search) {
    for test_filename in ["../words.txt"].iter() {
        println!("Testing {}", test_filename);
        let words_count = 1;//TODO: fix this

        let mut levenshtein = DamerauLevenshteinDistance::new(&[]);

        for distance in [0, 1, 2].iter() {
            let mut times: Vec<(u128, usize)> = (0..10).map(|_| {
                let start = Instant::now();

                // TODO: put in loop of all words.
                let word = "hello";
                levenshtein.reset(word.as_bytes());
                let amount = trie.search(&mut levenshtein, *distance).count();


                (start.elapsed().as_millis(), amount)
            }).collect();

            times.sort_by(|a, b| a.0.cmp(&b.0));
            // Removing outlier.
            let times = &times[2..times.len() - 2];

            let median = times[times.len() / 2];
            let median_time = median.0;
            let min_time = times[0].0;
            let max_time = times[times.len() - 1].0;

            assert!(times.iter().all(|x| x.1 == median.1), "Inconsistent result length");

            println!("distance: {}, time: {} ms (+- {} ms) => {} query/sec, result count: {}",
                distance,
                median_time,
                max(median_time - min_time, max_time - median_time),
                words_count * 1000 / max(1, median_time),
                median.1
            );
        }
    }
}

fn main() {
    if let Some(arg) = std::env::args().nth(1) {
        let trie = MiniSearch::load("small.bin").unwrap();

        match &*arg {
            "graph" => trie.graph(),
            "info" => {
                println!("words: {}", trie.words());
                println!("nodes: {}", trie.nodes());
                println!("height: {}", trie.height());
                println!("max_lenght: {}", trie.max_lenght());
            }
            "bench" => bench(&trie),

            _ => { println!("Hello world!") },
        }
    } else {
        basic_test()
    }
}