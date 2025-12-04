//! EPUB-specific table-of-contents content.

use crate::ebook::element::{AttributeData, Attributes, Href};
use crate::ebook::epub::manifest::{EpubManifestEntry, EpubManifestEntryProvider};
use crate::ebook::epub::{EpubConfig, EpubVersion};
use crate::ebook::toc::{Toc, TocChildren, TocEntry, TocEntryKind};
use std::collections::HashMap;
use std::collections::hash_map::Iter as HashMapIter;
use std::fmt::{Debug, Formatter};
use std::slice::Iter as SliceIter;

////////////////////////////////////////////////////////////////////////////////
// PRIVATE API
////////////////////////////////////////////////////////////////////////////////

pub(super) type TocGroups = HashMap<EpubTocKey<'static>, EpubTocEntryData>;

#[derive(Debug, Hash, PartialEq, Eq)]
pub(super) struct EpubTocKey<'a> {
    kind: TocEntryKind<'a>,
    version: Option<EpubVersion>,
}

impl<'a> EpubTocKey<'a> {
    pub(super) fn new(kind: TocEntryKind<'a>, version: Option<EpubVersion>) -> Self {
        Self { kind, version }
    }

    pub(super) fn of(kind: TocEntryKind<'a>, version: EpubVersion) -> Self {
        Self::new(kind, Some(version))
    }
}

#[derive(Debug, PartialEq)]
pub(super) struct EpubTocData {
    /// Despite a toc being required for EPUBs, allow
    /// leniency for those that don't if `strict` is
    /// disabled within [`EpubOpenOptions`](super::EpubOpenOptions).
    preferred_toc: Option<EpubVersion>,
    preferred_landmarks: Option<EpubVersion>,
    preferred_page_list: Option<EpubVersion>,
    toc_map: TocGroups,
}

impl EpubTocData {
    pub(super) fn new(toc_map: TocGroups) -> Self {
        Self {
            toc_map,
            preferred_toc: None,
            preferred_page_list: None,
            preferred_landmarks: None,
        }
    }

    pub(super) fn empty() -> Self {
        Self::new(HashMap::new())
    }

    pub(super) fn from_guide(data: EpubTocEntryData) -> Self {
        let mut map = HashMap::new();
        map.insert(
            EpubTocKey::of(TocEntryKind::Landmarks, EpubVersion::EPUB2),
            data,
        );
        Self::new(map)
    }

    pub(super) fn extend(&mut self, data: Self) {
        self.toc_map.extend(data.toc_map);
    }

    pub(super) fn set_preferences(&mut self, config: &EpubConfig) {
        self.preferred_toc = self.get_preferred_kind(TocEntryKind::Toc, config.preferred_toc);
        self.preferred_landmarks =
            self.get_preferred_kind(TocEntryKind::Landmarks, config.preferred_landmarks);
        self.preferred_page_list =
            self.get_preferred_kind(TocEntryKind::PageList, config.preferred_page_list);
    }

    /// Gets the id of the first available preference, otherwise returns [`None`].
    fn get_preferred_kind(
        &self,
        kind: TocEntryKind<'static>,
        preferred_version: EpubVersion,
    ) -> Option<EpubVersion> {
        let versions: [EpubVersion; 2] = match preferred_version {
            // Retrieve the EPUB2 variant with EPUB3 as a fallback.
            EpubVersion::Epub2(_) => [EpubVersion::EPUB2, EpubVersion::EPUB3],
            // Retrieve the EPUB3 variant with EPUB2 as a fallback.
            _ => [EpubVersion::EPUB3, EpubVersion::EPUB2],
        };

        versions.into_iter().find(move |version| {
            self.toc_map
                .contains_key(&EpubTocKey::of(kind.clone(), *version))
        })
    }
}

#[derive(Debug, Default, Hash, PartialEq)]
pub(super) struct EpubTocEntryData {
    pub(super) id: Option<String>,
    pub(super) order: usize,
    pub(super) depth: usize,
    pub(super) label: String,
    pub(super) kind: TocEntryKind<'static>,
    pub(super) href: Option<String>,
    pub(super) href_raw: Option<String>,
    pub(super) attributes: Vec<AttributeData>,
    pub(super) children: Vec<EpubTocEntryData>,
}

////////////////////////////////////////////////////////////////////////////////
// PUBLIC API
////////////////////////////////////////////////////////////////////////////////

