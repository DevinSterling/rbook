//! [`Reader`]-specific implementations for the [`Epub`] format.

use crate::ebook::epub::Epub;
use crate::ebook::epub::errors::EpubError;
use crate::ebook::epub::manifest::EpubManifestEntry;
use crate::ebook::epub::spine::EpubSpineEntry;
use crate::reader::errors::{ReaderError, ReaderResult};
use crate::reader::{Reader, ReaderContent, ReaderKey};
use crate::util::iter::IndexCursor;
use crate::util::{self, Sealed};
use std::cmp::PartialEq;

/// A [`Reader`] for an [`Epub`].
///
/// # Configuration
/// Reading behavior, such as how to handle non-linear content,
/// can be configured using [`Epub::reader_builder`] or [`EpubReaderOptions`].
///
/// # Examples
/// - Retrieving a new EPUB reader instance with configuration:
/// ```
/// # use rbook::Epub;
/// # use rbook::epub::reader::LinearBehavior;
/// # fn main() -> rbook::ebook::errors::EbookResult<()> {
/// let epub = Epub::open("tests/ebooks/example_epub")?;
/// let mut reader = epub.reader_builder()
///     .linear_behavior(LinearBehavior::LinearOnly) // Omit non-linear readable entries
///     .create();
/// # let mut count = 0;
///
/// // Stream over all linear content
/// for content_result in &mut reader {
///     # count += 1;
///     let content = content_result?;
///     assert!(content.spine_entry().is_linear());
/// }
/// # assert_eq!(3, count);
/// # assert_eq!(count, reader.len());
/// # Ok(())
/// # }
/// ```
#[derive(Clone, Debug, PartialEq)]
pub struct EpubReader<'ebook> {
    entries: Vec<EpubSpineEntry<'ebook>>,
    cursor: IndexCursor,
}

impl<'ebook> EpubReader<'ebook> {
    pub(super) fn new(epub: &'ebook Epub, config: EpubReaderConfig) -> Self {
        let entries = Self::get_entries(epub, config.linear_behavior);

        EpubReader {
            cursor: IndexCursor::new(entries.len()),
            entries,
        }
    }

    // If and when EpubReader grows, this method will be extracted to a submodule.
    fn get_entries(epub: &'ebook Epub, behavior: LinearBehavior) -> Vec<EpubSpineEntry<'ebook>> {
        let iterator = epub.spine().iter();
        match behavior {
            LinearBehavior::Original => iterator.collect(),
            LinearBehavior::LinearOnly | LinearBehavior::NonLinearOnly => {
                let predicate = behavior == LinearBehavior::LinearOnly;
                iterator
                    .filter(|entry| entry.is_linear() == predicate)
                    .collect()
            }
            LinearBehavior::PrependNonLinear | LinearBehavior::AppendNonLinear => {
                let (mut linear, mut non_linear) =
                    iterator.partition::<Vec<_>, _>(EpubSpineEntry::is_linear);

                if matches!(&behavior, LinearBehavior::AppendNonLinear) {
                    linear.extend(non_linear);
                    linear
                } else {
                    non_linear.extend(linear);
                    non_linear
                }
            }
        }
    }

    fn get_manifest_entry(
        spine_entry: EpubSpineEntry<'ebook>,
    ) -> ReaderResult<EpubManifestEntry<'ebook>> {
        spine_entry.manifest_entry().ok_or_else(|| {
            ReaderError::Format(EpubError::InvalidIdref(spine_entry.idref().to_owned()).into())
        })
    }

    fn find_entry_by_idref(&self, idref: &str) -> ReaderResult<usize> {
        self.entries
            .iter()
            .position(|entry| entry.idref() == idref)
            .ok_or_else(|| ReaderError::NoMapping(idref.to_string()))
    }

    fn find_entry_by_position(&self, position: usize) -> ReaderResult<EpubReaderContent<'ebook>> {
        let spine_entry = self.entries[position];
        let manifest_entry = Self::get_manifest_entry(spine_entry)?;

