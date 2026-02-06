//! EPUB-specific table-of-contents content.
//!
//! # See Also
//! - [`ebook::toc`](crate::ebook::toc) for the general ToC module.

#[cfg(feature = "write")]
mod write;

use crate::ebook::element::{Attributes, AttributesData, Href};
use crate::ebook::epub::EpubVersion;
use crate::ebook::epub::manifest::{EpubManifestContext, EpubManifestEntry};
use crate::ebook::resource::Resource;
use crate::ebook::toc::{Toc, TocEntry, TocEntryKind};
use crate::util::{self, Sealed};
use indexmap::IndexMap;
use indexmap::map::Iter as HashMapIter;
use std::fmt::Debug;
use std::slice::Iter as SliceIter;

#[cfg(feature = "write")]
pub use write::{
    DetachedEpubTocEntry, EpubTocEntryIterMut, EpubTocEntryMut, EpubTocIterMut, EpubTocMut,
};

////////////////////////////////////////////////////////////////////////////////
// PRIVATE API
////////////////////////////////////////////////////////////////////////////////

pub(super) type TocGroups = IndexMap<EpubTocKey, EpubTocEntryData>;

#[derive(Debug, Hash, PartialEq, Eq)]
pub(super) struct EpubTocKey {
    pub(super) kind: String,
    pub(super) version: EpubVersion,
}

impl EpubTocKey {
    pub(super) fn new(kind: String, version: EpubVersion) -> Self {
        Self { kind, version }
    }

    // Only used when the `write` feature is toggled currently.
    #[cfg(feature = "write")]
    pub(super) fn kind(&self) -> TocEntryKind<'_> {
        TocEntryKind::from(&self.kind)
    }
}

impl indexmap::Equivalent<EpubTocKey> for (&str, EpubVersion) {
    fn equivalent(&self, key: &EpubTocKey) -> bool {
        self.0 == key.kind && self.1 == key.version
    }
}

#[derive(Debug, PartialEq)]
pub(super) struct EpubTocData {
    pub(super) preferred_version: EpubVersion,
    pub(super) entries: TocGroups,
}

impl EpubTocData {
    pub(super) fn new(entries: TocGroups) -> Self {
        Self {
            // the preferred version here is a placeholder
            preferred_version: EpubVersion::EPUB3,
            entries,
        }
    }

    pub(super) fn empty() -> Self {
        Self::new(IndexMap::new())
    }

    pub(super) fn extend(&mut self, data: Self) {
        self.entries.extend(data.entries);
    }

    pub(super) fn get_preferred_version(&self, kind: TocEntryKind) -> EpubVersion {
        match kind {
            TocEntryKind::Landmarks | TocEntryKind::PageList | TocEntryKind::Toc => {
                self.preferred_version
            }
            _ => EpubVersion::EPUB3,
        }
    }
}

#[derive(Clone, Debug, Default, PartialEq)]
pub(super) struct EpubTocEntryData {
    pub(super) id: Option<String>,
    pub(super) label: String,
    pub(super) kind: Option<String>,
    pub(super) href: Option<String>,
    pub(super) href_raw: Option<String>,
    pub(super) attributes: AttributesData,
    pub(super) children: Vec<EpubTocEntryData>,
}

#[derive(Copy, Clone)]
pub(super) struct EpubTocContext<'ebook> {
    manifest_ctx: EpubManifestContext<'ebook>,
}

impl<'ebook> EpubTocContext<'ebook> {
    pub(super) fn new(manifest_ctx: EpubManifestContext<'ebook>) -> Self {
        Self { manifest_ctx }
    }

    pub(super) fn create_root(
        self,
        version: EpubVersion,
        data: &'ebook EpubTocEntryData,
    ) -> EpubTocEntry<'ebook> {
        self.create_entry(version, data, 0)
    }

    pub(super) fn create_entry(
        self,
        version: EpubVersion,
        data: &'ebook EpubTocEntryData,
        depth: usize,
    ) -> EpubTocEntry<'ebook> {
        EpubTocEntry {
            ctx: self,
            version,
            data,
            depth,
        }
    }
}

////////////////////////////////////////////////////////////////////////////////
// PUBLIC API
////////////////////////////////////////////////////////////////////////////////

