//! EPUB-specific spine content.
//!
//! # See Also
//! - [`ebook::spine`](crate::ebook::spine) for the general spine module.

#[cfg(feature = "write")]
mod write;

use crate::ebook::element::{Attributes, AttributesData, Properties, PropertiesData};
use crate::ebook::epub::manifest::{EpubManifestContext, EpubManifestEntry};
use crate::ebook::epub::metadata::{EpubRefinements, EpubRefinementsData};
use crate::ebook::epub::package::EpubPackageMetaContext;
use crate::ebook::resource::Resource;
use crate::ebook::spine::{PageDirection, Spine, SpineEntry};
use crate::util::{self, Sealed};
use std::fmt::Debug;

#[cfg(feature = "write")]
pub use write::{DetachedEpubSpineEntry, EpubSpineEntryMut, EpubSpineIterMut, EpubSpineMut};

////////////////////////////////////////////////////////////////////////////////
// PRIVATE API
////////////////////////////////////////////////////////////////////////////////

#[derive(Debug, PartialEq)]
pub(super) struct EpubSpineData {
    pub(super) page_direction: PageDirection,
    pub(super) entries: Vec<EpubSpineEntryData>,
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

#[derive(Clone, Debug, Default, PartialEq)]
pub(super) struct EpubSpineEntryData {
    pub(super) id: Option<String>,
    pub(super) idref: String,
    pub(super) linear: bool,
    pub(super) properties: PropertiesData,
    pub(super) attributes: AttributesData,
    pub(super) refinements: EpubRefinementsData,
}

#[derive(Copy, Clone)]
pub(super) struct EpubSpineContext<'ebook> {
    manifest_ctx: EpubManifestContext<'ebook>,
    meta_ctx: EpubPackageMetaContext<'ebook>,
}

impl<'ebook> EpubSpineContext<'ebook> {
    pub(super) fn new(
        manifest_ctx: EpubManifestContext<'ebook>,
        meta_ctx: EpubPackageMetaContext<'ebook>,
    ) -> Self {
        Self {
            manifest_ctx,
            meta_ctx,
        }
    }

    pub(super) fn create_entry(
        self,
        data: &'ebook EpubSpineEntryData,
        index: usize,
    ) -> EpubSpineEntry<'ebook> {
        EpubSpineEntry {
            ctx: self,
            data,
            index,
        }
    }
}

////////////////////////////////////////////////////////////////////////////////
// PUBLIC API
////////////////////////////////////////////////////////////////////////////////

/// The EPUB spine, accessible via [`Epub::spine`](super::Epub::spine).
/// See [`Spine`] for more details.
///
/// # See Also
/// - [`EpubSpineMut`] for a mutable view.
#[derive(Copy, Clone)]
pub struct EpubSpine<'ebook> {
    ctx: EpubSpineContext<'ebook>,
    spine: &'ebook EpubSpineData,
}

impl<'ebook> EpubSpine<'ebook> {
    pub(super) fn new(
        manifest_ctx: EpubManifestContext<'ebook>,
        meta_ctx: EpubPackageMetaContext<'ebook>,
        spine: &'ebook EpubSpineData,
    ) -> Self {
        Self {
            ctx: EpubSpineContext::new(manifest_ctx, meta_ctx),
            spine,
        }
    }

