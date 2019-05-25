/// The basic structure that need to be used for each search structure.
/// Each struture must be capable of storing the associated
/// data with it so that it can be retrieve without any problem
pub struct WordData<'a, T> {
    /// A slice of the word.
    /// Doesn't directly store the word
    /// as some structure might have a specific
    /// word saving format thaty allows to compact words.
    pub word: &'a[u8],
    /// The associated data with the given word.
    pub data: T
}

/// For the subject, each word's data is it's frequency.
/// Note that the frequency of the word is not representative
/// of the search done for the grade, so the frequency is
/// just a data that can't be used to optimize further
/// the search structure.
pub type WordFrequency<'a> = WordData<'a, u32>;

/// Compile the search structure to the disk.
/// Note that the compiler must not take more
/// than 512M of RAM to compile it's search structure.
pub trait Compiler<T> {
    /// Add the word data to the search structure.
    fn add<'a>(word_data: &WordData<'a, T>);
}

/// Perform all search on the compiled version
/// of the Search structure.
/// The structure must be compiled directly one the
/// disk as the words count can be enormous and
/// the structure can't be stored in RAM.
///
/// Note that the RAM usage can't be more than 512M.
pub trait Search<T> {
    /// Search for all the words under some given distance
    /// of the wanted word and return an iterator on all found words.
    ///
    /// This function must be capable of doing:
    /// - 3000 queries/seconds with a 0 distance.
    /// -  300 queries/seconds with a 1 distance.
    /// -   30 queries/seconds with a 2 distance.
    fn search<'a>(&self, word: &[u8], distance: usize) -> Box<Iterator<Item=&WordData<'a, T>>>;
}