/// The EPUB table of contents, accessible via [`Epub::toc`](super::Epub::toc).
/// See [`Toc`] for additional details.
///
/// For EPUB 3 ebooks backwards compatible with EPUB2,
/// the preferred toc format is configurable via
/// [`EpubOpenOptions::preferred_toc`](super::EpubOpenOptions::preferred_toc).
/// Methods regarding preferred format:
/// - [`EpubToc::contents`] (`toc`)
/// - [`EpubToc::page_list`] (`page-list`)
/// - [`EpubToc::landmarks`] (`landmarks/guide`)
/// - [`EpubToc::by_kind_version`]
///
/// # See Also
/// - [`EpubTocMut`] for a mutable view.
#[derive(Copy, Clone)]
pub struct EpubToc<'ebook> {
    ctx: EpubTocContext<'ebook>,
    toc: &'ebook EpubTocData,
}

impl<'ebook> EpubToc<'ebook> {
    pub(super) fn new(manifest_ctx: EpubManifestContext<'ebook>, toc: &'ebook EpubTocData) -> Self {
        Self {
            ctx: EpubTocContext::new(manifest_ctx),
            toc,
        }
    }

    fn by_toc_key(&self, kind: &str, version: EpubVersion) -> Option<EpubTocEntry<'ebook>> {
        self.toc
            .entries
            .get(&(kind, version))
            .map(|data| self.ctx.create_root(version, data))
    }

    /// Returns the preferred **page list** root entry.
    ///
    /// This maps to:
    /// - **EPUB 3:** XHTML `nav` where `epub:type` is `page-list`.
    /// - **EPUB 2:** NCX `pageList`.
    ///
    /// The default preferred format (EPUB 3) is configurable via
    /// [`EpubOpenOptions::preferred_toc`](super::EpubOpenOptions::preferred_toc).
    ///
    /// # Note
    /// This method is equivalent to calling [`EpubToc::by_kind`]
    /// with [`TocEntryKind::PageList`] as the argument.
    ///
    /// # See Also
    /// - **[`Self::by_kind`] to see selection and fallback behavior, which this method uses.*
    pub fn page_list(&self) -> Option<EpubTocEntry<'ebook>> {
        self.by_kind(TocEntryKind::PageList)
    }

    /// Returns the preferred **guide/landmarks** root entry.
    ///
    /// This maps to:
    /// - **EPUB 3:** XHTML `nav` where `epub:type` is `landmarks`.
    /// - **EPUB 2:** OPF `guide`.
    ///
    /// The default preferred format (EPUB 3) is configurable via
    /// [`EpubOpenOptions::preferred_toc`](super::EpubOpenOptions::preferred_toc).
    ///
    /// # Note
    /// This method is equivalent to calling [`EpubToc::by_kind`]
    /// with [`TocEntryKind::Landmarks`] as the argument.
    ///
    /// # See Also
    /// - **[`Self::by_kind`] to see selection and fallback behavior, which this method uses.**
    pub fn landmarks(&self) -> Option<EpubTocEntry<'ebook>> {
        self.by_kind(TocEntryKind::Landmarks)
    }

    /// Returns the root entry associated with the given `kind` and `version`, if present.
    ///
    /// Example Mappings:
    /// - [`TocEntryKind::PageList`] + [`EpubVersion::Epub2`] = Legacy EPUB 2 NCX page list.
    /// - [`TocEntryKind::PageList`] + [`EpubVersion::Epub3`] = EPUB 3 XHTML page list.
    ///
    /// # See Also
    /// - [`EpubOpenOptions`](super::EpubOpenOptions) to see conditional ToC-related parsing options.
    /// - [`Toc::by_kind`] to retrieve the toc root for a given [`TocEntryKind`].
    pub fn by_kind_version(
        &self,
        kind: impl Into<TocEntryKind<'ebook>>,
        version: EpubVersion,
    ) -> Option<EpubTocEntry<'ebook>> {
        let kind = kind.into();
        // "Normalize" epub version as the contained value
        // may be different (e.g., "3.1", "3.2")
        // Version must be `2.0` or `3.0`
        self.by_toc_key(kind.as_str(), version.as_major())
    }

    /// Returns the preferred **table of contents** root entry.
    ///
    /// This maps to:
    /// - **EPUB 3:** XHTML `nav` where `epub:type` is `toc`.
    /// - **EPUB 2:** NCX `navMap`.
    ///
    /// The default preferred variant (EPUB 3) is configurable via
    /// [`EpubOpenOptions::preferred_toc`](super::EpubOpenOptions::preferred_toc).
    ///
    /// # Note
    /// This method is equivalent to calling [`EpubToc::by_kind`]
    /// with [`TocEntryKind::Toc`] as the argument.
    ///
    #[doc = util::inherent_doc!(Toc, contents)]
    /// # See Also
    /// - **[`Self::by_kind`] to see selection and fallback behavior, which this method uses.**
    /// - [`Self::by_kind_version`] to retrieve a specific variant (e.g. EPUB 2 NCX).
    pub fn contents(&self) -> Option<EpubTocEntry<'ebook>> {
        self.by_kind(TocEntryKind::Toc)
    }

    // NOTE: This doc is nearly identical to EpubTocMut::by_kind_mut
    /// Returns the root entry associated with the given `kind` and preferred variant.
    ///
    /// The specific variant returned (EPUB 3 or EPUB 2 NCX) depends on:
    /// 1. Which variants an [`Epub`](super::Epub) contains when opened, as dictated by
    ///    [`EpubOpenOptions`](super::EpubOpenOptions).
    /// 2. Preferences such as
    ///    [`EpubOpenOptions::preferred_toc`](crate::epub::EpubOpenOptions::preferred_toc).
    ///
    ///    If an [`Epub`](super::Epub) was created in-memory via
    ///    [`new`](super::Epub::new) or [`builder`](super::Epub::builder),
    ///    all preferences are set to [`EpubVersion::EPUB3`].
    ///
    /// If the preferred variant is not present, the other variant
    /// (EPUB 3 or EPUB 2 NCX) is returned instead.
    /// If neither variant exists, [`None`] is returned.
    ///
    #[doc = util::inherent_doc!(Toc, by_kind)]
    /// # See Also
    /// - [`Self::by_kind_version`]
    ///   to retrieve a specific root entry without any fallback behavior.
    pub fn by_kind(&self, kind: impl Into<TocEntryKind<'ebook>>) -> Option<EpubTocEntry<'ebook>> {
        let kind = kind.into();
        let preferred_version = self.toc.get_preferred_version(kind);
        let attempts = std::iter::once(preferred_version)
            // If preferred version isn't available, try all standard versions
            // Note: If the preferred version is EPUB2/3, it is also included in `VERSIONS`.
            //       Despite the redundancy, the cost is negligible.
            .chain(EpubVersion::VERSIONS);

        // Mutable key for quick lookup & modification
        for version in attempts {
            if let Some(root) = self.by_toc_key(kind.as_str(), version) {
                return Some(root);
            }
        }
        None
    }

    /// Returns an iterator over all **root** [entries](EpubTocEntry).
    #[doc = util::inherent_doc!(Toc, iter)]
    pub fn iter(&self) -> EpubTocIter<'ebook> {
        self.into_iter()
    }
}

