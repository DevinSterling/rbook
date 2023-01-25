pub mod content;

use crate::formats::EbookError;
use crate::reader::content::Content;
use thiserror::Error;

/// Result type with [ReaderError](ReaderError) as the error.
pub type ReaderResult<T> = Result<T, ReaderError>;

pub(crate) trait Readable {
    // Reader navigation using a string
    fn navigate_str(&self, path: &str) -> ReaderResult<usize>;
    // Reader navigation using an index
    fn navigate(&self, index: usize) -> ReaderResult<Content>;
}

/// Possible errors for [Reader](Reader)
/// - **[OutOfBounds](Self::OutOfBounds)**
/// - **[InvalidReference](Self::InvalidReference)**
/// - **[NoContent](Self::NoContent)**
#[derive(Error, Debug)]
pub enum ReaderError {
    /// When a given index exceeds the reader's bounds.
    #[error("[OutOfBounds Error][{cause}]: {description}")]
    OutOfBounds { cause: String, description: String },
    /// When the reader fails to retrieve content due to lack of
    /// proper references. Usually caused by malformed files.
    #[error("[InvalidReference Error][{cause}]: {description}")]
    InvalidReference { cause: String, description: String },
    /// When retrieval of content to display fails.
    #[error("[NoContent Error]{0}")]
    NoContent(EbookError),
}

/// Reader that allows traversal of an ebook file by file
///
/// # Examples
/// Opening and reading an epub file:
/// ```
/// use rbook::Ebook;
///
/// let epub = rbook::Epub::new("tests/ebooks/moby-dick.epub").unwrap();
///
/// // Creating a reader instance
/// let mut reader = epub.reader();
///
/// assert_eq!(0, reader.current_index());
///
/// // Printing the contents of each page
/// while let Some(content) = reader.next_page() {
///     println!("{content}")
/// }
///
/// assert_eq!(143, reader.current_index());
/// ```
/// Traversing and retrieving pages from a reader:
/// ```
/// # use rbook::Ebook;
/// # let epub = rbook::Epub::new("tests/ebooks/moby-dick.epub").unwrap();
/// let mut reader = epub.reader();
///
/// // Set reader position using an index or string
/// let content1 = reader.set_current_page(56).unwrap();
/// let content2 = reader.set_current_page_str("chapter_051.xhtml").unwrap();
///
/// assert_eq!(content1, content2);
///
/// // Get a page without updating the reader index
/// let content1 = reader.fetch_page(1).unwrap();
/// let content2 = reader.fetch_page_str("titlepage.xhtml").unwrap();
///
/// assert_eq!(56, reader.current_index());
/// assert_eq!(content1, content2);
/// ```
pub struct Reader<'a> {
    ebook: &'a dyn Readable,
    page_count: usize,
    current_index: usize,
}

impl<'a: 'b, 'b> Reader<'a> {
    // TODO: Potentially remove the page count argument here...
    pub(crate) fn new(ebook: &'a dyn Readable, page_count: usize) -> Self {
        Self {
            ebook,
            page_count,
            current_index: 0,
        }
    }

    /// Retrieve the current index of the reader.
    pub fn current_index(&self) -> usize {
        self.current_index
    }

    /// Retrieve the count of pages that can be traversed.
    ///
    /// The maximum value of the reader index is `page_count - 1`,
    /// similar to an array.
    pub fn page_count(&self) -> usize {
        self.page_count
    }

    /// Retrieve the page content of where the reader's
    /// current index is at
    ///
    /// # Errors
    /// Possible errors are described in [ReaderError](ReaderError).
    pub fn current_page(&self) -> ReaderResult<Content> {
        self.fetch_page(self.current_index)
    }

    /// Retrieve the next page content. If retrieving the next page
    /// results in an error. The next page after it will be
    /// retrieved instead and so on.
    ///
    /// To view and handle errors, [try_previous_page()](Reader::try_next_page) can be used
    /// instead.
    pub fn next_page(&mut self) -> Option<Content> {
        while self.current_index < self.page_count - 1 {
            match self.try_next_page() {
                Ok(page_content) => return Some(page_content),
                _ => self.current_index += 1,
            }
        }

        None
    }

    /// Retrieve the previous page content. If retrieving the previous page
    /// results in an error. The previous page before it will be
    /// retrieved instead and so on.
    ///
    /// To view and handle errors, [try_previous_page()](Reader::try_previous_page) can be used
    /// instead.
    pub fn previous_page(&mut self) -> Option<Content> {
        while self.current_index > 0 {
            match self.try_previous_page() {
                Ok(page_content) => return Some(page_content),
                _ => self.current_index -= 1,
            }
        }

        None
    }

    /// Retrieve the next page content. If an error is encountered,
    /// the index is not updated.
    ///
    /// # Errors
    /// Possible errors are described in [ReaderError](ReaderError).
    pub fn try_next_page(&mut self) -> ReaderResult<Content<'b>> {
        self.set_current_page(self.current_index + 1)
    }

    /// Retrieve the previous page content. If an error is encountered,
    /// the index is not updated.
    ///
    /// # Errors
    /// Possible errors are described in [ReaderError](ReaderError).
    pub fn try_previous_page(&mut self) -> ReaderResult<Content<'b>> {
        self.set_current_page(self.current_index - 1)
    }

    /// Retrieve the content of a page and update the
    /// reader's current index.
    ///
    /// # Errors
    /// Possible errors are described in [ReaderError](ReaderError).
    pub fn set_current_page(&mut self, page_index: usize) -> ReaderResult<Content<'b>> {
        match self.fetch_page(page_index) {
            Ok(page_content) => {
                self.current_index = page_index;
                Ok(page_content)
            }
            Err(error) => Err(error),
        }
    }

    /// Retrieve the content of a page and update the
    /// reader's current index using a string value.
    ///
    /// # Errors
    /// Possible errors are described in [ReaderError](ReaderError).
    pub fn set_current_page_str(&mut self, path: &str) -> ReaderResult<Content> {
        match self.ebook.navigate_str(path) {
            Ok(index) => self.set_current_page(index),
            Err(error) => Err(error),
        }
    }

    /// Retrieve the content of a page without updating the
    /// reader's current index.
    ///
    /// # Errors
    /// Possible errors are described in [ReaderError](ReaderError).
    pub fn fetch_page(&self, page_index: usize) -> ReaderResult<Content<'b>> {
        self.ebook.navigate(page_index)
    }

    /// Retrieve the content of a page without updating the
    /// reader's current index using a string value.
    ///
    /// # Errors
    /// Possible errors are described in [ReaderError](ReaderError).
    pub fn fetch_page_str(&self, path: &str) -> ReaderResult<Content<'b>> {
        match self.ebook.navigate_str(path) {
            Ok(index) => self.fetch_page(index),
            Err(error) => Err(error),
        }
    }
}
