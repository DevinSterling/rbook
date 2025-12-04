//! EPUB-specific spine content.

use crate::ebook::element::{AttributeData, Attributes, Properties, PropertiesData};
use crate::ebook::epub::manifest::{EpubManifestEntry, EpubManifestEntryProvider};
use crate::ebook::epub::metadata::{EpubRefinements, EpubRefinementsData};
use crate::ebook::spine::{PageDirection, Spine, SpineEntry};
use std::cmp::Ordering;
use std::fmt::{Debug, Formatter};
use std::slice::Iter as SliceIter;

////////////////////////////////////////////////////////////////////////////////
// PRIVATE API
////////////////////////////////////////////////////////////////////////////////

#[derive(Debug, PartialEq)]
pub(super) struct EpubSpineData {
    page_direction: PageDirection,
    entries: Vec<EpubSpineEntryData>,
}

impl EpubSpineData {
    pub(super) fn new(page_direction: PageDirection, entries: Vec<EpubSpineEntryData>) -> Self {
        Self {
            page_direction,
            entries,
        }
    }

    pub(super) fn empty() -> Self {
        Self {
            page_direction: PageDirection::Default,
            entries: Vec::new(),
        }
    }
}

#[derive(Debug, Hash, PartialEq)]
pub(super) struct EpubSpineEntryData {
    pub(super) id: Option<String>,
    pub(super) order: usize,
    pub(super) idref: String,
    pub(super) linear: bool,
    pub(super) properties: PropertiesData,
    pub(super) attributes: Vec<AttributeData>,
    pub(super) refinements: EpubRefinementsData,
}

////////////////////////////////////////////////////////////////////////////////
// PUBLIC API
////////////////////////////////////////////////////////////////////////////////

/// An EPUB spine, see [`Spine`] for more details.
#[derive(Copy, Clone)]
pub struct EpubSpine<'ebook> {
    /// Manifest entry provider for resource lookup within the spine itself
    provider: EpubManifestEntryProvider<'ebook>,
    data: &'ebook EpubSpineData,
}

impl<'ebook> EpubSpine<'ebook> {
    pub(super) fn new(
        provider: EpubManifestEntryProvider<'ebook>,
        data: &'ebook EpubSpineData,
    ) -> Self {
        Self { provider, data }
    }

    fn by_predicate(
        &self,
        predicate: impl Fn(&EpubSpineEntryData) -> bool,
    ) -> Option<EpubSpineEntry<'ebook>> {
        self.data
            .entries
            .iter()
            .find(|&data| predicate(data))
            .map(|data| EpubSpineEntry::new(self.provider, data))
    }

    /// Returns the [`EpubSpineEntry`] that matches the given `id` if present,
    /// otherwise [`None`].
    ///
    /// # See Also
    /// - [`Self::by_idref`] to retrieve a spine entry by the [`id`](EpubManifestEntry::id)
    ///   of an [`EpubManifestEntry`].
    ///
    /// # Examples
    /// - Retrieving a spine entry by its id:
    /// ```
    /// # use rbook::ebook::errors::EbookResult;
    /// # use rbook::ebook::spine::SpineEntry;
    /// # use rbook::{Ebook, Epub};
    /// # fn main() -> EbookResult<()> {
    /// let epub = Epub::open("tests/ebooks/example_epub")?;
    ///
    /// let spine_entry = epub.spine().by_id("supplementary").unwrap();
    /// assert_eq!(Some("supplementary"), spine_entry.id());
    /// assert_eq!(3, spine_entry.order());
    ///
    /// // Attempt to retrieve a non-existent entry
    /// assert_eq!(None, epub.spine().by_id("end"));
    /// # Ok(())
    /// # }
    /// ```
    pub fn by_id(&self, id: &str) -> Option<EpubSpineEntry<'ebook>> {
        self.by_predicate(|data| data.id.as_deref() == Some(id))
    }

    /// Returns the [`EpubSpineEntry`] that matches the given `idref` if present,
    /// otherwise [`None`].
    ///
    /// An [`idref`](EpubSpineEntry::idref) is the [`id`](EpubManifestEntry::id) of a
    /// [`EpubManifestEntry`] referenced by a spine entry.
    ///
    /// # Examples
    /// - Retrieving a spine entry by its idref:
    /// ```
    /// # use rbook::ebook::errors::EbookResult;
    /// # use rbook::ebook::spine::SpineEntry;
    /// # use rbook::{Ebook, Epub};
    /// # fn main() -> EbookResult<()> {
    /// let epub = Epub::open("tests/ebooks/example_epub")?;
    ///
    /// let spine_entry = epub.spine().by_idref("c1").unwrap();
    /// assert_eq!("c1", spine_entry.idref());
    /// assert_eq!(2, spine_entry.order());
    ///
    /// let spine_entry = epub.spine().by_idref("c2").unwrap();
    /// assert_eq!("c2", spine_entry.idref());
    /// assert_eq!(4, spine_entry.order());
    ///
    /// // Attempt to retrieve a non-existent entry
    /// assert_eq!(None, epub.spine().by_idref("c999"));
    /// # Ok(())
    /// # }
    /// ```
    pub fn by_idref(&self, idref: &str) -> Option<EpubSpineEntry<'ebook>> {
        self.by_predicate(|data| data.idref == idref)
    }
}

#[allow(refining_impl_trait)]
impl<'ebook> Spine<'ebook> for EpubSpine<'ebook> {
    fn page_direction(&self) -> PageDirection {
        self.data.page_direction
    }

    fn len(&self) -> usize {
        self.data.entries.len()
    }

    fn by_order(&self, order: usize) -> Option<EpubSpineEntry<'ebook>> {
        self.data
            .entries
            .get(order)
            .map(|data| EpubSpineEntry::new(self.provider, data))
    }

    fn entries(&self) -> EpubSpineIter<'ebook> {
        EpubSpineIter {
            provider: self.provider,
            iter: self.data.entries.iter(),
        }
    }
}