impl Sealed for EpubToc<'_> {}

#[allow(refining_impl_trait)]
impl<'ebook> Toc<'ebook> for EpubToc<'ebook> {
    fn contents(&self) -> Option<EpubTocEntry<'ebook>> {
        self.contents()
    }

    fn by_kind(&self, kind: impl Into<TocEntryKind<'ebook>>) -> Option<EpubTocEntry<'ebook>> {
        self.by_kind(kind)
    }

    fn iter(&self) -> EpubTocIter<'ebook> {
        self.iter()
    }
}

impl Debug for EpubToc<'_> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("EpubToc")
            .field("data", self.toc)
            .finish_non_exhaustive()
    }
}

impl PartialEq for EpubToc<'_> {
    fn eq(&self, other: &Self) -> bool {
        self.toc == other.toc
    }
}

impl<'ebook> IntoIterator for &EpubToc<'ebook> {
    type Item = EpubTocEntry<'ebook>;
    type IntoIter = EpubTocIter<'ebook>;

    fn into_iter(self) -> Self::IntoIter {
        EpubTocIter {
            ctx: self.ctx,
            iter: self.toc.entries.iter(),
        }
    }
}

impl<'ebook> IntoIterator for EpubToc<'ebook> {
    type Item = EpubTocEntry<'ebook>;
    type IntoIter = EpubTocIter<'ebook>;

