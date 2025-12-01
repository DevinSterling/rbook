//! Sequential + random‐access [`Ebook`](super::Ebook) [`Reader`] module.
//!
//! # See Also
//! - [`epub::reader`][crate::epub::reader] for the epub-specific reader module.

pub mod errors;

use crate::ebook::manifest::ManifestEntry;
use crate::ebook::spine::SpineEntry;
use crate::reader::errors::ReaderResult;

/// A sequential + random-access [`Ebook`](super::reader) reader.
///
/// # Lifetime
/// All returned [`ReaderContent<'ebook>`](ReaderContent) are tied to the lifetime of the
/// underlying [`Ebook`](super::Ebook).
///
/// # See Also
/// - [`EpubReader`](crate::epub::reader::EpubReader) for epub-specific reader information.
///
/// # Examples
/// - Streaming over a reader's contents:
/// ```
/// # use rbook::{Ebook, Epub};
/// # use rbook::reader::{Reader, ReaderContent};
/// # use std::error::Error;
/// # fn main() -> Result<(), Box<dyn Error>> {
/// let epub = Epub::open("tests/ebooks/example_epub")?;
/// let mut reader = epub.reader();
/// # let mut count = 0;
///
/// // Stream over all content
/// while let Some(Ok(content)) = reader.read_next() {
/// #    count += 1;
///     // process content //
/// }
/// # assert_eq!(5, count);
/// # assert_eq!(count, reader.len());
/// # Ok(())
/// # }
/// ```
/// - Random access:
/// ```
/// # use rbook::{Ebook, Epub};
/// # use rbook::reader::{Reader, ReaderContent};
/// # use std::error::Error;
/// # fn main() -> Result<(), Box<dyn Error>> {
/// let epub = Epub::open("tests/ebooks/example_epub")?;
/// let mut reader = epub.reader();
///
/// let content_a = reader.get(2)?;
/// let content_b = reader.get("c1")?; // by idref
///
/// assert_eq!(2, content_a.position());
/// assert_eq!(2, content_b.position());
/// # assert_eq!(content_a.content(), content_b.content());
/// # Ok(())
/// # }
/// ```
pub trait Reader<'ebook> {
    /// Resets the reader's cursor to its initial state; **before** the first entry.
    ///
    /// After calling this method:
    /// - [`Self::current_position`] = [`None`]
    /// - [`Self::remaining`] = The total number of entries ([`Self::len`])
    ///
    /// By default, a newly created [`Reader`] starts in this state.
    ///
    /// # Examples
    /// - Assessing the current cursor position state:
    /// ```
    /// # use rbook::{Ebook, Epub};
    /// # use rbook::reader::{Reader, ReaderContent};
    /// # use std::error::Error;
    /// # fn main() -> Result<(), Box<dyn Error>> {
    /// let epub = Epub::open("tests/ebooks/example_epub")?;
    /// let mut reader = epub.reader();
    ///
    /// // Cursor is before the first entry
    /// assert_eq!(None, reader.current_position());
    /// assert_eq!(5, reader.remaining());
    ///
    /// // Iterate over all content
    /// for result in &mut reader {
    ///     // process content //
    /// }
    ///
    /// assert_eq!(0, reader.remaining());
    ///
    /// // Resetting the cursor, so it's **before** the first element
    /// reader.reset();
    /// assert_eq!(None, reader.current_position());
    /// assert_eq!(5, reader.remaining());
    ///
    /// // Setting the cursor **at** the first element.
    /// reader.read(0)?;
    /// assert_eq!(Some(0), reader.current_position());
    /// assert_eq!(4, reader.remaining());
    /// # Ok(())
    /// # }
    /// ```
    fn reset(&mut self);

    /// Returns the next [`ReaderContent`] and increments the reader's cursor by one.
    ///
    /// # Cases
    /// - `Some(Ok(content))`: Entry exists and reading it succeeded.  
    /// - `Some(Err(e))`: Entry exists yet reading it failed
    ///   (see [`ReaderError`](errors::ReaderError)).
    /// - `None`: No next entries; ***in this case, the cursor is not incremented.***
    ///
    /// # Examples
    /// - Observing how `read_next` affects the cursor position:
    /// ```
    /// # use rbook::{Ebook, Epub};
    /// # use rbook::reader::Reader;
    /// # use std::error::Error;
    /// # fn main() -> Result<(), Box<dyn Error>> {
    /// let epub = Epub::open("tests/ebooks/example_epub")?;
    /// let mut reader = epub.reader();
    ///
    /// // Current cursor position
    /// assert_eq!(None, reader.current_position());
    /// // Iterate to the end
    /// while let Some(Ok(content)) = reader.read_next() {
    ///     // process content //
    /// }
    /// // The current cursor position is now at the end
    /// assert_eq!(Some(4), reader.current_position());
    ///
    /// // No more next content
    /// assert!(reader.read_next().is_none());
    /// // The cursor is not updated
    /// assert_eq!(Some(4), reader.current_position());
    ///
    /// # Ok(())
    /// # }
    /// ```
    fn read_next(&mut self) -> Option<ReaderResult<impl ReaderContent<'ebook> + 'ebook>>;

    /// Returns the previous [`ReaderContent`] and decrements the reader's cursor by one.
    ///
    /// # Cases
    /// - `Some(Ok(content))`: Entry exists and reading it succeeded.  
    /// - `Some(Err(e))`: Entry exists yet reading it failed
    ///   (see [`ReaderError`](errors::ReaderError)).
    /// - `None`: No previous entries; ***in this case, the cursor is not decremented.***
    ///
    /// # Examples
    /// - Observing how `read_prev` affects the cursor position:
    /// ```
    /// # use rbook::{Ebook, Epub};
    /// # use rbook::reader::Reader;
    /// # use std::error::Error;
    /// # fn main() -> Result<(), Box<dyn Error>> {
    /// let epub = Epub::open("tests/ebooks/example_epub")?;
    /// let mut reader = epub.reader();
    ///
    /// // Jump to the end
    /// reader.seek(reader.len() - 1)?;
    /// assert_eq!(Some(4), reader.current_position());
    ///
    /// // Iterate to the start
    /// while let Some(Ok(content)) = reader.read_prev() {
    ///     // ... //
    /// }
    /// // Current cursor position at the start
    /// assert_eq!(Some(0), reader.current_position());
    ///
    /// // No more previous content
    /// assert!(reader.read_prev().is_none());
    /// // The cursor is not updated
    /// assert_eq!(Some(0), reader.current_position());
    ///
    /// # Ok(())
    /// # }
    /// ```
    fn read_prev(&mut self) -> Option<ReaderResult<impl ReaderContent<'ebook> + 'ebook>>;

    /// Returns the [`ReaderContent`] that the reader's cursor is currently positioned at.
    ///
    /// # Cases
    /// - `Some(Ok(content))`: Entry exists and reading it succeeded.  
    /// - `Some(Err(e))`: Entry exists yet reading it failed
    ///   (see [`ReaderError`](errors::ReaderError)).
    /// - `None`: No current entry ([`Self::current_position`] is [`None`]).
    fn read_current(&self) -> Option<ReaderResult<impl ReaderContent<'ebook> + 'ebook>>;

    /// Returns the [`ReaderContent`] at the given [`ReaderKey`]
    /// and moves the reader’s cursor to that position.
    ///
    /// Equivalent to [`Self::get`], except that this method updates the cursor.
    ///
    /// To re-iterate from the start, prefer [`Self::reset`]
    /// over `read(0)`, as `reset` puts the cursor **before** the first entry.
    ///
    /// # Errors
    /// See [`ReaderError`](errors::ReaderError).
    fn read<'a>(
        &mut self,
        key: impl Into<ReaderKey<'a>>,
    ) -> ReaderResult<impl ReaderContent<'ebook> + 'ebook>;

    /// Moves the reader’s cursor **to** the given [`ReaderKey`]
    /// and returns the resulting cursor position.
    ///
    /// To iterate from the start again, prefer [`Self::reset`]
    /// over `seek(0)`, as `reset` positions the cursor **before** the first entry.
    ///
    /// # Errors
    /// See [`ReaderError`](errors::ReaderError).
    fn seek<'a>(&mut self, key: impl Into<ReaderKey<'a>>) -> ReaderResult<usize>;

    /// Returns the [`ReaderContent`] at the given [`ReaderKey`]
    /// without moving the reader's cursor.
    ///
    /// # Errors
    /// See [`ReaderError`](errors::ReaderError).
    fn get<'a>(
        &self,
        key: impl Into<ReaderKey<'a>>,
    ) -> ReaderResult<impl ReaderContent<'ebook> + 'ebook>;

    /// The total number of traversable [`ReaderContent`] entries.
    ///
    /// This method returns the same value regardless of calls to methods that mutate
    /// a reader's cursor such as [`Self::read`].
    ///
    /// # See Also
    /// - [`Self::remaining`] to find out how many entries are left relative to a cursor.
    fn len(&self) -> usize;

    /// The position of the reader’s cursor (current entry).
    ///
    /// Returns [`None`] if the cursor is **before** the first entry
    /// (such as on a newly created reader or after invoking [`Self::reset`].
    /// Otherwise, `Some(i)` where `0 <= i < len`.
    ///
    /// # Examples
    /// - Retrieving the position upon navigating:
    /// ```
    /// # use rbook::{Ebook, Epub};
    /// # use rbook::reader::{Reader, ReaderContent};
    /// # use std::error::Error;
    /// # fn main() -> Result<(), Box<dyn Error>> {
    /// let epub = Epub::open("tests/ebooks/example_epub")?;
    /// let mut reader = epub.reader();
    ///
    /// assert_eq!(None, reader.current_position());
    ///
    /// // Set position to `0`
    /// reader.seek(0)?;
    /// assert_eq!(Some(0), reader.current_position());
    ///
    /// reader.read_next();
    /// assert_eq!(Some(1), reader.current_position());
    ///
    /// // Set position to the end
    /// reader.seek(reader.len() - 1)?;
    /// assert_eq!(Some(4), reader.current_position());
    ///
    /// // The position remains the same since the end is reached (len - 1)
    /// assert!(reader.read_next().is_none());
    /// assert_eq!(Some(4), reader.current_position());
    ///
    /// reader.reset();
    /// assert_eq!(None, reader.current_position());
    /// # Ok(())
    /// # }
    /// ```
    fn current_position(&self) -> Option<usize>;

    /// The total number of remaining traversable [`ReaderContent`]
    /// until the reader's cursor reaches the end.
    ///
    /// # Examples
    /// - Observing the number of contents remaining:
    /// ```
    /// # use rbook::{Ebook, Epub};
    /// # use rbook::reader::{Reader, ReaderContent};
    /// # use std::error::Error;
    /// # fn main() -> Result<(), Box<dyn Error>> {
    /// let epub = Epub::open("tests/ebooks/example_epub")?;
    /// let mut reader = epub.reader();
    ///
    /// assert_eq!(5, reader.len());
    /// assert_eq!(5, reader.remaining());
    /// assert_eq!(None, reader.current_position());
    ///
    /// // `len` remains fixed while `remaining` changes:
    /// reader.seek(3)?;
    /// assert_eq!(5, reader.len());
    /// assert_eq!(1, reader.remaining());
    /// assert_eq!(Some(3), reader.current_position());
    /// # Ok(())
    /// # }
    /// ```
    fn remaining(&self) -> usize {
        match self.current_position() {
            Some(position) => self.len().saturating_sub(position + 1),
            None => self.len(),
        }
    }

    /// Returns `true` if the reader has no [`ReaderContent`] to provide; a [`Reader::len`] of `0`.
    ///
    /// # Examples
    /// - Assessing if a reader has content:
    /// ```
    /// # use rbook::{Ebook, Epub};
    /// # use rbook::reader::{Reader, ReaderContent};
    /// # use std::error::Error;
    /// # fn main() -> Result<(), Box<dyn Error>> {
    /// let epub = Epub::open("tests/ebooks/example_epub")?;
    /// let mut reader = epub.reader();
    ///
    /// assert_eq!(5, reader.len());
    /// // The reader has 5 entries, so it is not empty:
    /// assert!(!reader.is_empty());
    /// # Ok(())
    /// # }
    /// ```
    fn is_empty(&self) -> bool {
        self.len() == 0
    }
}