impl Debug for EpubSpine<'_> {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("EpubSpine")
            .field("data", self.data)
            .finish_non_exhaustive()
    }
}

impl PartialEq for EpubSpine<'_> {
    fn eq(&self, other: &Self) -> bool {
        self.data == other.data
    }
}

impl<'ebook> IntoIterator for &EpubSpine<'ebook> {
    type Item = EpubSpineEntry<'ebook>;
    type IntoIter = EpubSpineIter<'ebook>;

    fn into_iter(self) -> EpubSpineIter<'ebook> {
        self.entries()
    }
}

impl<'ebook> IntoIterator for EpubSpine<'ebook> {
    type Item = EpubSpineEntry<'ebook>;
    type IntoIter = EpubSpineIter<'ebook>;

    fn into_iter(self) -> EpubSpineIter<'ebook> {
        self.entries()
    }
}

/// An iterator over all the [`entries`](EpubSpineEntry) of an [`EpubSpine`].
///
/// # See Also
/// - [`EpubSpine::entries`]
///
/// # Examples
/// - Iterating over all manifest entries:
/// ```
/// # use rbook::ebook::errors::EbookResult;
/// # use rbook::{Ebook, Epub};
/// # fn main() -> EbookResult<()> {
/// let epub = Epub::open("tests/ebooks/example_epub")?;
///
/// for entry in epub.spine() {
///     // process entry //
/// }
/// # Ok(())
/// # }
/// ```
pub struct EpubSpineIter<'ebook> {
    provider: EpubManifestEntryProvider<'ebook>,
    iter: SliceIter<'ebook, EpubSpineEntryData>,
}

impl<'ebook> Iterator for EpubSpineIter<'ebook> {
    type Item = EpubSpineEntry<'ebook>;

    fn next(&mut self) -> Option<Self::Item> {
        self.iter
            .next()
            .map(|data| EpubSpineEntry::new(self.provider, data))
    }
}

/// An entry contained within an [`EpubSpine`], encompassing associated metadata.
#[derive(Copy, Clone)]
pub struct EpubSpineEntry<'ebook> {
    provider: EpubManifestEntryProvider<'ebook>,
    data: &'ebook EpubSpineEntryData,
}

impl<'ebook> EpubSpineEntry<'ebook> {
    fn new(provider: EpubManifestEntryProvider<'ebook>, data: &'ebook EpubSpineEntryData) -> Self {
        Self { provider, data }
    }

    /// The unique id of a spine entry.
    pub fn id(&self) -> Option<&'ebook str> {
        self.data.id.as_deref()
    }

    /// The unique id reference to an [`EpubManifestEntry`] in the
    /// [`EpubManifest`](super::EpubManifest).
    ///
    /// For direct access to the resource, [`Self::resource`] or
    /// [`Self::manifest_entry`] is preferred.
    pub fn idref(&self) -> &'ebook str {
        &self.data.idref
    }

    /// Returns `true` if a spine entry’s `linear` attribute is `yes`
    /// (or is not specified).    
    ///
    /// When `true`, the entry is part of the default reading order.
    /// Otherwise, it is identified as supplementary content,
    /// which may be skipped or treated differently by applications.
    ///
    /// Regarding an [`EpubReader`](super::EpubReader), linear and non-linear content
    /// is shown in the exact order as written in the spine.
    /// This behavior can be changed through
    /// [`EpubReaderOptions::linear_behavior`](super::EpubReaderOptions::linear_behavior).
    pub fn is_linear(&self) -> bool {
        self.data.linear
    }

    /// The [`Properties`] associated with a spine entry.
    ///
    /// While not limited to, potential contained property values are:
    /// - `page-spread-left`
    /// - `page-spread-right`
    /// - `rendition:page-spread-left`
    /// - `rendition:page-spread-right`
    /// - `rendition:page-spread-center`
    ///
    /// See the specification for more details regarding properties:
    /// <https://www.w3.org/TR/epub/#app-itemref-properties-vocab>
    pub fn properties(&self) -> Properties<'ebook> {
        (&self.data.properties).into()
    }

    /// All additional `XML` [`Attributes`].
    ///
    /// # Omitted Attributes
    /// The following attributes will **not** be found within the returned collection:
    /// - [`id`](Self::id)
    /// - [`idref`](Self::idref)
    /// - [`linear`](Self::is_linear)
    /// - [`properties`](Self::properties)
    pub fn attributes(&self) -> Attributes<'ebook> {
        (&self.data.attributes).into()
    }

    /// Complementary refinement metadata entries.
    pub fn refinements(&self) -> EpubRefinements<'ebook> {
        (&self.data.refinements).into()
    }
}

#[allow(refining_impl_trait)]
impl<'ebook> SpineEntry<'ebook> for EpubSpineEntry<'ebook> {
    fn order(&self) -> usize {
        self.data.order
    }

    fn manifest_entry(&self) -> Option<EpubManifestEntry<'ebook>> {
        self.provider.by_id(self.idref())
    }
}

impl Debug for EpubSpineEntry<'_> {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("EpubSpineEntry")
            .field("data", self.data)
            .finish_non_exhaustive()
    }
}

impl Ord for EpubSpineEntry<'_> {
    fn cmp(&self, other: &Self) -> Ordering {
        self.order().cmp(&other.order())
    }
}

impl Eq for EpubSpineEntry<'_> {}

impl PartialEq for EpubSpineEntry<'_> {
    fn eq(&self, other: &Self) -> bool {
        self.data == other.data
    }
}

impl PartialOrd<Self> for EpubSpineEntry<'_> {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}