    fn into_iter(self) -> Self::IntoIter {
        (&self).into_iter()
    }
}

/// An iterator over all ToC roots contained within [`EpubToc`].
///
/// # See Also
/// - [`EpubToc::iter`] to create an instance of this struct.
///
/// # Examples
/// - Iterating over all root toc kinds:
/// ```
/// # use rbook::Epub;
/// # fn main() -> rbook::ebook::errors::EbookResult<()> {
/// let epub = Epub::open("tests/ebooks/example_epub")?;
///
/// for root in epub.toc() {
///     let kind = root.kind();
///
///     // process toc root //
/// }
/// # Ok(())
/// # }
/// ```
pub struct EpubTocIter<'ebook> {
    ctx: EpubTocContext<'ebook>,
    iter: HashMapIter<'ebook, EpubTocKey, EpubTocEntryData>,
}

impl<'ebook> Iterator for EpubTocIter<'ebook> {
    type Item = EpubTocEntry<'ebook>;

    fn next(&mut self) -> Option<Self::Item> {
        self.iter
            .next()
            .map(move |(key, data)| self.ctx.create_root(key.version, data))
    }
}

/// A [`TocEntry`] contained within an [`EpubToc`], encompassing associated metadata.
///
/// # See Also
/// - [`EpubTocEntryMut`] for a mutable view.
/// - [`Self::attributes`] to retrieve the legacy EPUB 2 NCX `playOrder` attribute.
///
/// # Examples
/// - Observing the root entry of landmarks:
/// ```
/// # use rbook::Epub;
/// # use rbook::ebook::toc::TocEntryKind;
/// # use rbook::epub::metadata::EpubVersion;
/// # fn main() -> rbook::ebook::errors::EbookResult<()> {
/// let epub = Epub::open("tests/ebooks/example_epub")?;
/// // Retrieving the landmarks
/// let landmarks_root = epub.toc().landmarks().unwrap();
///
/// // The version indicates whether the landmarks derive from
/// // the EPUB 2 NCX document or EPUB 3 XHTML nav document.
/// assert_eq!(EpubVersion::EPUB3, landmarks_root.version());
///
/// // The kind indicates the root contains entries specifying
/// // landmarks (i.e., points of interest).
/// assert_eq!(TocEntryKind::Landmarks, landmarks_root.kind());
///
/// // There are 3 direct entries that make up the landmarks.
/// assert_eq!(3, landmarks_root.len());
/// # Ok(())
/// # }
/// ```
#[derive(Copy, Clone)]
pub struct EpubTocEntry<'ebook> {
    ctx: EpubTocContext<'ebook>,
    version: EpubVersion,
    data: &'ebook EpubTocEntryData,
    depth: usize,
}

impl<'ebook> EpubTocEntry<'ebook> {
    /// Version information to indicate whether an entry derives from
    /// the EPUB 2 NCX document or the EPUB 3 XHTML nav document.
    pub fn version(&self) -> EpubVersion {
        self.version
    }

