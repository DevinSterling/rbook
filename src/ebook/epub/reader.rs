//! [`Reader`]-related implementations for the [`Epub`] format.

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
/// can be configured using [`EpubReaderSettings`].
///
/// # Examples
/// - Retrieving a new EPUB reader instance with configuration:
/// ```
/// # use rbook::{Ebook, Epub};
/// # use rbook::epub::reader::{EpubReaderSettings, LinearBehavior};
/// # use rbook::reader::{Reader, ReaderContent};
/// # use std::error::Error;
/// # fn main() -> Result<(), Box<dyn Error>> {
/// let epub = Epub::open("tests/ebooks/example_epub")?;
/// let mut epub_reader = epub.reader_with(
///     // Omit non-linear readable entries
///     EpubReaderSettings::builder().linear_behavior(LinearBehavior::LinearOnly),
/// );
/// # let mut count = 0;
///
/// // Stream over all linear content
/// for content_result in &mut epub_reader {
///     # count += 1;
///     let content = content_result.unwrap();
///     assert!(content.spine_entry().is_linear());
/// }
/// # assert_eq!(3, count);
/// # assert_eq!(count, epub_reader.len());
/// # Ok(())
/// # }
/// ```
#[derive(Clone, Debug, PartialEq)]
pub struct EpubReader<'ebook> {
    epub: &'ebook Epub,
    entries: Vec<EpubSpineEntry<'ebook>>,
    cursor: IndexCursor,
}

impl<'ebook> EpubReader<'ebook> {
    pub(super) fn new(epub: &'ebook Epub, settings: EpubReaderSettings) -> Self {
        let entries = Self::get_entries(epub, settings.linear_behavior);

        EpubReader {
            cursor: IndexCursor::new(entries.len()),
            epub,
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

                if let LinearBehavior::AppendNonLinear = &behavior {
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
        &self,
        spine_entry: EpubSpineEntry<'ebook>,
    ) -> ReaderResult<EpubManifestEntry<'ebook>> {
        spine_entry.manifest_entry().ok_or_else(|| {
            ReaderError::MalformedEbook(
                EpubFormatError::MissingAttribute(format!(
                    "spine idref of `{}` not found within the manifest",
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
        let manifest_entry = self.get_manifest_entry(spine_entry)?;

        self.crate_reader_content(position, spine_entry, manifest_entry)
    }

    fn find_entry_by_str(&self, idref: &str) -> ReaderResult<(usize, EpubReaderContent<'ebook>)> {
        let position = self.find_entry_by_idref(idref)?;
        let spine_entry = self.entries[position];
        let manifest_entry = self.get_manifest_entry(spine_entry)?;

        Ok((
            position,
            self.crate_reader_content(position, spine_entry, manifest_entry)?,
        ))
    }

    fn crate_reader_content(
        &self,
        position: usize,
        spine_entry: EpubSpineEntry<'ebook>,
        manifest_entry: EpubManifestEntry<'ebook>,
    ) -> ReaderResult<EpubReaderContent<'ebook>> {
        Ok(EpubReaderContent {
            content: self.epub.read_resource_str(manifest_entry.resource())?,
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
        Reader::read_next(self)
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

/// [`EpubReader`]-specific settings provided to [`Epub::reader_with`].
///
/// Create a mutable [`EpubReaderSettings`] instance via
/// [`EpubReaderSettings::builder`] or [`EpubReaderSettings::default`].
#[non_exhaustive]
#[derive(Clone, Debug, Default)]
pub struct EpubReaderSettings {
    /// How `linear` and `non-linear` spine content are handled.
    ///
    /// Through this setting, content can be re-arranged or omitted
    /// depending on the selected [`LinearBehavior`].
    ///
    /// Default: [`LinearBehavior::Original`]
    pub linear_behavior: LinearBehavior,
}

impl EpubReaderSettings {
    /// Returns a builder to create an [`EpubReaderSettings`] instance.
    pub fn builder() -> EpubReaderSettingsBuilder {
        EpubReaderSettingsBuilder(Default::default())
    }
}

impl From<EpubReaderSettingsBuilder> for EpubReaderSettings {
    fn from(builder: EpubReaderSettingsBuilder) -> Self {
        builder.build()
    }
}

/// Indicates arrangement/omission of `linear` and `non-linear` spine content
/// within an [`Epub`].
///
/// See [`EpubSpineEntry::is_linear`] for the difference between `linear` and `non-linear` content.
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

/// Builder to construct an [`EpubReaderSettings`] instance.
///
/// # Examples
/// - Passing a builder to create an [`EpubReader`] with:
/// ```
/// # use rbook::ebook::errors::EbookResult;
/// # use rbook::epub::EpubSettings;
/// # use rbook::epub::reader::{EpubReaderSettings, LinearBehavior};
/// # use rbook::{Ebook, Epub};
/// # fn main() -> EbookResult<()> {
/// let epub = Epub::open("tests/ebooks/example_epub")?;
/// let reader = epub.reader_with(
///     EpubReaderSettings::builder().linear_behavior(LinearBehavior::AppendNonLinear),
/// );
/// # Ok(())
/// # }
/// ```
#[derive(Clone, Debug)]
pub struct EpubReaderSettingsBuilder(EpubReaderSettings);

impl EpubReaderSettingsBuilder {
    /// Turn this builder into an [`EpubReaderSettings`] instance.
    pub fn build(self) -> EpubReaderSettings {
        self.0
    }

    /// See [`EpubReaderSettings::linear_behavior`].
    pub fn linear_behavior(mut self, linear_behavior: LinearBehavior) -> Self {
        self.0.linear_behavior = linear_behavior;
        self
    }
}