        Self::create_reader_content(position, spine_entry, manifest_entry)
    }

    fn find_entry_by_str(&self, idref: &str) -> ReaderResult<(usize, EpubReaderContent<'ebook>)> {
        let position = self.find_entry_by_idref(idref)?;
        let spine_entry = self.entries[position];
        let manifest_entry = Self::get_manifest_entry(spine_entry)?;

        Ok((
            position,
            Self::create_reader_content(position, spine_entry, manifest_entry)?,
        ))
    }

    fn create_reader_content(
        position: usize,
        spine_entry: EpubSpineEntry<'ebook>,
        manifest_entry: EpubManifestEntry<'ebook>,
    ) -> ReaderResult<EpubReaderContent<'ebook>> {
        Ok(EpubReaderContent {
            content: manifest_entry.read_str()?,
            position,
            spine_entry,
            manifest_entry,
        })
    }

    /// Resets the reader's cursor to its initial state; before the first entry.
    #[doc = util::inherent_doc!(Reader, reset)]
    pub fn reset(&mut self) {
        self.cursor.reset();
    }

    /// Returns the next [`EpubReaderContent`] and increments the reader's cursor by one.
    #[doc = util::inherent_doc!(Reader, read_next)]
    pub fn read_next(&mut self) -> Option<ReaderResult<EpubReaderContent<'ebook>>> {
        self.cursor
            .increment()
            .map(|index| self.find_entry_by_position(index))
    }

    /// Returns the previous [`EpubReaderContent`] and decrements the reader's cursor by one.
    #[doc = util::inherent_doc!(Reader, read_prev)]
    pub fn read_prev(&mut self) -> Option<ReaderResult<EpubReaderContent<'ebook>>> {
        self.cursor
            .decrement()
            .map(|index| self.find_entry_by_position(index))
    }

    /// Returns the [`EpubReaderContent`] that the reader's cursor is currently positioned at.
    #[doc = util::inherent_doc!(Reader, read_current)]
    pub fn read_current(&self) -> Option<ReaderResult<EpubReaderContent<'ebook>>> {
        self.current_position()
            .map(|position| self.find_entry_by_position(position))
    }

    /// Returns the [`EpubReaderContent`] at the given [`ReaderKey`]
    /// and moves the reader’s cursor to that position.
    #[doc = util::inherent_doc!(Reader, read)]
    pub fn read<'a>(
        &mut self,
        key: impl Into<ReaderKey<'a>>,
    ) -> ReaderResult<EpubReaderContent<'ebook>> {
        match key.into() {
            ReaderKey::Value(idref) => {
                let (index, content) = self.find_entry_by_str(idref)?;
                self.cursor.set(index);
                Ok(content)
            }
            ReaderKey::Position(index) if index < self.entries.len() => {
                let content = self.find_entry_by_position(index);
                self.cursor.set(index);
                content
            }
            ReaderKey::Position(index) => Err(ReaderError::OutOfBounds {
                position: index,
                len: self.entries.len(),
            }),
        }
    }

    /// Moves the reader’s cursor to the given [`ReaderKey`]
    /// and returns the resulting cursor position.
    #[doc = util::inherent_doc!(Reader, seek)]
    pub fn seek<'a>(&mut self, key: impl Into<ReaderKey<'a>>) -> ReaderResult<usize> {
        match key.into() {
            ReaderKey::Value(idref) => {
                let index = self.find_entry_by_idref(idref)?;
                self.cursor.set(index);
                Ok(index)
            }
            ReaderKey::Position(index) if index < self.entries.len() => {
                self.cursor.set(index);
                Ok(index)
            }
            ReaderKey::Position(index) => Err(ReaderError::OutOfBounds {
                position: index,
                len: self.entries.len(),
            }),
        }
    }

    /// Returns the [`EpubReaderContent`] at the given [`ReaderKey`]
    /// without moving the reader's cursor.
    #[doc = util::inherent_doc!(Reader, get)]
    pub fn get<'a>(
        &self,
        key: impl Into<ReaderKey<'a>>,
    ) -> ReaderResult<EpubReaderContent<'ebook>> {
        match key.into() {
            ReaderKey::Value(manifest_id) => self
                .find_entry_by_str(manifest_id)
                .map(|(_, content)| content),
            ReaderKey::Position(index) => self.find_entry_by_position(index),
        }
    }

    /// The total number of traversable [`EpubReaderContent`] entries.
    #[doc = util::inherent_doc!(Reader, len)]
    pub fn len(&self) -> usize {
        self.entries.len()
    }

    /// The position of the reader’s cursor (current entry).
    #[doc = util::inherent_doc!(Reader, current_position)]
    pub fn current_position(&self) -> Option<usize> {
        self.cursor.index()
    }

    /// The total number of remaining traversable [`EpubReaderContent`]
    /// until the reader's cursor reaches the end.
    #[doc = util::inherent_doc!(Reader, remaining)]
    pub fn remaining(&self) -> usize {
        Reader::remaining(self)
    }

    /// Returns `true` if the reader has no [`EpubReaderContent`] to provide;
    /// a [length](EpubReader::len) of `0`.
    #[doc = util::inherent_doc!(Reader, is_empty)]
    pub fn is_empty(&self) -> bool {
        Reader::is_empty(self)
    }
}

impl Sealed for EpubReader<'_> {}