    /// The unique ID of a toc entry.
    ///
    /// # Note
    /// For EPUB 3, this field is derived from the anchor (`a`) element.
    pub fn id(&self) -> Option<&'ebook str> {
        self.data.id.as_deref()
    }

    /// The raw `epub:type`/`type` value.
    ///
    /// This method is a lower-level call than [`Self::kind`],
    /// which allows inspecting the original value before normalization by [`TocEntryKind`].
    ///
    /// # Examples
    /// - Retrieving the raw and normalized value:
    /// ```
    /// # use rbook::Epub;
    /// # use rbook::ebook::toc::TocEntryKind;
    /// # use rbook::epub::metadata::EpubVersion;
    /// # fn main() -> rbook::ebook::errors::EbookResult<()> {
    /// let epub = Epub::options()
    ///     .retain_variants(true)
    ///     .open("tests/ebooks/example_epub")?;
    ///
    /// // Retrieving the EPUB 2 guide
    /// let guide = epub.toc().by_kind_version(TocEntryKind::Landmarks, EpubVersion::EPUB2).unwrap();
    ///
    /// // Retrieving the 3rd entry
    /// let entry = guide.get(2).unwrap();
    ///
    /// // Original:
    /// assert_eq!(Some("text"), entry.kind_raw());
    /// // Normalized:
    /// assert_eq!(TocEntryKind::BodyMatter, entry.kind());
    /// assert_eq!("bodymatter", entry.kind().as_str());
    /// # Ok(())
    /// # }
    /// ```
    pub fn kind_raw(&self) -> Option<&'ebook str> {
        self.data.kind.as_deref()
    }

    /// The resolved absolute percent-encoded `href`,
    /// indicating the location a toc entry points to.
    ///
    /// Returns [`None`] if the entry neither has an `href` (EPUB 3)
    /// nor a `src` (EPUB 2) attribute.
    ///
    /// Example of a resolved href:
    /// ```text
    /// /EPUB/OEBPS/chapters/c1.xhtml#part-1
    /// ```
    ///
    /// The href is resolved by calculating the location of [`Self::href_raw`]
    /// relative to the directory containing the associated toc `.ncx`/`.xhtml` file.
    ///
    /// # Note
    /// - The resolved href is pre-calculated during parsing.
    /// - The href is corrected if [`EpubOpenOptions::strict`](super::EpubOpenOptions::strict)
    ///   is disabled.
    ///   For example, if the source EPUB contained unencoded characters (e.g., spaces),
    ///   they are automatically encoded.
    ///
    /// # See Also
    /// - [`Href::path`] to retrieve the href value without the query and fragment.
    /// - [`Self::resource`] as the primary means for retrieving ebook content.
    pub fn href(&self) -> Option<Href<'ebook>> {
        self.data.href.as_deref().map(Href::new)
    }

    /// The raw (relative) `href`,
    /// indicating the location a toc entry points to.
    ///
    /// Returns [`None`] if the entry neither has an `href` (EPUB 3)
    /// nor a `src` (EPUB 2) attribute.
    ///
    /// Example of a raw (relative) href:
    /// ```text
    /// ../../../c1.xhtml#part-1
    /// ```
    ///
    /// # Percent-Encoding
    /// If [`EpubOpenOptions::strict`](super::EpubOpenOptions::strict) is disabled
    /// and the EPUB is malformed (e.g., unencoded hrefs),
    /// the returned [`Href`] will reflect that unencoded state.
    ///
    /// # Note
    /// - [`Self::href`] is recommended over this method.
    ///   Providing the raw href to a method such as
    ///   [`Ebook::read_resource_bytes`](crate::Ebook::read_resource_bytes) **may fail**.
    /// - In EPUB modification workflows, if the href of a manifest item is changed
    ///   via [`EpubManifestEntryMut::set_href`](super::manifest::EpubManifestEntryMut::set_href),
    ///   [`Self::href`] can return [`Some`] while this method returns [`None`].
    ///
    /// # See Also
    /// - [`Epub`](super::Epub) documentation of `copy_resource` for normalization details.
    /// - [`Href::path`] to retrieve the href value without the query and fragment.
    pub fn href_raw(&self) -> Option<Href<'ebook>> {
        self.data.href_raw.as_deref().map(Href::new)
    }

    /// All additional XML [`Attributes`].
    ///
    /// Attributes come from one of the following navigation elements:
    /// - **EPUB 3** Navigation Document:
    ///   - `nav` ([root](Self::is_root))
    ///   - `li`
    /// - **EPUB 2** NCX:
    ///   - `navMap` ([root](Self::is_root))
    ///   - `navPoint`
    ///   - `pageList` ([root](Self::is_root))
    ///   - `pageTarget`
    ///
    /// # Omitted Attributes
    /// The following attributes will **not** be found within the returned collection:
    /// - [`id`](Self::id)
    /// - [`href`](Self::href)
    /// - [`epub:type`](Self::kind_raw)
    /// - [`type`](Self::kind_raw) (EPUB 2; legacy)
    /// - [`src`](Self::href) (EPUB 2; legacy)
    ///
    /// # Legacy `playOrder` Attribute
    /// The legacy NCX `playOrder` attribute is accessible from this method, if present.
    /// However, it is ***not*** guaranteed to represent accurate play order in EPUBs.
    /// For a reliable order, it is recommended to call
    /// [`Self::flatten`] paired with [`Iterator::enumerate`]:
    /// ```
    /// # use rbook::Epub;
    /// # fn main() -> rbook::ebook::errors::EbookResult<()> {
    /// let epub = Epub::open("tests/ebooks/example_epub")?;
    /// // Retrieving the main toc contents
    /// let main_contents_root = epub.toc().contents().unwrap();
    ///
    /// for (order, entry) in main_contents_root.flatten().enumerate() {
    ///     let label = entry.label();
    ///
    ///     println!("{order}: {label}");
    /// }
    /// # Ok(())
    /// # }
    /// ```
    pub fn attributes(&self) -> &'ebook Attributes {
        &self.data.attributes
    }

    /// The depth of an entry relative to the root ([`0 = root`](Self::is_root)).
    #[doc = util::inherent_doc!(TocEntry, depth)]
    pub fn depth(&self) -> usize {
        self.depth
    }

    /// The human-readable label.
    #[doc = util::inherent_doc!(TocEntry, label)]
    pub fn label(&self) -> &'ebook str {
        &self.data.label
    }

    /// The semantic kind of content associated with an entry.
    #[doc = util::inherent_doc!(TocEntry, kind)]
    pub fn kind(&self) -> TocEntryKind<'ebook> {
        self.data
            .kind
            .as_deref()
            .map(TocEntryKind::from)
            .unwrap_or_default()
    }

    /// The [`EpubManifestEntry`] associated with an [`EpubTocEntry`].
    #[doc = util::inherent_doc!(TocEntry, manifest_entry)]
    pub fn manifest_entry(&self) -> Option<EpubManifestEntry<'ebook>> {
        self.href()
            .and_then(|href| self.ctx.manifest_ctx.by_href(href.path().as_str()))
    }

    /// The [`Resource`] intended to navigate to from an entry.
    #[doc = util::inherent_doc!(TocEntry, resource)]
    pub fn resource(&self) -> Option<Resource<'ebook>> {
        TocEntry::resource(self)
    }

    /// Returns the associated direct child [`EpubTocEntry`] if the given `index` is less than
    /// [`Self::len`], otherwise [`None`].
    #[doc = util::inherent_doc!(TocEntry, get)]
    pub fn get(&self, index: usize) -> Option<EpubTocEntry<'ebook>> {
        self.data
            .children
            .get(index)
            .map(|data| self.ctx.create_entry(self.version, data, self.depth + 1))
    }

    /// Returns an iterator over direct child entries
    /// (whose [`depth`](EpubTocEntry::depth) is one greater than the parent).
    #[doc = util::inherent_doc!(TocEntry, iter)]
    pub fn iter(&self) -> EpubTocEntryIter<'ebook> {
        EpubTocEntryIter {
            ctx: self.ctx,
            version: self.version,
            iter: self.data.children.iter(),
            next_depth: self.depth + 1,
        }
    }

    /// Returns a recursive iterator over **all** children.
    #[doc = util::inherent_doc!(TocEntry, flatten)]
    pub fn flatten(&self) -> impl Iterator<Item = EpubTocEntry<'ebook>> + 'ebook {
        struct FlatEpubTocEntryIterator<'ebook> {
            ctx: EpubTocContext<'ebook>,
            version: EpubVersion,
            stack: Vec<(usize, &'ebook EpubTocEntryData)>, // Vec<(depth, data)>
        }

        impl<'ebook> Iterator for FlatEpubTocEntryIterator<'ebook> {
            type Item = EpubTocEntry<'ebook>;

            fn next(&mut self) -> Option<Self::Item> {
                let (depth, data) = self.stack.pop()?;

                // Push children in reverse order to maintain DFS order
                self.stack
                    .extend(data.children.iter().rev().map(|data| (depth + 1, data)));
                Some(self.ctx.create_entry(self.version, data, depth))
            }
        }

        FlatEpubTocEntryIterator {
            ctx: self.ctx,
            version: self.version,
            stack: self
                .data
                .children
                .iter()
                .rev()
                .map(|data| (self.depth + 1, data))
                .collect(),
        }
    }

    /// The total number of direct [`children`](Self::iter) a toc entry has.
    #[doc = util::inherent_doc!(TocEntry, len)]
    pub fn len(&self) -> usize {
        self.data.children.len()
    }

    /// Returns `true` if there are no children.
    #[doc = util::inherent_doc!(TocEntry, is_empty)]
    pub fn is_empty(&self) -> bool {
        TocEntry::is_empty(self)
    }

    /// Returns `true` if the depth of a toc entry is `0`, indicating the root.
    #[doc = util::inherent_doc!(TocEntry, is_root)]
    pub fn is_root(&self) -> bool {
        TocEntry::is_root(self)
    }

    /// Calculates and returns the **maximum** depth relative to an entry.
    /// In other words, how many levels deep is the most-nested child?
    #[doc = util::inherent_doc!(TocEntry, max_depth)]
    pub fn max_depth(&self) -> usize {
        TocEntry::max_depth(self)
    }

    /// Calculates and returns the **total** number of all (direct and nested)
    /// children relative to an entry.
    #[doc = util::inherent_doc!(TocEntry, total_len)]
    pub fn total_len(&self) -> usize {
        TocEntry::total_len(self)
    }
}

