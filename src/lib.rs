pub mod bloom;
pub mod distance;
pub mod trie;
pub mod dictionary;

mod memory;

use core::num::NonZeroU32;

/// For the subject, each word's data is it's frequency.
/// Note that the frequency of the word is not representative
/// of the search done for the grade, so the frequency is
/// just a data that can't be used to optimize further
/// the search structure.
/// The frequency is NonZero as a zero frequency means
/// that the word doens't exist.
pub type WordFrequency = NonZeroU32;

/// The basic structure that need to be used for each search structure.
/// Each struture must be capable of storing the associated
/// data with it so that it can be retrieve without any problem
pub struct WordData {
    /// A slice of the word.
    /// Doesn't directly store the word
    /// as some structure might have a specific
    /// word saving format thaty allows to compact words.
    pub word: Vec<u8>,
    /// The associated data with the given word.
    pub data: WordFrequency,
    /// The distance from the word with the wanted
    pub distance: usize
}

/// Compile the search structure to the disk.
/// Note that the compiler must not take more
/// than 512M of RAM to compile it's search structure.
///
/// The compiler is known to have the file fully saved
/// when the Drop trait is called.
pub trait Compiler {
    /// Add the word data to the search structure.
    fn add(&mut self, word: &[u8], data: WordFrequency);

    /// Completely finish the compiled structure,
    /// no words can be added later, so various
    /// optimisation can be done in this structure.
    fn build(self);
}

/// Perform all search on the compiled version
/// of the Search structure.
/// The structure must be compiled directly one the
/// disk as the words count can be enormous and
/// the structure can't be stored in RAM.
///
/// Note that the RAM usage can't be more than 512M.
pub trait Search<'a> {
    /// Search for all the words under some given distance
    /// of the wanted word and return an iterator on all found words.
    ///
    /// This function must be capable of doing:
    /// - 3000 queries/seconds with a 0 distance.
    /// -  300 queries/seconds with a 1 distance.
    /// -   30 queries/seconds with a 2 distance.
    fn search(&'a self, word: &'a [u8], distance: usize) -> Box<dyn Iterator<Item=WordData> + 'a>;
}

/// Get information about a search structure
/// for easy visualisation and usefull information.
pub trait Information<'a> : Search<'a> {
    /// Get the number of words present in the structure
    fn words(&self) -> usize;

    /// Get the number of nodes used to represent all the words.
    fn nodes(&self) -> usize;

    /// Get the height of the search structure,
    /// as most structure as implemented as tree.
    fn height(&self) -> usize;

    /// Get the length of the longest word present
    /// in the search structure
    fn max_lenght(&self) -> usize;

    /// Display the search structure in the graphviz format
    /// so that it can be easily viewed by a user.
    ///
    /// The display must be done on the standart output.
    fn graph(&self);
}

pub mod limit {
    extern {
        #[cfg_attr(all(target_os = "macos", target_arch = "x86"), link_name = "setrlimit$UNIX2003")]
        fn setrlimit(resource: i32, rlim: *const rlimit) -> i32;
    }

    #[repr(C)]
    struct rlimit {
        rlim_cur: u64,
        rlim_max: u64,
    }

    /// All limits that can be placed on the process.
    /// A limit work affect the process even after going out of scope,
    /// and can't be reverted (other than applying another bigger limit).
    pub enum Limit {
        /// How many RAM memory in bytes the process can use.
        /// The quantity of virtual memory is not affected by this.
        Memory(u64),
        /// How much CPU times in seconds the process can use.
        CPUTime(u64),
    }

    impl Limit {
        /// Apply the limit and return if it could be applied or not.
        pub fn apply(self) -> bool {
            let (max, kind) = match self {
                Limit::Memory(value)  => (value, 2/* RLIMIT_DATA */),
                Limit::CPUTime(value) => (value, 0/* RLIMIT_CPU */),
            };

            let limit = rlimit { rlim_cur: max, rlim_max: max };
            unsafe { setrlimit(kind, &limit) == 0 }
        }
    }
}