/// An EPUB table of contents, see [`Toc`] for additional details.
///
/// For EPUB 3 ebooks backwards compatible with EPUB2,
/// the preferred toc formats are configurable via [`EpubOpenOptions`](super::EpubOpenOptions).
/// Methods regarding preferred format:
/// - [`EpubToc::contents`] (`toc`)
/// - [`EpubToc::page_list`] (`page-list`)
/// - [`EpubToc::landmarks`] (`landmarks/guide`)
/// - [`EpubToc::by_kind_version`]
#[derive(Copy, Clone)]
pub struct EpubToc<'ebook> {
    provider: EpubManifestEntryProvider<'ebook>,
    data: &'ebook EpubTocData,
}

impl<'ebook> EpubToc<'ebook> {
    pub(super) fn new(
        provider: EpubManifestEntryProvider<'ebook>,
        data: &'ebook EpubTocData,
    ) -> Self {
        EpubToc { provider, data }
    }

    fn by_toc_key(
        &self,
        kind: TocEntryKind<'ebook>,
        version: Option<EpubVersion>,
    ) -> Option<EpubTocEntry<'ebook>> {
        self.data
            .toc_map
            .get(&EpubTocKey::new(kind, version))
            .map(|data| EpubTocEntry::new(data, self.provider))
    }

    /// The preferred **page list** format, mapping to the EPUB 2 or EPUB 3
    /// [`TocEntryKind::PageList`] format, if present.
    ///
    /// The default preferred format (EPUB 3) is configurable via [`EpubOpenOptions`](super::EpubOpenOptions).
    pub fn page_list(&self) -> Option<EpubTocEntry<'ebook>> {
        self.by_toc_key(TocEntryKind::PageList, self.data.preferred_page_list)
    }

    /// The preferred **guide/landmarks** format, mapping to the EPUB 2 (Guide) or EPUB 3
    /// [`TocEntryKind::Landmarks`] format, if present.
    ///
    /// The default preferred format (EPUB 3) is configurable via [`EpubOpenOptions`](super::EpubOpenOptions).
    pub fn landmarks(&self) -> Option<EpubTocEntry<'ebook>> {
        self.by_toc_key(TocEntryKind::Landmarks, self.data.preferred_landmarks)
    }

    /// Returns the **root** toc entry for a given [`TocEntryKind`],
    /// using the specified [`EpubVersion`].
    ///
    /// **This method is useful when [`EpubOpenOptions::store_all`](super::EpubOpenOptions::store_all) is set to `true`.**
    ///
    /// An example:
    /// - [`TocEntryKind::PageList`] + [`EpubVersion::Epub2`] = Legacy EPUB 2 NCX page list.
    /// - [`TocEntryKind::PageList`] + [`EpubVersion::Epub3`] = EPUB 3 XHTML page list.
    ///
    /// This method is **only** effective for the following kinds,
    /// otherwise [`None`] is always returned:
    /// - [`TocEntryKind::Toc`]
    /// - [`TocEntryKind::Landmarks`]
    /// - [`TocEntryKind::PageList`]
    pub fn by_kind_version(
        &self,
        kind: impl Into<TocEntryKind<'ebook>>,
        version: EpubVersion,
    ) -> Option<EpubTocEntry<'ebook>> {
        // "Normalize" epub version as the contained value
        // may be different (e.g., "3.1", "3.2")
        // Version must be `2.0` or `3.0`
        self.by_toc_key(kind.into(), Some(version.as_major()))
    }
}

#[allow(refining_impl_trait)]
impl<'ebook> Toc<'ebook> for EpubToc<'ebook> {
    fn contents(&self) -> Option<EpubTocEntry<'ebook>> {
        self.by_toc_key(TocEntryKind::Toc, self.data.preferred_toc)
    }

    fn by_kind(&self, kind: impl Into<TocEntryKind<'ebook>>) -> Option<EpubTocEntry<'ebook>> {
        let kind = kind.into();
        let preferred_version = match kind {
            TocEntryKind::Landmarks => self.data.preferred_landmarks,
            TocEntryKind::PageList => self.data.preferred_page_list,
            TocEntryKind::Toc => self.data.preferred_toc,
            _ => None,
        };
        self.by_toc_key(kind, preferred_version)
    }

    fn kinds(&self) -> EpubTocIter<'ebook> {
        self.into_iter()
    }
}

impl Debug for EpubToc<'_> {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("EpubToc")
            .field("data", self.data)
            .finish_non_exhaustive()
    }
}

impl PartialEq for EpubToc<'_> {
    fn eq(&self, other: &Self) -> bool {
        self.data == other.data
    }
}