impl Sealed for EpubTocEntry<'_> {}

#[allow(refining_impl_trait)]
impl<'ebook> TocEntry<'ebook> for EpubTocEntry<'ebook> {
    fn depth(&self) -> usize {
        self.depth()
    }

    fn label(&self) -> &'ebook str {
        self.label()
    }

    fn kind(&self) -> TocEntryKind<'ebook> {
        self.kind()
    }

    fn manifest_entry(&self) -> Option<EpubManifestEntry<'ebook>> {
        self.manifest_entry()
    }

    fn get(&self, index: usize) -> Option<EpubTocEntry<'ebook>> {
        self.get(index)
    }

    fn iter(&self) -> EpubTocEntryIter<'ebook> {
        self.iter()
    }

    fn flatten(&self) -> impl Iterator<Item = EpubTocEntry<'ebook>> + 'ebook {
        self.flatten()
    }

    fn len(&self) -> usize {
        self.len()
    }
}

impl Debug for EpubTocEntry<'_> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("EpubTocEntry")
            .field("data", self.data)
            .finish_non_exhaustive()
    }
}

impl<'ebook> IntoIterator for &EpubTocEntry<'ebook> {
    type Item = EpubTocEntry<'ebook>;
    type IntoIter = EpubTocEntryIter<'ebook>;

