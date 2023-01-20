use thiserror::Error;
use crate::formats::EbookError;

pub(crate) trait Readable {
    // Reader navigation using a string
    fn navigate_str(&self, path: &str) -> Result<usize, ReaderError>;
    // Reader navigation using an index
    fn navigate(&self, index: usize) -> Result<String, ReaderError>;
}

/// Possible errors for [Reader](Reader)
/// - **OutOfBounds**: When a given index exceeds the reader's bounds.
/// - **InvalidReference**: When the reader fails to retrieve content
/// due to lack of proper references. Usually caused by malformed files.
/// - **NoContent**: When retrieval of content to display fails.
#[derive(Error, Debug)]
pub enum ReaderError {
    #[error("[OutOfBounds Error][{cause}]: {description}")]
    OutOfBounds { cause: String, description: String },
    #[error("[InvalidReference Error][{cause}]: {description}")]
    InvalidReference { cause: String, description: String },
    #[error("[NoContent Error]{0}")]
    NoContent(EbookError)
}

/// Reader that allows traversal of an ebook file by file
///
/// # Examples
/// Opening and reading an epub file:
/// ```
/// use rbook::Ebook;
///
/// let epub = rbook::Epub::new("example.epub").unwrap();
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
/// assert_eq!(58, reader.current_index());
/// ```
/// Traversing and retrieving pages from a reader:
/// ```
/// # use rbook::Ebook;
/// # let epub = rbook::Epub::new("example.epub").unwrap();
/// let mut reader = epub.reader();
///
/// // Set reader position using an index or string
/// let content1 = reader.set_current_page(3).unwrap();
/// let content2 = reader.set_current_page_str("insert003.xhtml").unwrap();
///
/// assert_eq!(content1, content2);
///
/// // Get a page without updating the reader index
/// let content1 = reader.fetch_page(7).unwrap();
/// let content2 = reader.fetch_page_str("titlepage.xhtml").unwrap();
///
/// assert_eq!(3, reader.current_index());
/// assert_eq!(content1, content2);
/// ```
pub struct Reader<'a> {
    ebook: &'a dyn Readable,
    page_count: usize,
    current_index: usize,
}

impl<'a> Reader<'a> {
    // TODO: Potentially remove the page count argument here...
    pub(crate) fn new(ebook: &'a dyn Readable, page_count: usize) -> Self {
        Self {
            ebook,
            page_count,
            current_index: 0,
        }
    }

    pub fn current_index(&self) -> usize {
        self.current_index
    }

    pub fn page_count(&self) -> usize {
        self.page_count
    }

    /// Retrieve the page content of where the reader's
    /// current index is at
    ///
    /// # Errors
    /// Possible errors are described in [ReaderError](ReaderError).
    pub fn current_page(&self) -> Result<String, ReaderError> {
        self.fetch_page(self.current_index)
    }

    /// Retrieve the next page content. If retrieving the next page
    /// results in an error. The next page after it will be
    /// retrieved instead and so on.
    ///
    /// To view and handle errors, [try_previous_page()](Reader::try_next_page) can be used
    /// instead.
    pub fn next_page(&mut self) -> Option<String> {
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
    pub fn previous_page(&mut self) -> Option<String> {
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
    pub fn try_next_page(&mut self) -> Result<String, ReaderError> {
        self.set_current_page(self.current_index + 1)
    }

    /// Retrieve the previous page content. If an error is encountered,
    /// the index is not updated.
    ///
    /// # Errors
    /// Possible errors are described in [ReaderError](ReaderError).
    pub fn try_previous_page(&mut self) -> Result<String, ReaderError> {
        self.set_current_page(self.current_index - 1)
    }

    /// Retrieve the content of a page and update the
    /// reader's current index.
    ///
    /// # Errors
    /// Possible errors are described in [ReaderError](ReaderError).
    pub fn set_current_page(&mut self, page_index: usize) -> Result<String, ReaderError> {
        match self.fetch_page(page_index) {
            Ok(page_content) => {
                self.current_index = page_index;
                Ok(page_content)
            }
            Err(error) => Err(error)
        }
    }

    /// Retrieve the content of a page and update the
    /// reader's current index using a string value.
    ///
    /// # Errors
    /// Possible errors are described in [ReaderError](ReaderError).
    pub fn set_current_page_str(&mut self, path: &str) -> Result<String, ReaderError> {
        match self.ebook.navigate_str(path) {
            Ok(index) => self.set_current_page(index),
            Err(error) => Err(error)
        }
    }

    /// Retrieve the content of a page without updating the
    /// reader's current index.
    ///
    /// # Errors
    /// Possible errors are described in [ReaderError](ReaderError).
    pub fn fetch_page(&self, page_index: usize) -> Result<String, ReaderError> {
        self.ebook.navigate(page_index)
    }

    /// Retrieve the content of a page without updating the
    /// reader's current index using a string value.
    ///
    /// # Errors
    /// Possible errors are described in [ReaderError](ReaderError).
    pub fn fetch_page_str(&self, path: &str) -> Result<String, ReaderError> {
        match self.ebook.navigate_str(path) {
            Ok(index) => self.fetch_page(index),
            Err(error) => Err(error)
        }
    }
}