impl<'ebook> IntoIterator for &EpubToc<'ebook> {
    type Item = (&'ebook TocEntryKind<'ebook>, EpubTocEntry<'ebook>);
    type IntoIter = EpubTocIter<'ebook>;

    fn into_iter(self) -> Self::IntoIter {
        EpubTocIter {
            provider: self.provider,
            iter: self.data.toc_map.iter(),
        }
    }
}

impl<'ebook> IntoIterator for EpubToc<'ebook> {
    type Item = (&'ebook TocEntryKind<'ebook>, EpubTocEntry<'ebook>);
    type IntoIter = EpubTocIter<'ebook>;

    fn into_iter(self) -> Self::IntoIter {
        (&self).into_iter()
    }
}

/// An iterator over all root toc kinds within an [`EpubToc`].
///
/// # See Also
/// - [`EpubToc::kinds`]
///
/// # Examples
/// - Iterating over all root toc kinds:
/// ```
/// # use rbook::ebook::errors::EbookResult;
/// # use rbook::{Ebook, Epub};
/// # fn main() -> EbookResult<()> {
/// let epub = Epub::open("tests/ebooks/example_epub")?;
///
/// for (kind, root) in epub.toc() {
///     // process toc root //
/// }
/// # Ok(())
/// # }
/// ```
pub struct EpubTocIter<'ebook> {
    provider: EpubManifestEntryProvider<'ebook>,
    iter: HashMapIter<'ebook, EpubTocKey<'ebook>, EpubTocEntryData>,
}

impl<'ebook> Iterator for EpubTocIter<'ebook> {
    type Item = (&'ebook TocEntryKind<'ebook>, EpubTocEntry<'ebook>);

    fn next(&mut self) -> Option<Self::Item> {
        self.iter
            .next()
            .map(move |(kind, data)| (&kind.kind, EpubTocEntry::new(data, self.provider)))
    }
}

/// An entry contained within an [`EpubToc`], encompassing associated metadata.
#[derive(Copy, Clone)]
pub struct EpubTocEntry<'ebook> {
    data: &'ebook EpubTocEntryData,
    provider: EpubManifestEntryProvider<'ebook>,
}

impl<'ebook> EpubTocEntry<'ebook> {
    fn new(data: &'ebook EpubTocEntryData, provider: EpubManifestEntryProvider<'ebook>) -> Self {
        Self { data, provider }
    }

    /// The unique id of a toc entry.
    pub fn id(&self) -> Option<&'ebook str> {
        self.data.id.as_deref()
    }

    /// The resolved absolute percent-encoded `href`,
    /// indicating the location a toc entry points to.
    ///
    /// Returns [`None`] if no `href` (EPUB 3) nor `src` (EPUB 2) attribute
    /// is associated with an entry.
    ///
    /// Example of a resolved href:
    /// ```text
    /// /EPUB/OEBPS/chapters/c1.xhtml#part-1
    /// ```
    ///
    /// # See Also
    /// - [`Href::path`] to retrieve the href value without the query and fragment.
    /// - [`Self::resource`] as the primary means for retrieving ebook content.
    pub fn href(&self) -> Option<Href<'ebook>> {
        self.data.href.as_deref().map(Into::into)
    }

    /// The raw (relative) percent-encoded `href`,
    /// indicating the location a toc entry points to.
    ///
    /// Returns [`None`] if no `href` (EPUB 3) nor `src` (EPUB 2) attribute
    /// is associated with an entry.
    ///
    /// Example of a raw (relative) href:
    /// ```text
    /// ../../../c1.xhtml#part-1
    /// ```
    ///
    /// # Note
    /// [`Self::href`] is recommended over this method unless access to the original
    /// raw `href` is required for analysis.
    /// Providing the raw value to a method such as
    /// [`Ebook::read_resource_bytes`](crate::Ebook::read_resource_bytes) can fail.
    ///
    /// # See Also
    /// - [`Epub`](super::Epub) documentation of `read_resource_bytes` for normalization details.
    /// - [`Href::path`] to retrieve the href value without the query and fragment.
    pub fn href_raw(&self) -> Option<Href<'ebook>> {
        self.data.href_raw.as_deref().map(Into::into)
    }

    /// All additional `XML` [`Attributes`].
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
    /// - [`epub:type`](Self::kind)
    /// - [`src`](Self::href) (EPUB 2; legacy)
    /// - [`playOrder`](Self::order) (EPUB 2; legacy)
    pub fn attributes(&self) -> Attributes<'ebook> {
        (&self.data.attributes).into()
    }
}

#[allow(refining_impl_trait)]
impl<'ebook> TocEntry<'ebook> for EpubTocEntry<'ebook> {
    fn order(&self) -> usize {
        self.data.order
    }

    fn depth(&self) -> usize {
        self.data.depth
    }

    fn label(&self) -> &'ebook str {
        &self.data.label
    }

    fn kind(&self) -> &'ebook TocEntryKind<'ebook> {
        &self.data.kind
    }

    fn children(&self) -> EpubTocChildren<'ebook> {
        EpubTocChildren(*self)
    }

    fn manifest_entry(&self) -> Option<EpubManifestEntry<'ebook>> {
        self.href()
            .and_then(|href| self.provider.by_href(href.path().as_str()))
    }
}