#[allow(refining_impl_trait)]
impl<'ebook> Reader<'ebook> for EpubReader<'ebook> {
    fn reset(&mut self) {
        self.reset()
    }

    fn read_next(&mut self) -> Option<ReaderResult<EpubReaderContent<'ebook>>> {
        self.read_next()
    }

    fn read_prev(&mut self) -> Option<ReaderResult<EpubReaderContent<'ebook>>> {
        self.read_prev()
    }

    fn read_current(&self) -> Option<ReaderResult<EpubReaderContent<'ebook>>> {
        self.read_current()
    }

    fn read<'a>(
        &mut self,
        key: impl Into<ReaderKey<'a>>,
    ) -> ReaderResult<EpubReaderContent<'ebook>> {
        self.read(key)
    }

    fn seek<'a>(&mut self, key: impl Into<ReaderKey<'a>>) -> ReaderResult<usize> {
        self.seek(key)
    }

    fn get<'a>(&self, key: impl Into<ReaderKey<'a>>) -> ReaderResult<EpubReaderContent<'ebook>> {
        self.get(key)
    }

    fn len(&self) -> usize {
        self.len()
    }

    fn current_position(&self) -> Option<usize> {
        self.current_position()
    }
}

impl<'ebook> Iterator for EpubReader<'ebook> {
    type Item = ReaderResult<EpubReaderContent<'ebook>>;

    fn next(&mut self) -> Option<Self::Item> {
        self.read_next()
    }
}

/// [`ReaderContent`] implementation for an [`EpubReader`].
#[derive(Clone, Debug, PartialEq)]
pub struct EpubReaderContent<'ebook> {
    content: String,
    position: usize,
    spine_entry: EpubSpineEntry<'ebook>,
    manifest_entry: EpubManifestEntry<'ebook>,
}

impl<'ebook> EpubReaderContent<'ebook> {
    /// The position of reader content within an [`EpubReader`] (0-index-based).
    #[doc = util::inherent_doc!(ReaderContent, position)]
    pub fn position(&self) -> usize {
        self.position
    }

    /// The readable content (e.g., `XHTML`, `HTML`, etc.).
    #[doc = util::inherent_doc!(ReaderContent, content)]
    pub fn content(&self) -> &str {
        self.content.as_str()
    }

    /// The associated [`EpubSpineEntry`] containing reading order details.
    #[doc = util::inherent_doc!(ReaderContent, spine_entry)]
    pub fn spine_entry(&self) -> EpubSpineEntry<'ebook> {
        self.spine_entry
    }

    /// The associated [`EpubManifestEntry`] containing resource details.
    #[doc = util::inherent_doc!(ReaderContent, manifest_entry)]
    pub fn manifest_entry(&self) -> EpubManifestEntry<'ebook> {
        self.manifest_entry
    }

    /// Takes the contained readable content string.
    #[doc = util::inherent_doc!(ReaderContent, into_string)]
    pub fn into_string(self) -> String {
        ReaderContent::into_string(self)
    }

    /// Takes the contained readable content bytes.
    #[doc = util::inherent_doc!(ReaderContent, into_bytes)]
    pub fn into_bytes(self) -> Vec<u8> {
        ReaderContent::into_bytes(self)
    }
}

impl Sealed for EpubReaderContent<'_> {}

#[allow(refining_impl_trait)]
impl<'ebook> ReaderContent<'ebook> for EpubReaderContent<'ebook> {
    fn position(&self) -> usize {
        self.position()
    }

    fn content(&self) -> &str {
        self.content()
    }

    fn spine_entry(&self) -> EpubSpineEntry<'ebook> {
        self.spine_entry()
    }

    fn manifest_entry(&self) -> EpubManifestEntry<'ebook> {
        self.manifest_entry()
    }
}

impl<'ebook> From<EpubReaderContent<'ebook>> for String {
    fn from(value: EpubReaderContent<'ebook>) -> Self {
        value.content
    }
}

impl<'ebook> From<EpubReaderContent<'ebook>> for Vec<u8> {
    fn from(value: EpubReaderContent<'ebook>) -> Self {
        value.content.into_bytes()
    }
}

/// Indicates arrangement/omission of `linear` and `non-linear` spine content
/// within an [`Epub`].
///
/// # See Also
/// - [`EpubSpineEntry::is_linear`] for the difference between `linear` and `non-linear` content.
///
/// Default: [`LinearBehavior::Original`]
#[derive(Copy, Clone, Debug, Default, Hash, PartialEq, Eq)]
pub enum LinearBehavior {
    /// `Linear` and `non-linear` content is retained in the original order
    /// written in the [`EpubSpine`](super::spine::EpubSpine).
    #[default]
    Original,
    /// Only `linear` content is retained; `non-linear` content is omitted.
    ///
    /// Content: `[linear…]`
    LinearOnly,
    /// Only `non-linear` content is retained; `linear` content is omitted.
    ///
    /// Content: `[non_linear…]`
    NonLinearOnly,
    /// `non-linear` content is prepended before `linear` content.
    ///
    /// Content: `[non_linear…, linear…]`
    PrependNonLinear,
    /// `non-linear` content is appended after `linear` content.
    ///
    /// Content: `[linear…, non_linear…]`
    AppendNonLinear,
}

