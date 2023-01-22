use crate::errors::EbookError;

/// Retrieve simple statistical information, such as the character
/// or word count of an ebook.
///
/// # Examples
/// Counting from an epub:
/// ```
/// use rbook::{Ebook, Stats};
///
/// let epub = rbook::Epub::new("tests/ebooks/moby-dick.epub").unwrap();
/// let file_content = epub.read_bytes_file("chapter_022.xhtml").unwrap();
/// let word_count = epub.count_words(&file_content).unwrap();
///
/// assert_eq!(1683, word_count);
/// ```
/// Counting total characters:
/// ```
/// use rbook::{Ebook, Stats};
///
/// let epub = rbook::Epub::new("tests/ebooks/childrens-literature.epub").unwrap();
/// let char_count = epub.try_count_total_chars().unwrap();
///
/// assert_eq!(329289, char_count);
/// ```
pub trait Stats {
    /// Iterate through all resource elements and perform a function.
    ///
    /// Resource elements that fail to be retrieved will be skipped and
    /// the next one will be retrieved and so on.
    ///
    /// To view and handle errors, [try_count_total(...)](Self::try_count_total) can be
    /// used instead.
    fn count_total<F: Fn(&[u8]) -> Result<usize, EbookError>>(&self, f: F) -> usize;

    /// Iterate through all resource elements and perform a function.
    fn try_count_total<F: Fn(&[u8]) -> Result<usize, EbookError>>(&self, f: F) -> Result<usize, EbookError>;

    /// Calculate the count of all characters from a given collection
    /// of bytes.
    fn count_chars(&self, data: &[u8]) -> Result<usize, EbookError>;

    /// Calculate the count of all characters from a given collection
    /// of bytes.
    fn count_words(&self, data: &[u8]) -> Result<usize, EbookError>;

    /// Calculate the count of all characters in the ebook file.
    ///
    /// If retrieving a page fails, the next will be retrieved
    /// instead and so on.
    ///
    /// To view and handle errors,
    /// [try_count_total_chars(...)](Self::try_count_total_chars)
    /// can be used instead.
    fn count_total_chars(&self) -> usize {
        self.count_total(|data| self.count_chars(data))
    }

    /// Calculate the count of all characters in the ebook file and
    /// handle errors if any.
    ///
    /// To ignore errors, [count_total_chars()](Self::count_total_chars)
    /// can be used instead.
    fn try_count_total_chars(&self) -> Result<usize, EbookError> {
        self.try_count_total(|data| self.count_chars(data))
    }

    /// Calculate the count of all words in the ebook file. Any
    /// errors are skipped
    ///
    /// If retrieving a page fails, the next will be retrieved
    /// instead and so on.
    ///
    /// To view and handle errors,
    /// [try_count_total_words(...)](Self::try_count_total_words)
    /// can be used instead.
    fn count_total_words(&self) -> usize {
        self.count_total(|data| self.count_words(data))
    }

    /// Calculate the count of all words in the ebook file and
    /// handle errors if any.
    ///
    /// To ignore errors, [count_total_words()](Self::count_total_words)
    /// can be used instead.
    fn try_count_total_words(&self) -> Result<usize, EbookError> {
        self.try_count_total(|data| self.count_words(data))
    }
}