/// Content provided by a [`Reader`], encompassing associated data.
///
/// # Examples
/// - Retrieving the content of the same entry by different [`keys`](ReaderKey):
/// ```
/// # use rbook::{Ebook, Epub};
/// # use rbook::reader::{Reader, ReaderContent};
/// # use rbook::ebook::manifest::ManifestEntry;
/// # use std::error::Error;
/// # fn main() -> Result<(), Box<dyn Error>> {
/// let epub = Epub::open("tests/ebooks/example_epub")?;
/// let mut reader = epub.reader();
///
/// let entry_by_idref = reader.get("cover")?;
/// let entry_by_position = reader.get(0)?;
/// let kind =  entry_by_idref.manifest_entry().resource_kind();
///
/// assert_eq!(0, entry_by_idref.position());
/// assert_eq!(0, entry_by_position.position());
/// assert_eq!("application/xhtml+xml", kind.as_str());
///
/// // Retrieving the main content
/// let string_ref: &str = entry_by_idref.content();
///
/// assert_eq!(string_ref, entry_by_position.content());
///
/// let string_content: String = entry_by_idref.into_string(); // or .into()
/// let bytes_content: Vec<u8> = entry_by_position.into(); // or .into_bytes()
///
/// assert_eq!(bytes_content, string_content.into_bytes());
/// # Ok(())
/// # }
/// ```
pub trait ReaderContent<'ebook>: PartialEq + Into<String> + Into<Vec<u8>> {
    /// The position of reader content within a [`Reader`] (0-index-based).
    ///
    /// This value may not equal [`SpineEntry::order`] depending
    /// on how a reader is configured.
    ///
    /// # Examples
    /// - Showcasing different positioning regarding EPUB:
    /// ```
    /// # use rbook::{Ebook, Epub};
    /// # use rbook::ebook::spine::SpineEntry;
    /// # use rbook::epub::reader::LinearBehavior;
    /// # use rbook::reader::{Reader, ReaderContent};
    /// # use std::error::Error;
    /// # fn main() -> Result<(), Box<dyn Error>> {
    /// let epub = Epub::open("tests/ebooks/example_epub")?;
    ///
    /// // Reader with non-linear spine entries prepended at the start of its internal buffer.
    /// let mut reader_a = epub.reader_builder()
    ///     .linear_behavior(LinearBehavior::PrependNonLinear)
    ///     .create();
    /// let content_a = reader_a.read_next().unwrap()?;
    ///
    /// assert_eq!(0, content_a.position());
    /// assert_eq!(0, content_a.spine_entry().order());
    /// assert_eq!("cover", content_a.spine_entry().idref());
    ///
    /// // Reader with non-linear spine entries appended at the end of its internal buffer.
    /// let mut reader_b = epub.reader_builder()
    ///     .linear_behavior(LinearBehavior::AppendNonLinear)
    ///     .create();
    /// let content_b = reader_b.read_next().unwrap()?;
    ///
    /// assert_eq!(0, content_b.position());
    /// assert_eq!(1, content_b.spine_entry().order());
    /// assert_eq!("toc", content_b.spine_entry().idref());
    /// # Ok(())
    /// # }
    /// ```
    fn position(&self) -> usize;

    /// The readable content (e.g., `XHTML`, `HTML`, etc.).
    fn content(&self) -> &str;

    /// The associated [`SpineEntry`] containing reading order details.
    fn spine_entry(&self) -> impl SpineEntry<'ebook> + 'ebook;

    /// The associated [`ManifestEntry`] containing resource details.
    fn manifest_entry(&self) -> impl ManifestEntry<'ebook> + 'ebook;

    /// Takes the contained readable content string.
    ///
    /// This method is equivalent to calling `into::<String>()`.
    ///
    /// See [`Self::content`] to retrieve a reference without taking ownership.
    ///
    /// # Examples:
    /// - Extracting the contained content in the form of a [`String`]:
    /// ```
    /// # use rbook::{Ebook, Epub};
    /// # use rbook::reader::{Reader, ReaderContent};
    /// # use std::error::Error;
    /// # fn main() -> Result<(), Box<dyn Error>> {
    /// let epub = Epub::open("tests/ebooks/example_epub")?;
    /// let mut reader = epub.reader();
    ///
    /// let content_a: String = reader.get(2)?.into();
    /// let content_b = reader.get(2)?.into_string();
    ///
    /// assert_eq!(content_a, content_b);
    /// # Ok(())
    /// # }
    /// ```
    fn into_string(self) -> String {
        self.into()
    }

    /// Takes the contained readable content bytes.
    ///
    /// This method is equivalent to calling `into::<Vec<u8>()`.
    ///
    /// # Examples:
    /// - Extracting the contained content in the form of bytes:
    /// ```
    /// # use rbook::{Ebook, Epub};
    /// # use rbook::reader::{Reader, ReaderContent};
    /// # use std::error::Error;
    /// # fn main() -> Result<(), Box<dyn Error>> {
    /// let epub = Epub::open("tests/ebooks/example_epub")?;
    /// let mut reader = epub.reader();
    ///
    /// let content_a: Vec<u8> = reader.get(2)?.into();
    /// let content_b = reader.get(2)?.into_bytes();
    ///
    /// assert_eq!(content_a, content_b);
    /// # Ok(())
    /// # }
    /// ```
    fn into_bytes(self) -> Vec<u8> {
        self.into()
    }
}

/// A key to access content within a [`Reader`].
#[non_exhaustive]
#[derive(Copy, Clone, Debug, Hash, Eq, PartialEq)]
pub enum ReaderKey<'a> {
    /// A string value, intended for lookup within a [`Reader`].
    ///
    /// For an [`Epub`](crate::Epub), this value corresponds to the `idref` of a spine entry.
    Value(&'a str),

    /// An absolute position within the internal buffer of a [`Reader`].
    ///
    /// When passed as an argument to a reader,
    /// it must be less than [`Reader::len`] or
    /// [`ReaderError::OutOfBounds`](errors::ReaderError::OutOfBounds) will be returned.
    Position(usize),
}

impl<'a> From<&'a str> for ReaderKey<'a> {
    fn from(value: &'a str) -> Self {
        Self::Value(value)
    }
}

impl From<usize> for ReaderKey<'_> {
    fn from(index: usize) -> Self {
        Self::Position(index)
    }
}