    fn into_iter(self) -> Self::IntoIter {
        self.iter()
    }
}

impl<'ebook> IntoIterator for EpubTocEntry<'ebook> {
    type Item = Self;
    type IntoIter = EpubTocEntryIter<'ebook>;

    fn into_iter(self) -> Self::IntoIter {
        (&self).into_iter()
    }
}

impl PartialEq<Self> for EpubTocEntry<'_> {
    fn eq(&self, other: &Self) -> bool {
        self.data == other.data
    }
}

/// An iterator over the direct [`children`](EpubTocEntry) contained within [`EpubTocEntry`].
///
/// # See Also
/// - [`EpubTocEntry::iter`] to create an [`EpubTocEntry`] instance.
/// - [`EpubTocEntry::flatten`] to iterate over all children in flattened form.
///
/// # Examples
/// - Iterating over direct children:
/// ```
/// # use rbook::Epub;
/// # fn main() -> rbook::ebook::errors::EbookResult<()> {
/// let epub = Epub::open("tests/ebooks/example_epub")?;
/// let nav_root = epub.toc().contents().unwrap();
///
/// for child in nav_root {
///     // process direct child //
/// }
/// # Ok(())
/// # }
/// ```
pub struct EpubTocEntryIter<'ebook> {
    ctx: EpubTocContext<'ebook>,
    version: EpubVersion,
    iter: SliceIter<'ebook, EpubTocEntryData>,
    next_depth: usize,
}

impl<'ebook> Iterator for EpubTocEntryIter<'ebook> {
    type Item = EpubTocEntry<'ebook>;

    fn next(&mut self) -> Option<Self::Item> {
        self.iter
            .next()
            .map(|data| self.ctx.create_entry(self.version, data, self.next_depth))
    }
}
