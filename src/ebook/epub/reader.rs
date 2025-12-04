//! [`Reader`]-specific implementations for the [`Epub`] format.

use crate::ebook::Ebook;
use crate::ebook::epub::Epub;
use crate::ebook::epub::errors::EpubFormatError;
use crate::ebook::epub::manifest::EpubManifestEntry;
use crate::ebook::epub::spine::EpubSpineEntry;
use crate::ebook::manifest::ManifestEntry;
use crate::ebook::spine::{Spine, SpineEntry};
use crate::reader::errors::{ReaderError, ReaderResult};
use crate::reader::{Reader, ReaderContent, ReaderKey};
use crate::util::IndexCursor;
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
/// # use rbook::{Ebook, Epub};
/// # use rbook::epub::reader::LinearBehavior;
/// # use rbook::reader::{Reader, ReaderContent};
/// # use std::error::Error;
/// # fn main() -> Result<(), Box<dyn Error>> {
/// let epub = Epub::open("tests/ebooks/example_epub")?;
/// let mut reader = epub.reader_builder()
///     .linear_behavior(LinearBehavior::LinearOnly) // Omit non-linear readable entries
///     .create();
/// # let mut count = 0;
///
/// // Stream over all linear content
/// for content_result in &mut reader {
///     # count += 1;
///     let content = content_result.unwrap();
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
        let iterator = epub.spine().entries();
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
            ReaderError::MalformedEbook(
                EpubFormatError::MissingAttribute(format!(
                    "Invalid spine idref - Resource with id of `{}` not found within the manifest",
                    spine_entry.idref(),
                ))
                .into(),
            )
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
}

#[allow(refining_impl_trait)]
impl<'ebook> Reader<'ebook> for EpubReader<'ebook> {
    fn reset(&mut self) {
        self.cursor.reset();
    }

    fn read_next(&mut self) -> Option<ReaderResult<EpubReaderContent<'ebook>>> {
        self.cursor
            .increment()
            .map(|index| self.find_entry_by_position(index))
    }

    fn read_prev(&mut self) -> Option<ReaderResult<EpubReaderContent<'ebook>>> {
        self.cursor
            .decrement()
            .map(|index| self.find_entry_by_position(index))
    }

    fn read_current(&self) -> Option<ReaderResult<EpubReaderContent<'ebook>>> {
        self.current_position()
            .map(|position| self.find_entry_by_position(position))
    }

    fn read<'a>(
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

    fn seek<'a>(&mut self, key: impl Into<ReaderKey<'a>>) -> ReaderResult<usize> {
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

    fn get<'a>(&self, key: impl Into<ReaderKey<'a>>) -> ReaderResult<EpubReaderContent<'ebook>> {
        match key.into() {
            ReaderKey::Value(manifest_id) => self
                .find_entry_by_str(manifest_id)
                .map(|(_, content)| content),
            ReaderKey::Position(index) => self.find_entry_by_position(index),
        }
    }

    fn len(&self) -> usize {
        self.entries.len()
    }

    fn current_position(&self) -> Option<usize> {
        self.cursor.index()
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

#[allow(refining_impl_trait)]
impl<'ebook> ReaderContent<'ebook> for EpubReaderContent<'ebook> {
    fn position(&self) -> usize {
        self.position
    }

    fn content(&self) -> &str {
        self.content.as_str()
    }

    fn spine_entry(&self) -> EpubSpineEntry<'ebook> {
        self.spine_entry
    }

    fn manifest_entry(&self) -> EpubManifestEntry<'ebook> {
        self.manifest_entry
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
    /// Content: `[linear...]`
    LinearOnly,
    /// Only `non-linear` content is retained; `linear` content is omitted.
    ///
    /// Content: `[non_linear...]`
    NonLinearOnly,
    /// `non-linear` content is prepended before `linear` content.
    ///
    /// Content: `[non_linear..., linear...]`
    PrependNonLinear,
    /// `non-linear` content is appended after `linear` content.
    ///
    /// Content: `[linear..., non_linear...]`
    AppendNonLinear,
}

pub(super) struct EpubReaderConfig {
    /// See [`EpubReaderOptions::linear_behavior`]
    linear_behavior: LinearBehavior,
}

// Temporary placeholder for now until 0.7.0
#[allow(deprecated)]
impl From<EpubReaderOptions> for EpubReaderConfig {
    fn from(settings: EpubReaderOptions) -> Self {
        Self {
            linear_behavior: settings.linear_behavior,
        }
    }
}