#[derive(Clone, Debug)]
pub(super) struct EpubReaderConfig {
    /// See [`EpubReaderOptions::linear_behavior`]
    linear_behavior: LinearBehavior,
}

impl Default for EpubReaderConfig {
    fn default() -> Self {
        Self {
            linear_behavior: LinearBehavior::Original,
        }
    }
}

/// Configuration to create an [`EpubReader`].
///
/// `EpubReaderOptions` supports two usage patterns:
/// 1. **Attached**:
///    Created via [`Epub::reader_builder`].
///    The options are bound to a specific [`Epub`].
///    Terminal methods (e.g., [`create`](EpubReaderOptions::<&Epub>::create)) consume the builder.
/// 2. **Detached**:
///    Created via [`EpubReaderOptions::default`].
///    The options are standalone.
///    Terminal methods take `&self`
///    (e.g., [`create`](EpubReaderOptions::create)),
///    and a reference to an [`Epub`], allowing the same configuration to be reused multiple times.
///
/// # Options
/// ## Ordering
/// - [`linear_behavior`](EpubReaderOptions::linear_behavior)
///   (Default: [`LinearBehavior::Original`])
///
/// # See Also
/// - [`Epub::reader_builder`] to create an [`EpubReader`] directly from an [`Epub`].
/// - [`EpubReaderOptions::default`] to create multiple [`EpubReader`] instances with identical options.
///
/// # Examples
/// - Creating an [`EpubReader`] (Attached):
/// ```
/// # use rbook::Epub;
/// # use rbook::epub::reader::LinearBehavior;
/// # fn main() -> rbook::ebook::errors::EbookResult<()> {
/// let epub = Epub::open("tests/ebooks/example_epub")?;
/// let mut reader = epub.reader_builder() // returns EpubReaderOptions
///     .linear_behavior(LinearBehavior::AppendNonLinear)
///     .create();
/// # Ok(())
/// # }
/// ```
/// - Creating multiple [`EpubReader`] instances (Detached):
/// ```
/// # use rbook::Epub;
/// # use rbook::epub::reader::{EpubReaderOptions, LinearBehavior};
/// # fn main() -> rbook::ebook::errors::EbookResult<()> {
/// let epub = Epub::open("tests/ebooks/example_epub")?;
/// let reader_options = EpubReaderOptions::new()
///     .linear_behavior(LinearBehavior::PrependNonLinear);
///
/// let mut reader_a = reader_options.create(&epub);
/// let mut reader_b = reader_options.create(&epub);
/// let mut reader_c = reader_options.create(&epub);
///
/// // All have the same applied options and initial state
/// assert_eq!(reader_a, reader_b);
/// assert_eq!(reader_b, reader_c);
/// # Ok(())
/// # }
/// ```
#[non_exhaustive]
#[derive(Clone, Debug, Default)]
pub struct EpubReaderOptions<T = ()> {
    container: T,
    config: EpubReaderConfig,
}

impl<T> EpubReaderOptions<T> {
    /// How `linear` and `non-linear` spine content are handled.
    ///
    /// Through this setting, content can be re-arranged or omitted
    /// depending on the selected [`LinearBehavior`].
    ///
    /// Default: [`LinearBehavior::Original`]
    pub fn linear_behavior(mut self, linear_behavior: LinearBehavior) -> Self {
        self.config.linear_behavior = linear_behavior;
        self
    }
}

impl<'ebook> EpubReaderOptions<&'ebook Epub> {
    pub(super) fn new(epub: &'ebook Epub) -> Self {
        Self {
            container: epub,
            config: EpubReaderConfig::default(),
        }
    }

    /// Consume this builder and create an [`EpubReader`].
    pub fn create(self) -> EpubReader<'ebook> {
        EpubReader::new(self.container, self.config)
    }
}

impl EpubReaderOptions {
    /// Creates a new builder with default values.
    ///
    /// # See Also
    /// - [`Epub::reader_builder`] to build an [`EpubReader`] directly from an [`Epub`]
    pub fn new() -> Self {
        Self::default()
    }

    /// Consume this builder and create an [`EpubReader`] associated with the given [`Epub`].
    pub fn create<'ebook>(&self, epub: &'ebook Epub) -> EpubReader<'ebook> {
        EpubReader::new(epub, self.config.clone())
    }
}