    /// Returns the [`EpubSpineEntry`] matching the given `id`, or [`None`] if not found.
    ///
    /// # See Also
    /// - [`Self::by_idref`] to retrieve spine entries by the [`id`](EpubManifestEntry::id)
    ///   of an [`EpubManifestEntry`].
    ///
    /// # Examples
    /// - Retrieving a spine entry by its ID:
    /// ```
    /// # use rbook::Epub;
    /// # fn main() -> rbook::ebook::errors::EbookResult<()> {
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
        self.spine
            .entries
            .iter()
            .enumerate()
            .find(|(_, data)| data.id.as_deref() == Some(id))
            .map(|(i, data)| self.ctx.create_entry(data, i))
    }

    /// Returns an iterator over all entries matching the given `idref`.
    ///
    /// An [`idref`](EpubSpineEntry::idref) is the [`id`](EpubManifestEntry::id) of a
    /// [`EpubManifestEntry`] referenced by a spine entry.
    ///
    /// Albeit uncommon, more than one spine entry can reference the same manifest entry.
    ///
    /// # Examples
    /// - Retrieving a spine entry by its idref:
    /// ```
    /// # use rbook::Epub;
    /// # fn main() -> rbook::ebook::errors::EbookResult<()> {
    /// let epub = Epub::open("tests/ebooks/example_epub")?;
    ///
    /// let spine_entry = epub.spine().by_idref("c1").next().unwrap();
    /// assert_eq!("c1", spine_entry.idref());
    /// assert_eq!(2, spine_entry.order());
    ///
    /// // Attempt to retrieve a non-existent entry
    /// assert_eq!(None, epub.spine().by_idref("c999").next());
    /// # Ok(())
    /// # }
    /// ```
    pub fn by_idref(
        &self,
        idref: &'ebook str,
    ) -> impl Iterator<Item = EpubSpineEntry<'ebook>> + 'ebook {
        let ctx = self.ctx;

        self.spine
            .entries
            .iter()
            .enumerate()
            .filter(move |(_, data)| data.idref == idref)
            .map(move |(i, data)| ctx.create_entry(data, i))
    }

    /// The [`PageDirection`] hint, indicating how readable content flows.
    #[doc = util::inherent_doc!(Spine, page_direction)]
    pub fn page_direction(&self) -> PageDirection {
        self.spine.page_direction
    }

    /// The total number of [entries](EpubSpineEntry) that makes up the spine.
    #[doc = util::inherent_doc!(Spine, len)]
    pub fn len(&self) -> usize {
        self.spine.entries.len()
    }

    /// Returns `true` if there are no [entries](EpubSpineEntry).
    #[doc = util::inherent_doc!(Spine, is_empty)]
    pub fn is_empty(&self) -> bool {
        Spine::is_empty(self)
    }

    /// Returns the associated [`EpubSpineEntry`] if the given `index` is less than
    /// [`Self::len`], otherwise [`None`].
    #[doc = util::inherent_doc!(Spine, get)]
    pub fn get(&self, index: usize) -> Option<EpubSpineEntry<'ebook>> {
        self.spine
            .entries
            .get(index)
            .map(|data| self.ctx.create_entry(data, index))
    }

    /// Returns an iterator over all [entries](EpubSpineEntry) within
    /// the spine in canonical order.
    #[doc = util::inherent_doc!(Spine, iter)]
    pub fn iter(&self) -> EpubSpineIter<'ebook> {
        EpubSpineIter {
            ctx: self.ctx,
            iter: self.spine.entries.iter().enumerate(),
        }
    }
}

impl Sealed for EpubSpine<'_> {}

#[allow(refining_impl_trait)]
impl<'ebook> Spine<'ebook> for EpubSpine<'ebook> {
    fn page_direction(&self) -> PageDirection {
        self.page_direction()
    }

    fn len(&self) -> usize {
        self.len()
    }

    fn get(&self, order: usize) -> Option<EpubSpineEntry<'ebook>> {
        self.get(order)
    }

    fn iter(&self) -> EpubSpineIter<'ebook> {
        self.iter()
    }
}

impl Debug for EpubSpine<'_> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("EpubSpine")
            .field("data", self.spine)
            .finish_non_exhaustive()
    }
}

impl PartialEq for EpubSpine<'_> {
    fn eq(&self, other: &Self) -> bool {
        self.spine == other.spine
    }
}

impl<'ebook> IntoIterator for &EpubSpine<'ebook> {
    type Item = EpubSpineEntry<'ebook>;
    type IntoIter = EpubSpineIter<'ebook>;

    fn into_iter(self) -> EpubSpineIter<'ebook> {
        self.iter()
    }
}

impl<'ebook> IntoIterator for EpubSpine<'ebook> {
    type Item = EpubSpineEntry<'ebook>;
    type IntoIter = EpubSpineIter<'ebook>;