// BACKWARD COMPATIBILITY (Renamed)
/// Deprecated; prefer [`EpubReaderOptions`] instead.
#[deprecated(since = "0.6.8", note = "Use `EpubReaderOptions` instead.")]
pub type EpubReaderSettings = EpubReaderOptions;
/// Deprecated; prefer [`EpubReaderOptions`] instead.
#[deprecated(since = "0.6.8", note = "Use `EpubReaderOptions` instead.")]
pub type EpubReaderSettingsBuilder = EpubReaderOptions;

/// Builder to create an [`EpubReader`].
///
/// Configurable options:
/// - [`linear_behavior`](EpubReaderOptions::linear_behavior)
///
/// # See Also
/// - [`Epub::reader_builder`] to create an [`EpubReader`] directly from an [`Epub`].
///
/// # Examples
/// - Creating multiple [`EpubReader`] instances:
/// ```
/// # use rbook::ebook::errors::EbookResult;
/// # use rbook::epub::reader::{EpubReaderOptions, LinearBehavior};
/// # use rbook::{Ebook, Epub};
/// # fn main() -> EbookResult<()> {
/// let epub = Epub::open("tests/ebooks/example_epub")?;
/// let reader_options = EpubReaderOptions::new()
///     .linear_behavior(LinearBehavior::PrependNonLinear);
///
/// let mut reader_a = reader_options.clone().create(&epub);
/// let mut reader_b = reader_options.clone().create(&epub);
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
pub struct EpubReaderOptions {
    /// See [`EpubReaderOptions::linear_behavior`].
    #[deprecated(since = "0.6.8", note = "Use `linear_behavior` method instead.")]
    pub linear_behavior: LinearBehavior,
}

impl EpubReaderOptions {
    /// Creates a new builder with default values.
    ///
    /// # See Also
    /// - [`Epub::reader_builder`] to build an [`EpubReader`] directly from an [`Epub`]
    pub fn new() -> Self {
        Self::default()
    }

    /// How `linear` and `non-linear` spine content are handled.
    ///
    /// Through this setting, content can be re-arranged or omitted
    /// depending on the selected [`LinearBehavior`].
    ///
    /// Default: [`LinearBehavior::Original`]
    #[allow(deprecated)]
    pub fn linear_behavior(mut self, linear_behavior: LinearBehavior) -> Self {
        self.linear_behavior = linear_behavior;
        self
    }

    /// Consume this builder and create an [`EpubReader`] associated with the given [`Epub`].
    pub fn create(self, epub: &Epub) -> EpubReader<'_> {
        EpubReader::new(epub, self.into())
    }

    /// Turn this builder into an [`EpubReaderOptions`] instance.
    #[deprecated(since = "0.6.8", note = "Use `Epub::reader_builder` instead.")]
    pub fn build(self) -> Self {
        self
    }

    /// Returns a builder to create an [`EpubReaderOptions`] instance.
    #[allow(deprecated)]
    #[deprecated(since = "0.6.8", note = "Use `EpubReaderOptions::new` instead.")]
    pub fn builder() -> Self {
        Self::default()
    }
}

/// Builder to create an [`EpubReader`] associated with an [`Epub`].
///
/// # See Also
/// - [`EpubReaderOptions`] to create multiple [`EpubReader`] instances with identical options.
///
/// # Examples
/// - Creating an [`EpubReader`]:
/// ```
/// # use rbook::ebook::errors::EbookResult;
/// # use rbook::epub::reader::LinearBehavior;
/// # use rbook::{Ebook, Epub};
/// # fn main() -> EbookResult<()> {
/// let epub = Epub::open("tests/ebooks/example_epub")?;
/// let mut reader = epub.reader_builder() // returns EpubReaderBuilder
///     .linear_behavior(LinearBehavior::AppendNonLinear)
///     .create();
/// # Ok(())
/// # }
/// ```
pub struct EpubReaderBuilder<'ebook> {
    epub: &'ebook Epub,
    settings: EpubReaderOptions,
}

impl<'ebook> EpubReaderBuilder<'ebook> {
    pub(crate) fn new(epub: &'ebook Epub) -> Self {
        Self {
            epub,
            settings: EpubReaderOptions::default(),
        }
    }

    /// Consume this builder and create an [`EpubReader`].
    pub fn create(self) -> EpubReader<'ebook> {
        self.settings.create(self.epub)
    }

    /// See [`EpubReaderOptions::linear_behavior`].
    pub fn linear_behavior(mut self, linear_behavior: LinearBehavior) -> Self {
        self.settings = self.settings.linear_behavior(linear_behavior);
        self
    }
}
