pub mod content;

use std::fmt::Debug;
use thiserror::Error;

use crate::formats::EbookError;
use crate::reader::content::Content;

/// Result type with [ReaderError] as the error.
pub type ReaderResult<T> = Result<T, ReaderError>;

pub(crate) trait Readable: Debug {
    fn page_count(&self) -> usize;
    // Reader navigation using a string
    fn navigate_str(&self, path: &str) -> Option<ReaderResult<usize>>;
    // Reader navigation using an index
    fn navigate(&self, index: usize) -> Option<ReaderResult<Content>>;
}

/// Possible errors for [Reader]
/// - [InvalidReference](Self::InvalidReference)
/// - [NoContent](Self::NoContent)
#[derive(Error, Debug)]
pub enum ReaderError {
    /// When the reader fails to retrieve content due to lack of
    /// proper references. Usually caused by malformed files.
    #[error("[InvalidReference Error][{cause}]: {description}")]
    InvalidReference { cause: String, description: String },
    /// When retrieval of content fails, such as invalid utf-8.
    #[error("[NoContent Error]{0}")]
    NoContent(EbookError),
}

/// Reader that allows traversal of an ebook file by file.
///
/// The reader always starts at the first file of an ebook, which can be
/// accessed using [current_page()](Reader::current_page). As such,
/// calling [next_page()](Reader::next_page) is not required when a
/// [Reader] instance is created.
///
/// To iterate over all files at once, [iter()](Reader::iter) can do so.
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
/// // Reader starts at first page by default
/// assert_eq!(0, reader.current_index());
/// println!("{}", reader.current_page().unwrap());
///
/// // Printing the contents of the next pages
/// while let Some(Ok(content)) = reader.next_page() {
///     println!("{content}")
/// }
///
/// assert_eq!(143, reader.current_index());
/// ```
/// Iterators can also be used to read the contents of an
/// ebook. However, iterators will not update the internal
/// index of a [Reader] instance.
///
/// Using a for loop to read an epub:
/// ```
/// use rbook::Ebook;
///
/// let epub = rbook::Epub::new("tests/ebooks/moby-dick.epub").unwrap();
///
/// // Creating a reader instance
/// let mut reader = epub.reader();
/// let mut count = 0;
///
/// for (i, content) in reader.iter().enumerate() {
///     count = i;
///     println!("{}", content.unwrap());
/// }
///
/// assert_eq!(143, count);
/// // Iterators do not update the internal index of a `Reader`
/// assert_eq!(0, reader.current_index());
/// ```
/// Traversing and retrieving pages from a reader:
/// ```
/// # use rbook::Ebook;
/// # let epub = rbook::Epub::new("tests/ebooks/moby-dick.epub").unwrap();
/// let mut reader = epub.reader();
///
/// // Set reader position using an index or string
/// let content1 = reader
///     .set_current_page(56)
///     .expect("Index should be within bounds")
///     .expect("Associated content should be valid");
/// let content2 = reader
///     .set_current_page_str("chapter_051.xhtml")
///     .expect("Page should be within ebook")
///     .expect("Associated content should be valid");
///
/// assert_eq!(content1, content2);
///
/// // Get a page without updating the reader index
/// let content1 = reader.fetch_page(1).unwrap().unwrap();
/// let content2 = reader.fetch_page_str("titlepage.xhtml").unwrap().unwrap();
///
/// assert_eq!(56, reader.current_index());
/// assert_eq!(content1, content2);
/// ```
#[derive(Debug, Clone)]
pub struct Reader<'a> {
    ebook: &'a dyn Readable,
    current_index: usize,
}

impl<'a> Reader<'a> {
    pub(crate) fn new(ebook: &'a dyn Readable) -> Self {
        Self {
            ebook,
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
        self.ebook.page_count()
    }

    /// Retrieve an iterator to iterate over all the pages of
    /// an ebook.
    ///
    /// The retrieved iterator does not update the internal
    /// index of the [Reader] instance.
    pub fn iter(&self) -> ReaderIter {
        ReaderIter {
            reader: self,
            index: 0,
        }
    }

    /// Retrieve the page content of where the reader's
    /// current index is at
    ///
    /// # Errors
    /// Possible errors are described in [ReaderError].
    pub fn current_page(&self) -> ReaderResult<Content<'a>> {
        self.fetch_page(self.current_index)
            .expect("Should be within bounds")
    }

    /// Retrieve the next page content.
    ///
    /// # Errors
    /// Possible errors are described in [ReaderError].
    pub fn next_page(&mut self) -> Option<ReaderResult<Content<'a>>> {
        if self.current_index < self.page_count() - 1 {
            self.current_index += 1;
            self.set_current_page(self.current_index)
        } else {
            None
        }
    }

    /// Retrieve the previous page content.
    ///
    /// # Errors
    /// Possible errors are described in [ReaderError].
    pub fn previous_page(&mut self) -> Option<ReaderResult<Content<'a>>> {
        if self.current_index > 0 {
            self.current_index -= 1;
            self.set_current_page(self.current_index)
        } else {
            None
        }
    }

    /// Retrieve the content of a page and update the
    /// reader's current index.
    ///
    /// # Errors
    /// Possible errors are described in [ReaderError].
    pub fn set_current_page(&mut self, page_index: usize) -> Option<ReaderResult<Content<'a>>> {
        if page_index < self.page_count() {
            self.current_index = page_index;
            self.fetch_page(page_index)
        } else {
            None
        }
    }

    /// Retrieve the content of a page and update the
    /// reader's current index using a string value.
    ///
    /// # Errors
    /// Possible errors are described in [ReaderError].
    pub fn set_current_page_str(&mut self, path: &str) -> Option<ReaderResult<Content<'a>>> {
        match self.ebook.navigate_str(path) {
            Some(Ok(index)) => self.set_current_page(index),
            Some(Err(error)) => Some(Err(error)),
            _ => None,
        }
    }

    /// Retrieve the content of a page without updating the
    /// reader's current index.
    ///
    /// # Errors
    /// Possible errors are described in [ReaderError].
    pub fn fetch_page(&self, page_index: usize) -> Option<ReaderResult<Content<'a>>> {
        self.ebook.navigate(page_index)
    }

    /// Retrieve the content of a page without updating the
    /// reader's current index using a string value.
    ///
    /// # Errors
    /// Possible errors are described in [ReaderError].
    pub fn fetch_page_str(&self, path: &str) -> Option<ReaderResult<Content<'a>>> {
        match self.ebook.navigate_str(path) {
            Some(Ok(index)) => self.fetch_page(index),
            Some(Err(error)) => Some(Err(error)),
            _ => None,
        }
    }
}

impl<'a> IntoIterator for &'a Reader<'_> {
    type Item = ReaderResult<Content<'a>>;
    type IntoIter = ReaderIter<'a>;

    fn into_iter(self) -> Self::IntoIter {
        ReaderIter {
            reader: self,
            index: 0,
        }
    }
}

/// Iterator for a [Reader] instance.
pub struct ReaderIter<'a> {
    reader: &'a Reader<'a>,
    index: usize,
}

impl<'a> Iterator for ReaderIter<'a> {
    type Item = ReaderResult<Content<'a>>;

    fn next(&mut self) -> Option<Self::Item> {
        if self.index >= self.reader.page_count() {
            self.index -= 1;
            None
        } else {
            let current = self.reader.fetch_page(self.index);
            self.index += 1;
            current
        }
    }
}