impl Debug for EpubTocEntry<'_> {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("EpubTocEntry")
            .field("data", self.data)
            .finish_non_exhaustive()
    }
}

impl<'a> IntoIterator for &EpubTocEntry<'a> {
    type Item = EpubTocEntry<'a>;
    type IntoIter = EpubTocEntryIter<'a>;

    fn into_iter(self) -> Self::IntoIter {
        EpubTocEntryIter {
            provider: self.provider,
            iter: self.data.children.iter(),
        }
    }
}

impl<'a> IntoIterator for EpubTocEntry<'a> {
    type Item = Self;
    type IntoIter = EpubTocEntryIter<'a>;

    fn into_iter(self) -> Self::IntoIter {
        (&self).into_iter()
    }
}

/// The children of an [`EpubTocEntry`].
///
/// See [`TocChildren`] for more details.
#[derive(Copy, Clone, Debug, PartialEq)]
pub struct EpubTocChildren<'ebook>(EpubTocEntry<'ebook>);

#[allow(refining_impl_trait)]
impl<'ebook> TocChildren<'ebook> for EpubTocChildren<'ebook> {
    fn get(&self, index: usize) -> Option<EpubTocEntry<'ebook>> {
        self.0
            .data
            .children
            .get(index)
            .map(|data| EpubTocEntry::new(data, self.0.provider))
    }

    fn iter(&self) -> EpubTocEntryIter<'ebook> {
        self.into_iter()
    }

    fn flatten(&self) -> impl Iterator<Item = EpubTocEntry<'ebook>> + 'ebook {
        struct FlatEpubTocEntryIterator<'ebook> {
            stack: Vec<&'ebook EpubTocEntryData>,
            provider: EpubManifestEntryProvider<'ebook>,
        }

        impl<'ebook> Iterator for FlatEpubTocEntryIterator<'ebook> {
            type Item = EpubTocEntry<'ebook>;

            fn next(&mut self) -> Option<Self::Item> {
                let entry = self.stack.pop()?;

                // Push children in reverse order to maintain DFS order
                self.stack.extend(entry.children.iter().rev());
                Some(EpubTocEntry::new(entry, self.provider))
            }
        }

        FlatEpubTocEntryIterator {
            stack: self.0.data.children.iter().rev().collect(),
            provider: self.0.provider,
        }
    }

    fn len(&self) -> usize {
        self.0.data.children.len()
    }
}

impl PartialEq<Self> for EpubTocEntry<'_> {
    fn eq(&self, other: &Self) -> bool {
        self.data == other.data
    }
}

impl<'ebook> IntoIterator for &EpubTocChildren<'ebook> {
    type Item = EpubTocEntry<'ebook>;
    type IntoIter = EpubTocEntryIter<'ebook>;

    fn into_iter(self) -> Self::IntoIter {
        self.0.into_iter()
    }
}

impl<'ebook> IntoIterator for EpubTocChildren<'ebook> {
    type Item = EpubTocEntry<'ebook>;
    type IntoIter = EpubTocEntryIter<'ebook>;

    fn into_iter(self) -> Self::IntoIter {
        self.0.into_iter()
    }
}

/// An iterator over the immediate [`children`](EpubTocEntry) of an [`EpubTocEntry`].
///
/// # See Also
/// - [`EpubTocChildren::iter`]
/// - [`EpubTocChildren::flatten`]
///
/// # Examples
/// - Iterating over immediate children:
/// ```
/// # use rbook::ebook::errors::EbookResult;
/// # use rbook::ebook::toc::{Toc, TocEntry};
/// # use rbook::{Ebook, Epub};
/// # fn main() -> EbookResult<()> {
/// let epub = Epub::open("tests/ebooks/example_epub")?;
/// let nav_root = epub.toc().contents().unwrap();
///
/// for child in nav_root {
///     // process immediate child //
/// }
/// # Ok(())
/// # }
/// ```
pub struct EpubTocEntryIter<'ebook> {
    provider: EpubManifestEntryProvider<'ebook>,
    iter: SliceIter<'ebook, EpubTocEntryData>,
}

impl<'ebook> Iterator for EpubTocEntryIter<'ebook> {
    type Item = EpubTocEntry<'ebook>;

    fn next(&mut self) -> Option<Self::Item> {
        self.iter
            .next()
            .map(|data| EpubTocEntry::new(data, self.provider))
    }
}
