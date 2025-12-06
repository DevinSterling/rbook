//! EPUB-specific spine content.

use crate::ebook::element::{AttributeData, Attributes, Properties, PropertiesData};
use crate::ebook::epub::SynchronousArchive;
use crate::ebook::epub::manifest::{EpubManifestEntryData, EpubManifestEntryProvider};
use crate::ebook::epub::metadata::{EpubRefinements, EpubRefinementsData};
use crate::ebook::spine::{PageDirection, Spine, SpineEntry};
use std::cmp::Ordering;
use std::fmt::{Debug, Formatter};
use std::slice::Iter as SliceIter;

////////////////////////////////////////////////////////////////////////////////
// PRIVATE API
////////////////////////////////////////////////////////////////////////////////

#[derive(Debug, PartialEq)]
pub(super) struct InternalEpubSpine {
    page_direction: PageDirection,
    entries: Vec<InternalEpubSpineEntry>,
}

impl InternalEpubSpine {
    pub(super) fn new(page_direction: PageDirection, entries: Vec<InternalEpubSpineEntry>) -> Self {
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
pub(super) struct InternalEpubSpineEntry {
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

/// todo
pub type EpubSpine<'ebook> = EpubSpineData<'ebook, &'ebook SynchronousArchive>;
/// todo
pub type EpubSpineEntry<'ebook> = EpubSpineEntryData<'ebook, &'ebook SynchronousArchive>;

/// An EPUB spine, see [`Spine`] for more details.
#[derive(Copy, Clone)]
pub struct EpubSpineData<'ebook, A> {
    /// Manifest entry provider for resource lookup within the spine itself
    provider: EpubManifestEntryProvider<'ebook, A>,
    data: &'ebook InternalEpubSpine,
}

impl<'ebook, A: Copy> EpubSpineData<'ebook, A> {
    pub(super) fn new(
        provider: EpubManifestEntryProvider<'ebook, A>,
        data: &'ebook InternalEpubSpine,
    ) -> Self {
        Self { provider, data }
    }

    fn by_predicate(
        &self,
        predicate: impl Fn(&InternalEpubSpineEntry) -> bool,
    ) -> Option<EpubSpineEntryData<'ebook, A>> {
        self.data
            .entries
            .iter()
            .find(|&data| predicate(data))
            .map(|data| EpubSpineEntryData::new(self.provider, data))
    }

    /// Returns the [`EpubSpineEntryData`] that matches the given `id` if present,
    /// otherwise [`None`].
    ///
    /// # See Also
    /// - [`Self::by_idref`] to retrieve a spine entry by the [`id`](EpubManifestEntryData::id)
    ///   of an [`EpubManifestEntryData`].
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
    pub fn by_id(&self, id: &str) -> Option<EpubSpineEntryData<'ebook, A>> {
        self.by_predicate(|data| data.id.as_deref() == Some(id))
    }

    /// Returns the [`EpubSpineEntryData`] that matches the given `idref` if present,
    /// otherwise [`None`].
    ///
    /// An [`idref`](EpubSpineEntryData::idref) is the [`id`](EpubManifestEntryData::id) of a
    /// [`EpubManifestEntryData`] referenced by a spine entry.
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
    pub fn by_idref(&self, idref: &str) -> Option<EpubSpineEntryData<'ebook, A>> {
        self.by_predicate(|data| data.idref == idref)
    }
}

#[allow(refining_impl_trait)]
impl<'ebook, A: Copy> Spine<'ebook> for EpubSpineData<'ebook, A> {
    fn page_direction(&self) -> PageDirection {
        self.data.page_direction
    }

    fn len(&self) -> usize {
        self.data.entries.len()
    }

    fn by_order(&self, order: usize) -> Option<EpubSpineEntryData<'ebook, A>> {
        self.data
            .entries
            .get(order)
            .map(|data| EpubSpineEntryData::new(self.provider, data))
    }

    fn entries(&self) -> EpubSpineIter<'ebook, A> {
        EpubSpineIter {
            provider: self.provider,
            iter: self.data.entries.iter(),
        }
    }
}

impl<A> Debug for EpubSpineData<'_, A> {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("EpubSpine")
            .field("data", self.data)
            .finish_non_exhaustive()
    }
}

impl<A> PartialEq for EpubSpineData<'_, A> {
    fn eq(&self, other: &Self) -> bool {
        self.data == other.data
    }
}

impl<'ebook, A: Copy> IntoIterator for &EpubSpineData<'ebook, A> {
    type Item = EpubSpineEntryData<'ebook, A>;
    type IntoIter = EpubSpineIter<'ebook, A>;

    fn into_iter(self) -> EpubSpineIter<'ebook, A> {
        self.entries()
    }
}

impl<'ebook, A: Copy> IntoIterator for EpubSpineData<'ebook, A> {
    type Item = EpubSpineEntryData<'ebook, A>;
    type IntoIter = EpubSpineIter<'ebook, A>;

    fn into_iter(self) -> EpubSpineIter<'ebook, A> {
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
pub struct EpubSpineIter<'ebook, A> {
    provider: EpubManifestEntryProvider<'ebook, A>,
    iter: SliceIter<'ebook, InternalEpubSpineEntry>,
}

impl<'ebook, A: Copy> Iterator for EpubSpineIter<'ebook, A> {
    type Item = EpubSpineEntryData<'ebook, A>;

    fn next(&mut self) -> Option<Self::Item> {
        self.iter
            .next()
            .map(|data| EpubSpineEntryData::new(self.provider, data))
    }
}

/// An entry contained within an [`EpubSpine`], encompassing associated metadata.
#[derive(Copy, Clone)]
pub struct EpubSpineEntryData<'ebook, A> {
    provider: EpubManifestEntryProvider<'ebook, A>,
    data: &'ebook InternalEpubSpineEntry,
}

impl<'ebook, A> EpubSpineEntryData<'ebook, A> {
    fn new(
        provider: EpubManifestEntryProvider<'ebook, A>,
        data: &'ebook InternalEpubSpineEntry,
    ) -> Self {
        Self { provider, data }
    }

    /// The unique id of a spine entry.
    pub fn id(&self) -> Option<&'ebook str> {
        self.data.id.as_deref()
    }

    /// The unique id reference to an [`EpubManifestEntryData`] in the
    /// [`EpubManifestData`](super::EpubManifestData).
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
    /// [`EpubReaderOptions::linear_behavior`](super::reader::EpubReaderOptions::linear_behavior).
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
impl<'ebook, A: Copy> SpineEntry<'ebook> for EpubSpineEntryData<'ebook, A> {
    fn order(&self) -> usize {
        self.data.order
    }

    fn manifest_entry(&self) -> Option<EpubManifestEntryData<'ebook, A>> {
        self.provider.by_id(self.idref())
    }
}

impl<A> Debug for EpubSpineEntryData<'_, A> {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("EpubSpineEntry")
            .field("data", self.data)
            .finish_non_exhaustive()
    }
}

impl<A: Copy> Ord for EpubSpineEntryData<'_, A> {
    fn cmp(&self, other: &Self) -> Ordering {
        self.order().cmp(&other.order())
    }
}

impl<A> Eq for EpubSpineEntryData<'_, A> {}

impl<A> PartialEq for EpubSpineEntryData<'_, A> {
    fn eq(&self, other: &Self) -> bool {
        self.data == other.data
    }
}

impl<A: Copy> PartialOrd<Self> for EpubSpineEntryData<'_, A> {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}