    fn into_iter(self) -> EpubSpineIter<'ebook> {
        self.iter()
    }
}

/// An iterator over all the [entries](EpubSpineEntry) contained within [`EpubSpine`].
///
/// # See Also
/// - [`EpubSpine::iter`] to create an instance of this struct.
///
/// # Examples
/// - Iterating over all manifest entries:
/// ```
/// # use rbook::Epub;
/// # fn main() -> rbook::ebook::errors::EbookResult<()> {
/// let epub = Epub::open("tests/ebooks/example_epub")?;
///
/// for entry in epub.spine() {
///     // process entry //
/// }
/// # Ok(())
/// # }
/// ```
pub struct EpubSpineIter<'ebook> {
    ctx: EpubSpineContext<'ebook>,
    iter: std::iter::Enumerate<std::slice::Iter<'ebook, EpubSpineEntryData>>,
}

impl<'ebook> Iterator for EpubSpineIter<'ebook> {
    type Item = EpubSpineEntry<'ebook>;

    fn next(&mut self) -> Option<Self::Item> {
        self.iter
            .next()
            .map(|(i, data)| self.ctx.create_entry(data, i))
    }
}

/// An entry contained within an [`EpubSpine`], encompassing associated metadata.
///
/// # See Also
/// - [`EpubSpineEntryMut`] for a mutable view.
#[derive(Copy, Clone)]
pub struct EpubSpineEntry<'ebook> {
    ctx: EpubSpineContext<'ebook>,
    data: &'ebook EpubSpineEntryData,
    index: usize,
}

impl<'ebook> EpubSpineEntry<'ebook> {
    /// The unique ID of a spine entry.
    pub fn id(&self) -> Option<&'ebook str> {
        self.data.id.as_deref()
    }

    /// The unique ID reference to an [`EpubManifestEntry`] in the
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
    pub fn properties(&self) -> &'ebook Properties {
        &self.data.properties
    }

    /// All additional XML [`Attributes`].
    ///
    /// # Omitted Attributes
    /// The following attributes will not be found within the returned collection:
    /// - [`id`](Self::id)
    /// - [`idref`](Self::idref)
    /// - [`linear`](Self::is_linear)
    /// - [`properties`](Self::properties)
    pub fn attributes(&self) -> &'ebook Attributes {
        &self.data.attributes
    }

    /// Complementary refinement metadata entries.
    pub fn refinements(&self) -> EpubRefinements<'ebook> {
        self.ctx
            .meta_ctx
            .create_refinements(self.id(), &self.data.refinements)
    }

    /// The canonical order of an entry (`0 = first entry`).
    #[doc = util::inherent_doc!(SpineEntry, order)]
    pub fn order(&self) -> usize {
        self.index
    }

    /// The [`EpubManifestEntry`] associated with a [`EpubSpineEntry`].
    #[doc = util::inherent_doc!(SpineEntry, manifest_entry)]
    pub fn manifest_entry(&self) -> Option<EpubManifestEntry<'ebook>> {
        self.ctx.manifest_ctx.by_id(self.idref())
    }

    /// The textual [`Resource`] intended for end-user reading an entry points to.
    #[doc = util::inherent_doc!(SpineEntry, resource)]
    pub fn resource(&self) -> Option<Resource<'ebook>> {
        SpineEntry::resource(self)
    }
}

impl Sealed for EpubSpineEntry<'_> {}

#[allow(refining_impl_trait)]
impl<'ebook> SpineEntry<'ebook> for EpubSpineEntry<'ebook> {
    fn order(&self) -> usize {
        self.order()
    }

    fn manifest_entry(&self) -> Option<EpubManifestEntry<'ebook>> {
        self.manifest_entry()
    }
}

impl Debug for EpubSpineEntry<'_> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("EpubSpineEntry")
            .field("data", self.data)
            .finish_non_exhaustive()
    }
}

impl PartialEq for EpubSpineEntry<'_> {
    fn eq(&self, other: &Self) -> bool {
        self.data == other.data
    }
}
