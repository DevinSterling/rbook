//! EPUB-specific metadata content.
//!
//! # See Also
//! - [`ebook::metadata`](crate::ebook::metadata) for the general metadata module.

#[cfg(feature = "write")]
mod write;

use crate::ebook::element::{
    Attribute, Attributes, AttributesData, Href, Name, Properties, TextDirection,
};
use crate::ebook::epub::consts::{dc, opf};
use crate::ebook::epub::metadata::macros::{impl_meta_entry, impl_meta_entry_abstraction};
use crate::ebook::epub::package::{EpubPackageData, EpubPackageMetaContext};
use crate::ebook::metadata::datetime::DateTime;
use crate::ebook::metadata::{
    AlternateScript, Contributor, Identifier, Language, LanguageKind, LanguageTag, MetaEntry,
    Metadata, Scheme, Tag, Title, TitleKind, Version,
};
use crate::ebook::resource::ResourceKind;
use crate::util::{self, Sealed};
use indexmap::IndexMap;
use std::fmt::Display;
use std::iter::{Enumerate, FlatMap};
use std::slice::Iter as SliceIter;

#[cfg(feature = "write")]
pub use write::{
    DetachedEpubMetaEntry, EpubMetaEntryMut, EpubMetadataIterMut, EpubMetadataMut,
    EpubRefinementsIterMut, EpubRefinementsMut, marker,
};

////////////////////////////////////////////////////////////////////////////////
// PRIVATE API
////////////////////////////////////////////////////////////////////////////////

pub(super) type EpubMetaGroups = IndexMap<String, Vec<EpubMetaEntryData>>;

#[derive(Debug, PartialEq)]
pub(super) struct EpubMetadataData {
    pub(super) entries: EpubMetaGroups,
}

impl EpubMetadataData {
    pub(super) fn new(entries: EpubMetaGroups) -> Self {
        Self { entries }
    }

    pub(super) fn empty() -> Self {
        Self::new(IndexMap::new())
    }

    pub(super) fn epub2_cover_image_id(&self) -> Option<&str> {
        self.entries
            .get(opf::COVER)
            .and_then(|group| group.first())
            .map(|cover| cover.value.as_str())
    }
}

#[derive(Clone, Debug, Default, PartialEq)]
pub(super) struct EpubRefinementsData(Vec<EpubMetaEntryData>);

impl EpubRefinementsData {
    pub(crate) fn new(refinements: Vec<EpubMetaEntryData>) -> Self {
        Self(refinements)
    }

    fn get_schemes(&self, key: &str) -> impl Iterator<Item = Scheme<'_>> {
        self.by_refinements(key).map(|(_, key_item)| {
            // Note: There is usually a `scheme` associated with the `value`,
            // although it's not always guaranteed
            let scheme = key_item.attributes.get_value(opf::SCHEME);
            Scheme::new(scheme, &key_item.value)
        })
    }

    fn by_refinements(&self, property: &str) -> impl Iterator<Item = (usize, &EpubMetaEntryData)> {
        self.0
            .iter()
            .enumerate()
            .filter(move |(_, refinement)| refinement.property == property)
    }

    pub(crate) fn has_refinement(&self, property: &str) -> bool {
        self.0
            .iter()
            .any(|refinement| refinement.property == property)
    }

    pub(crate) fn by_refinement(&self, property: &str) -> Option<&EpubMetaEntryData> {
        self.0
            .iter()
            .find(|refinement| refinement.property == property)
    }
}

impl std::ops::Deref for EpubRefinementsData {
    type Target = Vec<EpubMetaEntryData>;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl std::ops::DerefMut for EpubRefinementsData {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.0
    }
}

#[derive(Clone, Debug, PartialEq)]
pub(super) struct EpubMetaEntryData {
    pub(super) id: Option<String>,
    pub(super) property: String,
    pub(super) value: String,
    pub(super) language: Option<String>,
    pub(super) text_direction: TextDirection,
    pub(super) attributes: AttributesData,
    pub(super) refinements: EpubRefinementsData,
    pub(super) kind: EpubMetaEntryKind,
}

impl Default for EpubMetaEntryData {
    fn default() -> Self {
        Self {
            id: None,
            property: String::new(),
            value: String::new(),
            language: None,
            text_direction: TextDirection::Auto,
            attributes: AttributesData::default(),
            refinements: EpubRefinementsData::default(),
            // A placeholder until the proper kind is provided during parsing
            kind: EpubMetaEntryKind::Meta {
                version: EpubVersion::EPUB3,
            },
        }
    }
}

impl EpubMetaEntryData {
    fn language(&self) -> Option<&str> {
        self.language.as_deref()
    }
}

////////////////////////////////////////////////////////////////////////////////
// PUBLIC API
////////////////////////////////////////////////////////////////////////////////

/// EPUB metadata accessible via [`Epub::metadata`](super::Epub::metadata).
/// See [`Metadata`] for more details.
///
/// # Behavior
/// `rbook` supports arbitrary refinements for any metadata element,
/// including `item` and `itemref` elements.
/// Refinement chains of arbitrary length are supported as well.
///
/// Elements and ID references within the `<package>` element are **not** required
/// to be in chronological order, so refinements are allowed to come before parent elements.
///
/// # Legacy Metadata Attributes
/// [Legacy `opf:*` attributes](https://www.w3.org/submissions/2017/SUBM-epub-packages-20170125/#sec-shared-attrs)
/// on metadata elements are supported.
///
/// Legacy attributes are mapped to the following when the newer
/// [refinement](https://www.w3.org/TR/epub/#app-meta-property-vocab)
/// equivalent is ***not*** present:
/// - [`opf:scheme`](Identifier::scheme)
/// - [`opf:role`](Contributor::main_role)
/// - [`opf:file-as`](MetaEntry::file_as)
/// - [`opf:authority + opf:term`](Tag::scheme)
/// - [`opf:alt-rep + opf:alt-rep-lang`](MetaEntry::alternate_scripts)
///
/// # See Also
/// - [`EpubMetadataMut`] for a mutable view.
#[derive(Copy, Clone, Debug)]
pub struct EpubMetadata<'ebook> {
    ctx: EpubPackageMetaContext<'ebook>,
    package: &'ebook EpubPackageData,
    data: &'ebook EpubMetadataData,
}

impl<'ebook> EpubMetadata<'ebook> {
    pub(super) fn new(package: &'ebook EpubPackageData, data: &'ebook EpubMetadataData) -> Self {
        Self {
            ctx: EpubPackageMetaContext::new(package),
            package,
            data,
        }
    }

    fn data_by_property(
        &self,
        property: &str,
    ) -> impl Iterator<Item = (usize, &'ebook EpubMetaEntryData)> + 'ebook {
        self.data
            .entries
            .get(property)
            .map_or::<&[_], _>(&[], Vec::as_slice)
            .iter()
            .enumerate()
    }

    /// Returns an iterator over non-refining [entries](EpubMetaEntry) whose
    /// [`property`](EpubMetaEntry::property) matches the given one.
    ///
    /// This method is especially useful for retrieving metadata
    /// not explicitly provided from [`EpubMetadata`]:
    /// - `belongs-to-collection`
    /// - `dc:coverage`
    /// - `dc:format`
    /// - `dc:relation`
    /// - `dc:rights`
    /// - `dc:source`
    /// - `dc:type`
    ///
    /// For more information regarding EPUB metadata see:
    /// <https://www.w3.org/TR/epub/#sec-pkg-metadata>
    ///
    /// # Excluded Entries
    /// Refining entries, `<meta>` elements with a `refines` field, are excluded:
    /// ```xml
    /// <meta refines="#parent-id">…</meta>
    /// ```
    ///
    /// # See Also
    /// - [`Self::iter`] to iterate over non-refining metadata entries.
    pub fn by_property(
        &self,
        property: &str,
    ) -> impl Iterator<Item = EpubMetaEntry<'ebook>> + 'ebook {
        let ctx = self.ctx;

        self.data_by_property(property)
            .map(move |(i, data)| ctx.create_entry(data, i))
    }

    /// Searches the metadata hierarchy, including refinements, and returns the
    /// [`EpubMetaEntry`] matching the given `id`, or [`None`] if not found.
    ///
    /// # Performance Implications
    /// This is a recursive linear operation as the underlying structure
    /// is ***not*** a hashmap with `id` as the key.
    /// Generally, the number of metadata entries is small (<20).
    /// However, for larger sets, frequent lookups can impede performance.
    ///
    /// # Note
    /// Refinements on
    /// [`EpubManifestEntry`](super::manifest::EpubManifestEntry::refinements) and
    /// [`EpubSpineEntry`](super::spine::EpubSpineEntry::refinements)
    /// are not searched.
    ///
    /// # Examples
    /// - Retrieving a creator by ID:
    /// ```
    /// # use rbook::Epub;
    /// # use rbook::ebook::toc::TocEntryKind;
    /// # fn main() -> rbook::ebook::errors::EbookResult<()> {
    /// let epub = Epub::open("tests/ebooks/example_epub")?;
    /// let metadata = epub.metadata();
    ///
    /// // Retrieve the author
    /// let author = metadata.creators().next().unwrap();
    /// // Retrieve the same entry by ID
    /// let author_by_id = metadata.by_id("author").unwrap();
    ///
    /// assert_eq!(author, author_by_id);
    /// assert_eq!(Some("author"), author_by_id.id());
    ///
    /// // Attempt to retrieve a non-existent author
    /// assert_eq!(None, metadata.by_id("other-author"));
    /// # Ok(())
    /// # }
    /// ```
    pub fn by_id(&self, id: &str) -> Option<EpubMetaEntry<'ebook>> {
        // Returns the matched entry along with its local `index` and `refines` field:
        // `(index, refines, entry)`
        fn dfs_by_id<'a>(
            id: &str,
            entry: &'a EpubMetaEntryData,
            // The index of `entry` within its parent collection.
            // - The index is passed in by the caller because entries do not store
            //   their position within the parent collection.
            //   The index is a required field for `EpubMetaEntry` views.
            index: usize,
        ) -> Option<(usize, Option<&'a str>, &'a EpubMetaEntryData)> {
            if entry.id.as_deref() == Some(id) {
                return Some((index, None, entry));
            }

            for (i, refinement) in entry.refinements.iter().enumerate() {
                if let Some(mut found) = dfs_by_id(id, refinement, i) {
                    // Set the refines field if the found entry is a direct refinement
                    if refinement == found.2 {
                        found.1 = entry.id.as_deref();
                    }
                    return Some(found);
                }
            }
            None
        }

        self.data
            .entries
            .values()
            .flat_map(|group| group.iter().enumerate())
            .find_map(|(i, entry)| dfs_by_id(id, entry, i))
            .map(|(i, refines, data)| self.ctx.create_refining_entry(refines, data, i))
    }

    /// The [`Epub`](super::Epub) version (e.g., `2.0`, `3.2`, etc.).
    ///
    /// The returned version may be [`EpubVersion::Unknown`] if
    /// [`EpubOpenOptions::strict`](super::EpubOpenOptions::strict) is disabled.
    ///
    /// See [`EpubMetadata::version_str`] for the original representation.
    ///
    /// # Note
    /// This method is equivalent to calling
    /// [`EpubPackage::version`](super::EpubPackage::version).
    pub fn version(&self) -> EpubVersion {
        self.package.version.parsed
    }

    /// The underlying [`Epub`](super::Epub) version string.
    ///
    /// # Note
    /// This method is equivalent to calling
    /// [`EpubPackage::version_str`](super::EpubPackage::version_str).
    pub fn version_str(&self) -> &'ebook str {
        self.package.version.raw.as_str()
    }

    /// Returns an iterator over non-refining link entries.
    /// Only entries which have a [`kind`](EpubMetaEntry::kind) of [`EpubMetaEntryKind::Link`]
    /// are included in the iterator.
    ///
    /// # Excluded Entries
    /// Refining entries, `<link>` elements with a `refines` field are excluded, for example:
    /// ```xml
    /// <link refines="#parent-id" … />
    /// ```
    ///
    /// # See Also
    /// - [`Self::iter`] to iterate over **non-refining** metadata entries, including links.
    pub fn links(&self) -> impl Iterator<Item = EpubLink<'ebook>> + 'ebook {
        self.iter().filter_map(|entry| entry.as_link())
    }

    /// The publication date; when an [`Epub`](super::Epub) was published.
    #[doc = util::inherent_doc!(Metadata, published)]
    /// # See Also
    /// - [`Self::published_entry`] to retrieve the source [`EpubMetaEntry`] instead.
    pub fn published(&self) -> Option<DateTime> {
        self.published_entry()
            .and_then(|entry| DateTime::parse(entry.value()))
    }

    /// The last modified date; when an [`Epub`](super::Epub) was last modified.
    #[doc = util::inherent_doc!(Metadata, modified)]
    /// # See Also
    /// - [`Self::modified_entry`] to retrieve the source [`EpubMetaEntry`] instead.
    pub fn modified(&self) -> Option<DateTime> {
        self.modified_entry()
            .and_then(|entry| DateTime::parse(entry.value()))
    }

    /// The publication date entry; when an [`Epub`](super::Epub) was published.
    ///
    /// # Date Value
    /// The contained date [value](EpubMetaEntry::value) is non-parsed,
    /// typically in [**ISO 8601-1**](https://www.iso.org/iso-8601-date-and-time-format.html) format.
    ///
    /// The value may be in different formats across various EPUBs:
    /// - `2025-12-01` (ISO 8601-1)
    /// - `2025-12-01T00:40:51Z` (ISO 8601-1)
    /// - `2025-12-01 00:00:00+03:27` (RFC 3339)
    ///
    /// Rare and generally not recommended, although possible:
    /// - `December 2025`
    /// - `1.12.2025`
    ///
    /// # See Also
    /// - [`Self::published`] to retrieve the parsed date.
    pub fn published_entry(&self) -> Option<EpubMetaEntry<'ebook>> {
        let mut inferred_date = None;

        for (i, date) in self.data_by_property(dc::DATE) {
            match date.attributes.by_name(opf::OPF_EVENT) {
                Some(opf_event) if opf_event.value() == opf::PUBLICATION => {
                    return Some(self.ctx.create_entry(date, i));
                }
                // If the attribute is not present, infer as the publication date for now
                None if inferred_date.is_none() => inferred_date = Some((i, date)),
                _ => {}
            }
        }

        // Fallback to `dc:date` without `opf:event=publication`
        inferred_date.map(|(i, date)| self.ctx.create_entry(date, i))
    }

    /// The last modified date entry; when an [`Epub`](super::Epub) was last modified.
    ///
    /// # See Also
    /// - [`Self::published_entry`] to retrieve the publication date entry.
    /// - [`Self::modified`] to retrieve the parsed date.
    pub fn modified_entry(&self) -> Option<EpubMetaEntry<'ebook>> {
        // Attempt to retrieve `dcterms:modified` first
        if let Some((i, modified_date)) = self.data_by_property(dc::MODIFIED).next() {
            return Some(self.ctx.create_entry(modified_date, i));
        }

        // Fallback to `dc:date` with `opf:event=modification`
        self.data_by_property(dc::DATE)
            .find(|(_, date)| {
                date.attributes
                    .get_value(opf::OPF_EVENT)
                    .is_some_and(|opf_event| opf_event == opf::MODIFICATION)
            })
            .map(|(i, date)| self.ctx.create_entry(date, i))
    }

    /// The primary unique [identifier](EpubIdentifier) of an [`Epub`](super::Epub).
    ///
    /// Returns [`None`] if there is no primary unique identifier when
    /// [`EpubOpenOptions::strict`](super::EpubOpenOptions::strict) is disabled.
    ///
    /// As EPUBs may include multiple identifiers (`<dc:identifier>` elements),
    /// this method is for retrieval of the primary identifier as declared in
    /// the `<package>` element by the `unique-identifier` attribute.
    #[doc = util::inherent_doc!(Metadata, identifier)]
    pub fn identifier(&self) -> Option<EpubIdentifier<'ebook>> {
        self.data_by_property(dc::IDENTIFIER)
            .find(|(_, data)| data.id.as_ref() == Some(&self.package.unique_identifier))
            .map(|(i, data)| EpubIdentifier::new(self.ctx.create_entry(data, i)))
    }

    /// Returns an iterator over **all** [identifiers](EpubIdentifier)
    /// by [`order`](EpubMetaEntry::order).
    #[doc = util::inherent_doc!(Metadata, identifiers)]
    pub fn identifiers(&self) -> impl Iterator<Item = EpubIdentifier<'ebook>> + 'ebook {
        let ctx = self.ctx;

        self.data_by_property(dc::IDENTIFIER)
            .map(move |(i, data)| EpubIdentifier::new(ctx.create_entry(data, i)))
    }

    /// The main [language](EpubLanguage) of an [`Epub`](super::Epub).
    ///
    /// Returns [`None`] if there is no language specified when
    /// [`EpubOpenOptions::strict`](super::EpubOpenOptions::strict) is disabled.
    #[doc = util::inherent_doc!(Metadata, language)]
    pub fn language(&self) -> Option<EpubLanguage<'ebook>> {
        self.languages().next()
    }

    /// Returns an iterator over **all** [languages](EpubLanguage)
    /// by [`order`](EpubMetaEntry::order).
    #[doc = util::inherent_doc!(Metadata, languages)]
    pub fn languages(&self) -> impl Iterator<Item = EpubLanguage<'ebook>> + 'ebook {
        let ctx = self.ctx;

        self.data_by_property(dc::LANGUAGE)
            .map(move |(i, data)| EpubLanguage::new(ctx.create_entry(data, i)))
    }

    /// The main [title](EpubTitle) of an [`Epub`](super::Epub).
    ///
    /// Returns [`None`] if there is no title specified when
    /// [`EpubOpenOptions::strict`](super::EpubOpenOptions::strict) is disabled.
    #[doc = util::inherent_doc!(Metadata, title)]
    pub fn title(&self) -> Option<EpubTitle<'ebook>> {
        self.titles().find(|title| title.is_main_title)
    }

    /// Returns an iterator over **all** [titles](EpubTitle)
    /// by [`order`](EpubMetaEntry::order).
    #[doc = util::inherent_doc!(Metadata, titles)]
    pub fn titles(&self) -> impl Iterator<Item = EpubTitle<'ebook>> + 'ebook {
        let ctx = self.ctx;
        // Although caching the inferred main title is possible,
        // this is generally inexpensive (nearly all publications only have one title).
        // Caching also complicates write-back of EPUB metadata.
        let inferred_main_title_index = self
            .data_by_property(dc::TITLE)
            // First, try to find if a main `title-type` exists
            .position(|(_, title)| {
                title
                    .refinements
                    .by_refinement(opf::TITLE_TYPE)
                    .is_some_and(|title_type| title_type.value == opf::MAIN_TITLE_TYPE)
            })
            // If not, retrieve the first title
            .unwrap_or(0);

        self.data_by_property(dc::TITLE).map(move |(i, data)| {
            EpubTitle::new(ctx.create_entry(data, i), i == inferred_main_title_index)
        })
    }

    /// The main description with an [`order`](EpubMetaEntry::order) of `0`.
    #[doc = util::inherent_doc!(Metadata, description)]
    pub fn description(&self) -> Option<EpubMetaEntry<'ebook>> {
        self.descriptions().next()
    }

    /// Returns an iterator over **all** descriptions by [`order`](EpubMetaEntry::order).
    #[doc = util::inherent_doc!(Metadata, descriptions)]
    pub fn descriptions(&self) -> impl Iterator<Item = EpubMetaEntry<'ebook>> + 'ebook {
        let ctx = self.ctx;

        self.data_by_property(dc::DESCRIPTION)
            .map(move |(i, data)| ctx.create_entry(data, i))
    }

    /// Returns an iterator over **all** [creators](EpubContributor)
    /// by [`order`](EpubMetaEntry::order).
    #[doc = util::inherent_doc!(Metadata, creators)]
    pub fn creators(&self) -> impl Iterator<Item = EpubContributor<'ebook>> + 'ebook {
        let ctx = self.ctx;

        self.data_by_property(dc::CREATOR)
            .map(move |(i, data)| EpubContributor::new(ctx.create_entry(data, i)))
    }

    /// Returns an iterator over **all** [contributors](EpubContributor)
    /// by [`order`](EpubMetaEntry::order).
    #[doc = util::inherent_doc!(Metadata, contributors)]
    pub fn contributors(&self) -> impl Iterator<Item = EpubContributor<'ebook>> + 'ebook {
        let ctx = self.ctx;

        self.data_by_property(dc::CONTRIBUTOR)
            .map(move |(i, data)| EpubContributor::new(ctx.create_entry(data, i)))
    }

    /// Returns an iterator over **all** [publishers](EpubContributor)
    /// by [`order`](EpubMetaEntry::order).
    #[doc = util::inherent_doc!(Metadata, publishers)]
    pub fn publishers(&self) -> impl Iterator<Item = EpubContributor<'ebook>> + 'ebook {
        let ctx = self.ctx;

        self.data_by_property(dc::PUBLISHER)
            .map(move |(i, data)| EpubContributor::new(ctx.create_entry(data, i)))
    }

    /// Returns an iterator over **all** generators by [`order`](EpubMetaEntry::order).
    #[doc = util::inherent_doc!(Metadata, generators)]
    pub fn generators(&self) -> impl Iterator<Item = EpubMetaEntry<'ebook>> + 'ebook {
        self.by_property(opf::GENERATOR)
    }

    /// Returns an iterator over **all** [tags](EpubTag) by [`order`](EpubMetaEntry::order).
    #[doc = util::inherent_doc!(Metadata, tags)]
    pub fn tags(&self) -> impl Iterator<Item = EpubTag<'ebook>> + 'ebook {
        let ctx = self.ctx;

        self.data_by_property(dc::SUBJECT)
            .map(move |(i, data)| EpubTag::new(ctx.create_entry(data, i)))
    }

    /// Returns an iterator over non-refining metadata entries.
    #[doc = util::inherent_doc!(Metadata, iter)]
    /// # See Also
    /// - [`EpubMetadataIter`] for important details.
    pub fn iter(&self) -> EpubMetadataIter<'ebook> {
        EpubMetadataIter {
            ctx: self.ctx,
            iter: self
                .data
                .entries
                .values()
                .flat_map(|group| group.iter().enumerate()),
        }
    }
}

impl Sealed for EpubMetadata<'_> {}

#[allow(refining_impl_trait)]
impl<'ebook> Metadata<'ebook> for EpubMetadata<'ebook> {
    fn version_str(&self) -> Option<&'ebook str> {
        Some(self.version_str())
    }

    fn version(&self) -> Option<Version> {
        Some(self.version().version())
    }

    fn published(&self) -> Option<DateTime> {
        self.published()
    }

    fn modified(&self) -> Option<DateTime> {
        self.modified()
    }

    fn identifier(&self) -> Option<EpubIdentifier<'ebook>> {
        self.identifier()
    }

    fn identifiers(&self) -> impl Iterator<Item = EpubIdentifier<'ebook>> + 'ebook {
        self.identifiers()
    }

    fn language(&self) -> Option<EpubLanguage<'ebook>> {
        self.language()
    }

    fn languages(&self) -> impl Iterator<Item = EpubLanguage<'ebook>> + 'ebook {
        self.languages()
    }

    fn title(&self) -> Option<EpubTitle<'ebook>> {
        self.title()
    }

    fn titles(&self) -> impl Iterator<Item = EpubTitle<'ebook>> + 'ebook {
        self.titles()
    }

    fn description(&self) -> Option<EpubMetaEntry<'ebook>> {
        self.description()
    }

    fn descriptions(&self) -> impl Iterator<Item = EpubMetaEntry<'ebook>> + 'ebook {
        self.descriptions()
    }

    fn creators(&self) -> impl Iterator<Item = EpubContributor<'ebook>> + 'ebook {
        self.creators()
    }

    fn contributors(&self) -> impl Iterator<Item = EpubContributor<'ebook>> + 'ebook {
        self.contributors()
    }

    fn publishers(&self) -> impl Iterator<Item = EpubContributor<'ebook>> + 'ebook {
        self.publishers()
    }

    fn generators(&self) -> impl Iterator<Item = impl MetaEntry<'ebook> + 'ebook> + 'ebook {
        self.generators()
    }

    fn tags(&self) -> impl Iterator<Item = EpubTag<'ebook>> + 'ebook {
        self.tags()
    }

    fn iter(&self) -> EpubMetadataIter<'ebook> {
        self.iter()
    }
}

impl PartialEq for EpubMetadata<'_> {
    fn eq(&self, other: &Self) -> bool {
        self.data == other.data
    }
}

impl<'ebook> IntoIterator for &EpubMetadata<'ebook> {
    type Item = EpubMetaEntry<'ebook>;
    type IntoIter = EpubMetadataIter<'ebook>;

    fn into_iter(self) -> Self::IntoIter {
        self.iter()
    }
}

impl<'ebook> IntoIterator for EpubMetadata<'ebook> {
    type Item = EpubMetaEntry<'ebook>;
    type IntoIter = EpubMetadataIter<'ebook>;

    fn into_iter(self) -> Self::IntoIter {
        self.iter()
    }
}

pub(super) type InnerMetadataIter<ValuesIter, SliceIter, MapArgs> =
    FlatMap<ValuesIter, Enumerate<SliceIter>, fn(MapArgs) -> Enumerate<SliceIter>>;

/// An iterator over non-refining metadata [entries](EpubMetaEntry)
/// contained within [`EpubMetadata`].
///
/// Each entry is first grouped by its [`property`](EpubMetadata::by_property),
/// then [`order`](MetaEntry::order) before being flattened into a single iterator.
/// As grouping by property relies on a hash map, the order in which property groups appear
/// is arbitrary; non-deterministic.
///
/// # Included Entries
/// All the following types retrievable via [`EpubMetaEntry::kind`] are included
/// in the iterator:
/// - [`EpubMetaEntryKind::DublinCore`]
/// - [`EpubMetaEntryKind::Meta`]
/// - [`EpubMetaEntryKind::Link`]
///
/// # Excluded Entries
/// Refining entries elements with a `refines` field are excluded, for example:
/// ```xml
/// <meta refines="#parent-id" …>…</meta>
/// <link refines="#parent-id" … />
/// ```
///
/// # See Also
/// - [`EpubMetadata::by_property`] to iterate over **non-refining** metadata entries by property.
/// - [`EpubMetadata::links`] to iterate over **non-refining** link entries.
/// - [`EpubMetadata::iter`] to create an instance of this struct.
///
/// # Examples
/// - Iterating over metadata entries:
/// ```
/// # use rbook::Epub;
/// # fn main() -> rbook::ebook::errors::EbookResult<()> {
/// let epub = Epub::open("tests/ebooks/example_epub")?;
///
/// // Iterate over all non-refining metadata elements
/// for entry in epub.metadata() {
///     // Access refinements
///     for refinement in entry.refinements() {
///         // A refinement references its parent by id
///         assert_eq!(entry.id(), refinement.refines());
///
///         // Although rare, a refinement can also have refinements
///         let nested_refinements = refinement.refinements();
///     }
/// }
/// # Ok(())
/// # }
/// ```
pub struct EpubMetadataIter<'ebook> {
    ctx: EpubPackageMetaContext<'ebook>,
    iter: InnerMetadataIter<
        indexmap::map::Values<'ebook, String, Vec<EpubMetaEntryData>>,
        SliceIter<'ebook, EpubMetaEntryData>,
        &'ebook Vec<EpubMetaEntryData>,
    >,
}

impl<'ebook> Iterator for EpubMetadataIter<'ebook> {
    type Item = EpubMetaEntry<'ebook>;

    fn next(&mut self) -> Option<Self::Item> {
        self.iter
            .next()
            .map(|(i, data)| self.ctx.create_entry(data, i))
    }
}

/// A collection of [`EpubMetaEntry`] refinements providing further clarifying detail.
///
/// Refinements, such as `alternate-script`, `title-type`, and `media:duration`,
/// provide additional details for their parent
/// (metadata entries, manifest entries, or spine entries).
///
/// For more information regarding refinements, see:
/// <https://www.w3.org/TR/epub/#attrdef-refines>
///
/// # See Also
/// - [`EpubRefinementsMut`] for a mutable view.
#[derive(Copy, Clone, Debug)]
pub struct EpubRefinements<'ebook> {
    ctx: EpubPackageMetaContext<'ebook>,
    /// When modifying refinements, the `parent_id` may be [`None`].
    ///
    /// See [`EpubMetaEntryMut::refinements_mut`].
    parent_id: Option<&'ebook str>,
    data: &'ebook EpubRefinementsData,
}

impl<'ebook> EpubRefinements<'ebook> {
    pub(super) fn new(
        ctx: EpubPackageMetaContext<'ebook>,
        parent_id: Option<&'ebook str>,
        data: &'ebook EpubRefinementsData,
    ) -> Self {
        Self {
            ctx,
            parent_id,
            data,
        }
    }

    /// The number of refinement entries contained within.
    pub fn len(&self) -> usize {
        self.data.len()
    }

    /// Returns `true` if there are no refinements.
    pub fn is_empty(&self) -> bool {
        self.data.is_empty()
    }

    /// Returns the associated refinement ([`EpubMetaEntry`])
    /// if the given `index` is less than [`Self::len`], otherwise [`None`].
    pub fn get(&self, index: usize) -> Option<EpubMetaEntry<'ebook>> {
        self.data
            .get(index)
            .map(|data| self.ctx.create_entry(data, index))
    }

    /// Returns an iterator over all **direct** refining [`EpubMetaEntry`] entries.
    ///
    /// # Nested Refinements
    /// The returned iterator is not recursive.
    /// To iterate over nested refinements, call [`Self::iter`] on yielded entries.
    pub fn iter(&self) -> EpubRefinementsIter<'ebook> {
        EpubRefinementsIter {
            ctx: self.ctx,
            parent_id: self.parent_id,
            iter: self.data.iter().enumerate(),
        }
    }

    /// Returns an iterator over all **direct** refinements matching the specified `property`.
    ///
    /// # Examples
    /// - Retrieving the `role` refinement for a creator:
    /// ```
    /// # use rbook::Epub;
    /// # fn main() -> rbook::ebook::errors::EbookResult<()> {
    /// let epub = Epub::open("tests/ebooks/example_epub")?;
    /// let author = epub.metadata().creators().next().unwrap();
    /// let role = author.refinements().by_property("role").next().unwrap();
    ///
    /// assert_eq!("aut", role.value());
    /// # Ok(())
    /// # }
    /// ```
    pub fn by_property(
        &self,
        property: &'ebook str,
    ) -> impl Iterator<Item = EpubMetaEntry<'ebook>> + 'ebook {
        let ctx = self.ctx;
        let parent_id = self.parent_id;

        self.data
            .by_refinements(property)
            .map(move |(i, data)| ctx.create_refining_entry(parent_id, data, i))
    }

    /// Returns `true` if the `property` is present.
    ///
    /// # Examples
    /// - Checking if a refinement exists:
    /// ```
    /// # use rbook::Epub;
    /// # fn main() -> rbook::ebook::errors::EbookResult<()> {
    /// let epub = Epub::open("tests/ebooks/example_epub")?;
    /// let author = epub.metadata().creators().next().unwrap();
    /// let refinements = author.refinements();
    ///
    /// assert!(refinements.has_property("file-as"));
    /// # Ok(())
    /// # }
    /// ```
    pub fn has_property(&self, property: &str) -> bool {
        self.data.has_refinement(property)
    }
}

impl PartialEq for EpubRefinements<'_> {
    fn eq(&self, other: &Self) -> bool {
        self.data == other.data
    }
}

impl<'ebook> IntoIterator for &EpubRefinements<'ebook> {
    type Item = EpubMetaEntry<'ebook>;
    type IntoIter = EpubRefinementsIter<'ebook>;

    fn into_iter(self) -> Self::IntoIter {
        self.iter()
    }
}

impl<'ebook> IntoIterator for EpubRefinements<'ebook> {
    type Item = EpubMetaEntry<'ebook>;
    type IntoIter = EpubRefinementsIter<'ebook>;

    fn into_iter(self) -> Self::IntoIter {
        self.iter()
    }
}

/// An iterator over all direct [entries](EpubMetaEntry) contained within [`EpubRefinements`].
///
/// # See Also
/// - [`EpubRefinements::iter`] to create an instance of this struct.
///
/// # Examples
/// - Iterating over all refinements of a metadata entry:
/// ```
/// # use rbook::Epub;
/// # fn main() -> rbook::ebook::errors::EbookResult<()> {
/// let epub = Epub::open("tests/ebooks/example_epub")?;
/// let source = epub.metadata().by_property("dc:source").next().unwrap();
///
/// for refinement in source.refinements() {
///     // process refinement //
/// }
/// # Ok(())
/// # }
/// ```
pub struct EpubRefinementsIter<'ebook> {
    ctx: EpubPackageMetaContext<'ebook>,
    parent_id: Option<&'ebook str>,
    iter: Enumerate<std::slice::Iter<'ebook, EpubMetaEntryData>>,
}

impl<'ebook> Iterator for EpubRefinementsIter<'ebook> {
    type Item = EpubMetaEntry<'ebook>;

    fn next(&mut self) -> Option<Self::Item> {
        self.iter
            .next()
            .map(|(i, data)| self.ctx.create_refining_entry(self.parent_id, data, i))
    }
}

/// A [`MetaEntry`] within [`EpubMetadata`].
///
/// A metadata entry is an element that appears within `<metadata>`
/// inside an EPUB's package.opf file, specifically:
/// - Dublin Core elements: `<dc:*>`
/// - `meta` elements: `<meta>`
/// - `link` elements: `<link>`
///
/// # See Also
/// - [`EpubMetaEntryMut`] for a mutable view.
/// - [`EpubMetaEntryKind`] for more info regarding the different kinds of metadata entries.
/// - [`EpubLink`] for metadata regarding potentially linked external content.
#[derive(Copy, Clone, Debug)]
pub struct EpubMetaEntry<'ebook> {
    ctx: EpubPackageMetaContext<'ebook>,
    /// The "parent id" of refining meta entries.
    refines: Option<&'ebook str>,
    data: &'ebook EpubMetaEntryData,
    index: usize,
}

impl_meta_entry!(EpubMetaEntry);

impl<'ebook> EpubMetaEntry<'ebook> {
    pub(super) fn new(
        ctx: EpubPackageMetaContext<'ebook>,
        refines: Option<&'ebook str>,
        data: &'ebook EpubMetaEntryData,
        index: usize,
    ) -> Self {
        Self {
            ctx,
            refines,
            data,
            index,
        }
    }

    // `data` and `index` provide a common accessor between
    // `EpubMetaEntry` and more specialized views (e.g., EpubContributor).
    //
    // Mainly assists within the `impl_meta_entry` macro.
    fn data(&self) -> &'ebook EpubMetaEntryData {
        self.data
    }

    fn index(&self) -> usize {
        self.index
    }

    /// The unique `id` of a metadata entry.
    pub fn id(&self) -> Option<&'ebook str> {
        self.data.id.as_deref()
    }

    /// The associated element `id` a metadata entry refines.
    ///
    /// Returns [`Some`] if an entry is refining another, otherwise [`None`].
    ///
    /// This is akin to a parent-child relationship where children are
    /// [refinements](Self::refinements),
    /// referring to their parent by `id` through the `refines` field.
    ///
    /// Referenced entries by `id` can be the following:
    /// - [`EpubSpineEntry`](super::spine::EpubSpineEntry::refinements)
    /// - [`EpubManifestEntry`](super::manifest::EpubManifestEntry::refinements)
    /// - [`EpubMetaEntry`](Self::refinements)
    ///
    /// # Example
    /// - Accessing a refinement and inspecting its `refines` field:
    /// ```
    /// # use rbook::Epub;
    /// # fn main() -> rbook::ebook::errors::EbookResult<()> {
    /// let epub = Epub::open("tests/ebooks/example_epub")?;
    ///
    /// // metadata().creators() can be called alternatively as well
    /// let author = epub.metadata().by_property("dc:creator").next().unwrap();
    /// assert_eq!(Some("author"), author.id());
    /// assert_eq!("John Doe", author.value());
    ///
    /// // Occasionally, metadata entries have refinements:
    /// let refinement1 = author.refinements().iter().next().unwrap();
    /// // All refinement refers to their "parent" via id:
    /// assert_eq!(Some("author"), refinement1.refines());
    ///
    /// // Inspecting a refinement's property and value:
    /// assert_eq!("alternate-script", refinement1.property());
    /// assert_eq!("山田太郎", refinement1.value());
    /// # Ok(())
    /// # }
    /// ```
    pub fn refines(&self) -> Option<&'ebook str> {
        self.refines
    }

    /// The `property`, such as `dc:title`, `media:duration`, `file-as`, etc.
    ///
    /// # Property Mapping
    /// Depending on the underlying element type and EPUB version, this field may be mapped
    /// differently:
    ///
    /// | Element Type         | Mapped From                                                        |
    /// |----------------------|--------------------------------------------------------------------|
    /// | Dublin Core `<dc:*>` | Element tag (`<dc:title>…</dc:title>`)                           |
    /// | EPUB 2 `<meta>`      | `name` attribute (`<meta name="cover" content="…"/>`)            |
    /// | EPUB 3 `<meta>`      | `property` attribute (`<meta property="media:duration">…</meta>`)|
    pub fn property(&self) -> Name<'ebook> {
        Name::new(&self.data.property)
    }

    /// The [`Scheme`] of an entry.
    ///
    /// # Scheme Mapping
    /// The behavior of this method changes depending on an entry’s immediate attributes
    /// ([`Self::attributes`]).
    ///
    /// | Attribute presence  | [`Scheme::source`] Mapping | [`Scheme::code`] Mapping |
    /// |---------------------|----------------------------|--------------------------|
    /// | Legacy `opf:scheme` | [`None`]                   | value of `opf:scheme`    |
    /// | `scheme`            | value of `scheme`          | [`Self::value`]          |
    /// | None of the above   | [`None`]                   | [`Self::value`]          |
    ///
    /// # Examples
    /// - Legacy `opf:scheme` attribute present:
    /// ```xml
    /// <dc:identifier id="uid" opf:scheme="URL">
    ///     https://github.com/devinsterling/rbook
    /// </dc:identifier>
    /// ```
    /// ```
    /// # use rbook::Epub;
    /// # fn main() -> rbook::ebook::errors::EbookResult<()> {
    /// let epub = Epub::open("tests/ebooks/example_epub")?;
    /// let metadata = epub.metadata();
    /// // metadata.identifier()?.scheme() may be used alternatively for a higher level view,
    /// // which also takes the `identifier-type` refinement into account unlike here.
    /// let identifier_scheme = metadata.by_property("dc:identifier").next().unwrap().scheme();
    ///
    /// assert_eq!(None, identifier_scheme.source());
    /// assert_eq!("URL", identifier_scheme.code());
    /// # Ok(())
    /// # }
    /// ```
    /// - `scheme` attribute present:
    /// ```xml
    /// <meta property="role" refines="#author" scheme="marc:relators">
    ///     aut
    /// </meta>
    /// ```
    /// ```
    /// # use rbook::Epub;
    /// # fn main() -> rbook::ebook::errors::EbookResult<()> {
    /// # let epub = Epub::open("tests/ebooks/example_epub")?;
    /// # let metadata = epub.metadata();
    /// // metadata.creators().next()?.main_role() may be used alternatively for a higher level view,
    /// // which also takes the `role` refinement into account unlike here.
    /// let author = metadata.by_property("dc:creator").next().unwrap();
    /// let role_scheme = author.refinements().by_property("role").next().unwrap().scheme();
    ///
    /// assert_eq!(Some("marc:relators"), role_scheme.source());
    /// assert_eq!("aut", role_scheme.code());
    /// # Ok(())
    /// # }
    /// ```
    /// - No scheme attribute present:
    /// ```xml
    /// <dc:language>
    ///     en
    /// </dc:language>
    /// ```
    /// ```
    /// # use rbook::Epub;
    /// # fn main() -> rbook::ebook::errors::EbookResult<()> {
    /// # let epub = Epub::open("tests/ebooks/example_epub")?;
    /// # let metadata = epub.metadata();
    /// // metadata.language() may be used alternatively for a higher level view
    /// let language_scheme = metadata.by_property("dc:language").next().unwrap().scheme();
    ///
    /// assert_eq!(None, language_scheme.source());
    /// assert_eq!("en", language_scheme.code());
    /// # Ok(())
    /// # }
    /// ```
    pub fn scheme(&self) -> Scheme<'ebook> {
        if let Some(code) = self.data.attributes.get_value(opf::OPF_SCHEME) {
            return Scheme::new(None, code);
        }

        Scheme::new(self.data.attributes.get_value(opf::SCHEME), self.value())
    }

    /// The specified or inherited language in `BCP 47` format.
    ///
    /// [`None`] is returned if the element contains no `xml:lang` attribute
    /// (and the `<package>` has none).
    ///
    /// # See Also
    /// - [`EpubLink::href_lang`] for the language of the linked resource.
    ///   `<link>` elements do not formally support the `xml:lang` attribute.
    pub fn xml_language(&self) -> Option<LanguageTag<'ebook>> {
        self.data
            .language()
            .or_else(|| self.ctx.package_language())
            .map(|code| LanguageTag::new(code, LanguageKind::Bcp47))
    }

    /// The specified or inherited text direction (`ltr`, `rtl`, or `auto`).
    ///
    /// [`TextDirection::Auto`] is returned if any of the following conditions are met:
    /// - The `<package>` and specified element contains no `dir` attribute.
    /// - [`Self::kind`] is [`EpubMetaEntryKind::Link`], as `<link>`
    ///   elements do not formally support the `dir` attribute.
    pub fn text_direction(&self) -> TextDirection {
        match self.data.text_direction {
            TextDirection::Auto => self.ctx.package_text_direction(),
            text_direction => text_direction,
        }
    }

    /// All additional XML [`Attributes`].
    ///
    /// # Omitted Attributes
    /// The following attributes will **not** be found within the returned collection:
    /// - [`id`](Self::id)
    /// - [`xml:lang`](Self::xml_language)
    /// - [`dir`](Self::text_direction)
    /// - [`property`](Self::property)
    /// - [`refines`](Self::refines)
    /// - [`name`](Self::property) (EPUB 2; legacy)
    /// - [`content`](Self::value) (EPUB 2; legacy)
    pub fn attributes(&self) -> &'ebook Attributes {
        &self.data.attributes
    }

    /// Complementary refinement metadata entries.
    ///
    /// # See Also
    /// - [`Self::refines`] to inspect which entry (by `id`) a metadata entry refines.
    /// - [`EpubManifestEntry::refinements`](super::manifest::EpubManifestEntry::refinements)
    ///   for manifest entry refinements.
    /// - [`EpubSpineEntry::refinements`](super::spine::EpubSpineEntry::refinements)
    ///   for spine entry refinements.
    pub fn refinements(&self) -> EpubRefinements<'ebook> {
        self.ctx
            .create_refinements(self.id(), &self.data.refinements)
    }

    /// The kind of metadata entry.
    pub fn kind(&self) -> EpubMetaEntryKind {
        self.data.kind
    }

    /// Returns an [`EpubLink`] view of a metadata entry,
    /// otherwise [`None`] if the entry is not a `<link>` element.
    ///
    /// This method provides convenient access to link-specific attributes
    /// (e.g., `href`, `hreflang`, `rel`).
    ///
    /// # See Also
    /// - [`EpubMetadata::links`] to iterate over non-refining link entries.
    ///
    /// # Examples
    /// - Converting to [`EpubLink`]:
    /// ```
    /// # use rbook::Epub;
    /// # use rbook::ebook::element::Href;
    /// # fn main() -> rbook::ebook::errors::EbookResult<()> {
    /// let epub = Epub::open("tests/ebooks/example_epub")?;
    /// let meta_entry = epub.metadata().by_id("example-link").unwrap();
    ///
    /// // Convert to link view
    /// let link = meta_entry.as_link().unwrap();
    /// assert_eq!("https://github.com/devinsterling/rbook", link.href().unwrap().as_str());
    /// # Ok(())
    /// # }
    /// ```
    pub fn as_link(&self) -> Option<EpubLink<'ebook>> {
        self.kind().is_link().then_some(EpubLink(*self))
    }
}

impl PartialEq for EpubMetaEntry<'_> {
    fn eq(&self, other: &Self) -> bool {
        self.data == other.data
    }
}

/// A specialized view of [`EpubMetaEntry`] for `<link>` elements.
///
/// Link elements provide associations to resources related to the EPUB publication
/// (e.g., metadata records, alternate representations, related resources).
///
/// # Examples
/// - Accessing link metadata:
/// ```
/// # use rbook::Epub;
/// # fn main() -> rbook::ebook::errors::EbookResult<()> {
/// let epub = Epub::open("tests/ebooks/example_epub")?;
///
/// for link in epub.metadata().links() {
///     if let Some(href) = link.href() {
///         println!("Link to: {}", href.as_str());
///     }
///
///     if let Some(media_type) = link.media_type() {
///         println!("Media type: {}", media_type);
///     }
/// }
/// # Ok(())
/// # }
/// ```
///
/// # See Also
/// - [`DetachedEpubMetaEntry::link`] to create a link metadata entry.
/// - [`EpubMetaEntry::as_link`] to convert from [`EpubMetaEntry`].
/// - <https://www.w3.org/TR/epub/#sec-link-elem> for official EPUB `<link>` documentation
pub struct EpubLink<'ebook>(EpubMetaEntry<'ebook>);

impl<'ebook> EpubLink<'ebook> {
    /// The location of the specified resource a link points to.
    ///
    /// Returns [`None`] if not present.
    pub fn href(&self) -> Option<Href<'ebook>> {
        self.0.data.attributes.get_value(opf::HREF).map(Href::new)
    }

    /// The language of the resource referenced by [`Self::href`].
    ///
    /// Returns [`None`] if not present.
    pub fn href_lang(&self) -> Option<LanguageTag<'ebook>> {
        self.0
            .data
            .attributes
            .get_value(opf::HREFLANG)
            .map(|hreflang| LanguageTag::new(hreflang, LanguageKind::Bcp47))
    }

    /// The `MIME` identifying the media type
    /// of the resource referenced by [`Self::href`].
    ///
    /// Returns [`None`] if not present.
    ///
    /// This method is a lower-level call than [`Self::kind`].
    pub fn media_type(&self) -> Option<&'ebook str> {
        self.0.data.attributes.get_value(opf::MEDIA_TYPE)
    }

    /// The [`ResourceKind`] identifying the media type
    /// of the resource referenced by [`Self::href`].
    ///
    /// Returns [`None`] if not present.
    pub fn kind(&self) -> Option<ResourceKind<'ebook>> {
        self.media_type().map(Into::into)
    }

    /// List of property values.
    pub fn properties(&self) -> &'ebook Properties {
        self.0
            .data
            .attributes
            .by_name(opf::PROPERTIES)
            .map_or(Properties::EMPTY_REFERENCE, Attribute::as_properties)
    }

    /// List of relationship values describing the linked resource.
    ///
    /// Common values include:
    /// - `alternate`: Alternate representation
    /// - `record`: Metadata record
    /// - `xml-signature`: XML signature
    pub fn rel(&self) -> &'ebook Properties {
        self.0
            .data
            .attributes
            .by_name(opf::REL)
            .map_or(Properties::EMPTY_REFERENCE, Attribute::as_properties)
    }

    /// Returns the underlying [`EpubMetaEntry`] to access generic metadata details
    /// such as id, refinements, and attributes.
    pub fn as_meta(&self) -> EpubMetaEntry<'_> {
        self.0
    }
}

/// Contains the kinds of metadata entries that can be found within an
/// [`Epub`](super::Epub)'s `<metadata>` element:
/// - [`DublinCore`](EpubMetaEntryKind::DublinCore)
/// - [`Meta`](EpubMetaEntryKind::Meta)
/// - [`Link`](EpubMetaEntryKind::Link)
///
/// # See Also
/// - [`EpubMetaEntry::kind`] to retrieve the kind from an [`EpubMetaEntry`].
#[non_exhaustive]
#[derive(Copy, Clone, Debug, Hash, PartialEq, Eq)]
pub enum EpubMetaEntryKind {
    /// A set of standardized metadata elements that provide a common vocabulary
    /// for describing relevant information about a publication
    /// (e.g., `dc:title`, `dc:creator`, `dc:publisher`).
    ///
    /// **Dublin Core (`<dc:title>`) element structure example**:
    /// ```xml
    /// <dc:title>
    ///   rbook
    /// </dc:title>
    /// ```
    ///
    /// # See Also
    /// <https://www.w3.org/TR/epub/#sec-opf-dcmes-hd> for official EPUB Dublin Core documentation.
    #[non_exhaustive]
    DublinCore {},

    /// General metadata packaged within an [`Epub`](super::Epub).
    ///
    /// - **Legacy EPUB 2 (OPF2) `<meta>` element structure example**:
    /// ```xml
    /// <meta name="cover" content="c0" />
    /// ```
    /// - **EPUB 3 `<meta>` element structure example**:
    /// ```xml
    /// <meta property="display-seq" refines="#parent">
    ///   0
    /// </meta>
    /// ```
    ///
    /// # Examples
    /// - Pattern Matching:
    /// ```
    /// # use rbook::Epub;
    /// # use rbook::epub::metadata::{EpubMetaEntryKind, EpubVersion};
    /// # fn main() -> rbook::ebook::errors::EbookResult<()> {
    /// let epub = Epub::open("tests/ebooks/example_epub")?;
    /// let source = epub.metadata().by_property("dc:source").next().unwrap();
    ///
    /// match source.kind() {
    ///     EpubMetaEntryKind::Meta { version: EpubVersion::EPUB3, ..} => {},
    ///     EpubMetaEntryKind::Meta { version: EpubVersion::EPUB2, ..} => {},
    ///     EpubMetaEntryKind::Meta { .. } => {},
    ///     _ => {/* Other kind (i.e., Dublin Core, link) */},
    /// };
    /// # Ok(())
    /// # }
    /// ```
    ///
    /// # See Also
    /// <https://www.w3.org/TR/epub/#sec-meta-elem> for official EPUB `<meta>` documentation.
    #[non_exhaustive]
    Meta {
        /// Structural version associated with a `<meta>` element.
        ///
        /// # See Also
        /// - [`EpubMetaEntryKind::version`]
        version: EpubVersion,
    },

    /// A link element pertaining to a task such as resource associations,
    /// linkage to external resources, etc.
    ///
    /// **`<link>` element structure example**:
    /// ```xml
    /// <link rel="record" href="meta/133333333337.xml" media-type="application/marc" />
    /// ```
    ///
    /// # See Also
    /// <https://www.w3.org/TR/epub/#sec-link-elem> for official EPUB `<link>` documentation.
    #[non_exhaustive]
    Link {},
}

impl EpubMetaEntryKind {
    /// Returns `true` if the kind is [`Self::DublinCore`].
    ///
    /// # Examples
    /// - Assessing a Dublin Core (`<dc:*>`) element:
    /// ```
    /// # use rbook::Epub;
    /// # use rbook::epub::metadata::EpubMetaEntryKind;
    /// # fn main() -> rbook::ebook::errors::EbookResult<()> {
    /// let epub = Epub::open("tests/ebooks/example_epub")?;
    /// let title = epub.metadata().title().unwrap();
    /// let kind = title.as_meta().kind();
    ///
    /// assert!(matches!(kind, EpubMetaEntryKind::DublinCore { .. }));
    /// assert!(kind.is_dublin_core());
    ///
    /// // Kinds are mutually exclusive (Below will always resolve to `false`)
    /// assert!(!kind.is_meta());
    /// assert!(!kind.is_link());
    /// # Ok(())
    /// # }
    /// ```
    pub fn is_dublin_core(&self) -> bool {
        matches!(self, Self::DublinCore { .. })
    }

    /// Returns `true` if the kind is [`Self::Meta`].
    ///
    /// # See Also
    /// - [`Self::version`] for the structural version of a `<meta>` element.
    /// - [`Self::is_epub2_meta`] to check if the structural version is legacy EPUB 2.
    /// - [`Self::is_epub3_meta`] to check if the structural version is EPUB 3.
    ///
    /// # Examples
    /// - Assessing a `meta` element:
    /// ```
    /// # use rbook::Epub;
    /// # use rbook::epub::metadata::{EpubMetaEntryKind, EpubVersion};
    /// # fn main() -> rbook::ebook::errors::EbookResult<()> {
    /// let epub = Epub::open("tests/ebooks/example_epub")?;
    /// let cover_meta = epub.metadata().by_property("cover").next().unwrap();
    /// let kind = cover_meta.kind();
    ///
    /// assert!(matches!(kind, EpubMetaEntryKind::Meta { version: EpubVersion::EPUB2, .. }));
    /// assert!(kind.is_meta());
    ///
    /// // Kinds are mutually exclusive (Below will always resolve to `false`)
    /// assert!(!kind.is_dublin_core());
    /// assert!(!kind.is_link());
    /// # Ok(())
    /// # }
    /// ```
    pub fn is_meta(&self) -> bool {
        matches!(self, Self::Meta { .. })
    }

    /// Returns `true` if the kind is legacy EPUB 2 [`Self::Meta`].
    ///
    /// # See Also
    /// - [`Self::version`] for the structural version of a `<meta>` element.
    ///
    /// # Examples
    /// - Assessing a legacy EPUB 2 `meta` element:
    /// ```
    /// # use rbook::Epub;
    /// # use rbook::epub::metadata::{EpubMetaEntryKind, EpubVersion};
    /// # fn main() -> rbook::ebook::errors::EbookResult<()> {
    /// let epub = Epub::open("tests/ebooks/example_epub")?;
    /// let cover_meta = epub.metadata().by_property("cover").next().unwrap();
    /// let kind = cover_meta.kind();
    ///
    /// assert!(kind.is_meta());
    /// assert!(kind.is_epub2_meta());
    ///
    /// // Kinds are mutually exclusive (Below will always resolve to `false`)
    /// assert!(!kind.is_epub3_meta());
    /// assert!(!kind.is_dublin_core());
    /// assert!(!kind.is_link());
    /// # Ok(())
    /// # }
    /// ```
    pub fn is_epub2_meta(&self) -> bool {
        matches!(
            self,
            Self::Meta {
                version: EpubVersion::Epub2(_),
            }
        )
    }

    /// Returns `true` if the kind is EPUB 3 [`Self::Meta`].
    ///
    /// # See Also
    /// - [`Self::version`] for the structural version of a `<meta>` element.
    ///
    /// # Examples
    /// - Assessing an EPUB 3 `meta` element:
    /// ```
    /// # use rbook::Epub;
    /// # use rbook::epub::metadata::{EpubMetaEntryKind, EpubVersion};
    /// # fn main() -> rbook::ebook::errors::EbookResult<()> {
    /// let epub = Epub::open("tests/ebooks/example_epub")?;
    /// let title = epub.metadata().title().unwrap();
    ///
    /// // Retrieve the `title-type` refinement
    /// let refinement = title.refinements().by_property("title-type").next().unwrap();
    /// let kind = refinement.kind();
    ///
    /// assert!(kind.is_meta());
    /// assert!(kind.is_epub3_meta());
    ///
    /// // Kinds are mutually exclusive (Below will always resolve to `false`)
    /// assert!(!kind.is_epub2_meta());
    /// assert!(!kind.is_dublin_core());
    /// assert!(!kind.is_link());
    /// # Ok(())
    /// # }
    /// ```
    pub fn is_epub3_meta(&self) -> bool {
        matches!(
            self,
            Self::Meta {
                version: EpubVersion::Epub3(_),
            }
        )
    }

    /// Returns `true` if the kind is [`Self::Link`].
    ///
    /// # Examples
    /// - Assessing a `link` element:
    /// ```
    /// # use rbook::Epub;
    /// # use rbook::epub::metadata::EpubMetaEntryKind;
    /// # fn main() -> rbook::ebook::errors::EbookResult<()> {
    /// let epub = Epub::open("tests/ebooks/example_epub")?;
    /// let link = epub.metadata().by_id("example-link").unwrap();
    /// let kind = link.kind();
    ///
    /// assert!(matches!(kind, EpubMetaEntryKind::Link { .. }));
    /// assert!(kind.is_link());
    ///
    /// // Kinds are mutually exclusive (Below will always resolve to `false`)
    /// assert!(!kind.is_dublin_core());
    /// assert!(!kind.is_meta());
    /// # Ok(())
    /// # }
    /// ```
    pub fn is_link(&self) -> bool {
        matches!(self, Self::Link { .. })
    }

    /// The structural [`EpubVersion`] a metadata entry is associated with.
    ///
    /// # Note
    /// This method is applicable if [`EpubMetaEntryKind`] is [`EpubMetaEntryKind::Meta`].
    ///
    /// For all other kinds ([`DublinCore`](EpubMetaEntryKind::DublinCore),
    /// [`Link`](EpubMetaEntryKind::Link)),
    /// the returned version is **always** [`None`]
    /// as their main structure remains the same between EPUB 2 and 3.
    ///
    /// # See Also
    /// - [`Self::Meta`] for the structural differences between EPUB 2 and 3 `<meta>` elements.
    ///
    /// # Examples
    /// - Retrieving the structural version:
    /// ```
    /// # use rbook::Epub;
    /// # use rbook::epub::metadata::{EpubMetaEntryKind, EpubVersion};
    /// # fn main() -> rbook::ebook::errors::EbookResult<()> {
    /// let epub = Epub::open("tests/ebooks/example_epub")?;
    /// let metadata = epub.metadata();
    ///
    /// let cover = metadata.by_property("cover").next().unwrap();
    /// let title = metadata.title().unwrap();
    /// let link = metadata.by_id("example-link").unwrap();
    ///
    /// // `<meta>` elements always have a structural version
    /// assert_eq!(cover.kind().version(), Some(EpubVersion::EPUB2));
    ///
    /// // Dublin Core `<dc:*>` elements never have an associated structural version
    /// assert_eq!(title.as_meta().kind().version(), None);
    ///
    /// // `<link>` elements never have an associated structural version
    /// assert_eq!(link.kind().version(), None);
    /// # Ok(())
    /// # }
    /// ```
    pub fn version(&self) -> Option<EpubVersion> {
        match self {
            Self::Meta { version, .. } => Some(*version),
            _ => None,
        }
    }
}

/// The version of an [`Epub`](super::Epub).
///
/// # Versions
/// - [`Epub2`](EpubVersion::Epub2) (Legacy)
/// - [`Epub3`](EpubVersion::Epub3)
/// - [`Unknown`](EpubVersion::Unknown)
///
/// # See Also
/// - [`EpubMetadata::version_str`] for the original representation.
#[non_exhaustive]
#[derive(Copy, Clone, Debug, Hash, PartialEq, Eq)]
pub enum EpubVersion {
    /// [`Epub`](super::Epub) Version `2.*` **(Legacy)**
    Epub2(Version),
    /// [`Epub`](super::Epub) Version `3.*`
    ///
    /// Epubs with this version may be backwards compatible with version 2,
    /// `rbook` handles such scenarios behind-the-scenes.
    ///
    /// # See Also
    /// - [`EpubOpenOptions`](super::EpubOpenOptions) for preferences between versions 2 and 3.
    Epub3(Version),
    /// An unknown [`Epub`](super::Epub) version
    ///
    /// An [`Epub`](super::Epub) may contain this version when
    /// [`EpubOpenOptions::strict`](super::EpubOpenOptions::strict) is set to `false`.
    Unknown(Version),
}

impl EpubVersion {
    /// Utility to retrieve the major of each compatible EPUB versions.
    pub(crate) const VERSIONS: [EpubVersion; 2] = [Self::EPUB2, Self::EPUB3];

    /// [`EpubVersion::Epub2`] constant with a predefined [`Version`] of `2.0`.
    pub const EPUB2: Self = Self::Epub2(Version(2, 0));

    /// [`EpubVersion::Epub3`] constant with a predefined [`Version`] of `3.0`.
    pub const EPUB3: Self = Self::Epub3(Version(3, 0));

    /// Returns the major form of an epub version.
    ///
    /// If the contained [`Version`] is `3.3`, then the returned [`EpubVersion`]
    /// will have a contained value of `3.0`.
    ///
    /// # Examples
    /// - Retrieving the major:
    /// ```
    /// # use rbook::ebook::metadata::Version;
    /// # use rbook::ebook::epub::metadata::EpubVersion;
    /// let epub3 = EpubVersion::from(Version(3, 3));
    ///
    /// assert_eq!(Version(3, 3), epub3.version());
    /// assert_eq!(Version(3, 0), epub3.as_major().version());
    /// ```
    pub fn as_major(&self) -> Self {
        match self {
            Self::Epub2(_) => Self::EPUB2,
            Self::Epub3(_) => Self::EPUB3,
            Self::Unknown(version) => Self::Unknown(Version(version.0, 0)),
        }
    }

    /// The encapsulated version information.
    ///
    /// # Note
    /// If [`EpubOpenOptions::strict`](super::EpubOpenOptions::strict) is set to `false`,
    /// the returned [`Version`] may not be within the valid range: `2 <= version < 4`.
    pub fn version(&self) -> Version {
        match self {
            Self::Epub2(version) | Self::Epub3(version) | Self::Unknown(version) => *version,
        }
    }

    /// Returns `true` if the variant is [`EpubVersion::Epub2`].
    pub fn is_epub2(&self) -> bool {
        matches!(self, Self::Epub2(_))
    }

    /// Returns `true` if the variant is [`EpubVersion::Epub3`].
    pub fn is_epub3(&self) -> bool {
        matches!(self, Self::Epub3(_))
    }

    /// Returns `true` if the variant is [`EpubVersion::Unknown`].
    pub fn is_unknown(&self) -> bool {
        matches!(self, Self::Unknown(_))
    }
}

impl PartialOrd for EpubVersion {
    fn partial_cmp(&self, other: &Self) -> Option<std::cmp::Ordering> {
        Some(self.cmp(other))
    }
}

impl Ord for EpubVersion {
    fn cmp(&self, other: &Self) -> std::cmp::Ordering {
        self.version().cmp(&other.version())
    }
}

impl<I: Into<Version>> From<I> for EpubVersion {
    fn from(version: I) -> Self {
        let version = version.into();

        match version.0 {
            2 => Self::Epub2(version),
            3 => Self::Epub3(version),
            _ => Self::Unknown(version),
        }
    }
}

impl Display for EpubVersion {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        self.version().fmt(f)
    }
}

/// Implementation of [`Identifier`].
///
/// # See Also
/// - [`DetachedEpubMetaEntry::identifier`] to create an identifier metadata entry.
/// - [`Self::as_meta`] to access finer details such as attributes and refinements.
#[derive(Copy, Clone, Debug)]
pub struct EpubIdentifier<'ebook> {
    entry: EpubMetaEntry<'ebook>,
}

impl_meta_entry_abstraction!(EpubIdentifier);

impl<'ebook> EpubIdentifier<'ebook> {
    fn new(entry: EpubMetaEntry<'ebook>) -> Self {
        Self { entry }
    }

    /// Fallback when [`Self::get_modern_identifier_type`] is not available.
    fn get_legacy_identifier_type(&self) -> Option<Scheme<'ebook>> {
        self.entry
            .data
            .attributes
            .get_value(opf::OPF_SCHEME)
            .map(|identifier_type| Scheme::new(None, identifier_type))
    }

    fn get_modern_identifier_type(&self) -> Option<Scheme<'ebook>> {
        self.entry
            .data
            .refinements
            .get_schemes(opf::IDENTIFIER_TYPE)
            .next()
    }

    /// The identifier’s scheme or [`None`] if unspecified.
    #[doc = util::inherent_doc!(Identifier, scheme)]
    pub fn scheme(&self) -> Option<Scheme<'ebook>> {
        self.get_modern_identifier_type()
            .or_else(|| self.get_legacy_identifier_type())
    }
}

impl<'ebook> Identifier<'ebook> for EpubIdentifier<'ebook> {
    fn scheme(&self) -> Option<Scheme<'ebook>> {
        self.scheme()
    }
}

impl Eq for EpubIdentifier<'_> {}

impl PartialEq for EpubIdentifier<'_> {
    fn eq(&self, other: &Self) -> bool {
        self.scheme() == other.scheme() && self.value() == other.value()
    }
}

impl std::hash::Hash for EpubIdentifier<'_> {
    fn hash<H: std::hash::Hasher>(&self, state: &mut H) {
        self.value().hash(state);
        self.scheme().hash(state);
    }
}

/// Implementation of [`Title`].
///
/// # See Also
/// - [`DetachedEpubMetaEntry::title`] to create a title metadata entry.
/// - [`Self::as_meta`] to access finer details such as attributes and refinements.
#[derive(Copy, Clone, Debug)]
pub struct EpubTitle<'ebook> {
    entry: EpubMetaEntry<'ebook>,
    /// Whether the title is inferred as the main title.
    ///
    /// Based on the logic contained within [`Metadata::titles`],
    /// when a `title-type` of `main` is absent, infers the main `Title`
    /// by selecting the `<dc:title>` with the highest precedence (lowest display order).
    ///
    /// This guarantees consistent [`TitleKind::Main`] identification across all EPUBs via
    /// [`Title::kind`].
    is_main_title: bool,
}

impl_meta_entry_abstraction!(EpubTitle);

impl<'ebook> EpubTitle<'ebook> {
    fn new(entry: EpubMetaEntry<'ebook>, is_main_title: bool) -> Self {
        Self {
            entry,
            is_main_title,
        }
    }

    /// The title’s scheme or [`None`] if unspecified.
    #[doc = util::inherent_doc!(Title, scheme)]
    pub fn scheme(&self) -> Option<Scheme<'ebook>> {
        self.entry
            .data
            .refinements
            .by_refinement(opf::TITLE_TYPE)
            .map(|title_type| Scheme::new(None, &title_type.value))
    }

    /// The kind of title.
    #[doc = util::inherent_doc!(Title, kind)]
    pub fn kind(&self) -> TitleKind {
        if self.is_main_title {
            return TitleKind::Main;
        }
        self.entry
            .data
            .refinements
            .by_refinement(opf::TITLE_TYPE)
            .map_or(TitleKind::Unknown, |title_type| {
                TitleKind::from(&title_type.value)
            })
    }
}

impl<'ebook> Title<'ebook> for EpubTitle<'ebook> {
    fn scheme(&self) -> Option<Scheme<'ebook>> {
        self.scheme()
    }

    fn kind(&self) -> TitleKind {
        self.kind()
    }
}

impl PartialEq for EpubTitle<'_> {
    fn eq(&self, other: &Self) -> bool {
        self.entry == other.entry
    }
}

/// Implementation of [`Tag`].
///
/// # See Also
/// - [`DetachedEpubMetaEntry::tag`] to create a tag (`dc:subject`) metadata entry.
/// - [`Self::as_meta`] to access finer details such as attributes and refinements.
#[derive(Copy, Clone, Debug)]
pub struct EpubTag<'ebook> {
    entry: EpubMetaEntry<'ebook>,
}

impl_meta_entry_abstraction!(EpubTag);

impl<'ebook> EpubTag<'ebook> {
    fn new(entry: EpubMetaEntry<'ebook>) -> Self {
        Self { entry }
    }

    /// Fallback when [`Self::get_modern_scheme`] is not available.
    fn get_legacy_scheme(&self) -> Option<Scheme<'ebook>> {
        let refinements = &self.entry.data.refinements;
        let auth = refinements.by_refinement(opf::AUTHORITY)?;
        let term = refinements.by_refinement(opf::TERM)?;
        Some(Scheme::new(Some(&auth.value), &term.value))
    }

    fn get_modern_scheme(&self) -> Option<Scheme<'ebook>> {
        let attributes = &self.entry.data.attributes;
        let authority = attributes.get_value(opf::OPF_AUTHORITY)?;
        let term = attributes.get_value(opf::OPF_TERM)?;
        Some(Scheme::new(Some(authority), term))
    }

    /// The tag’s scheme or [`None`] if unspecified.
    ///
    /// The **authority** is the [`Scheme::source`]
    /// and the **term** is the [`Scheme::code`].
    #[doc = util::inherent_doc!(Tag, scheme)]
    pub fn scheme(&self) -> Option<Scheme<'ebook>> {
        // A `term` must be present if there's an `authority`.
        // Otherwise, the element does not follow spec/malformed.
        self.get_modern_scheme()
            .or_else(|| self.get_legacy_scheme())
    }
}

impl<'ebook> Tag<'ebook> for EpubTag<'ebook> {
    fn scheme(&self) -> Option<Scheme<'ebook>> {
        self.scheme()
    }
}

impl PartialEq for EpubTag<'_> {
    fn eq(&self, other: &Self) -> bool {
        self.entry == other.entry
    }
}

/// Implementation of [`Contributor`].
///
/// # marc:relators
/// While not guaranteed, if a [`role`](Contributor::roles)
/// has a [`Scheme::source`] of [`None`],
/// the value of [`Scheme::code`] may originate from `marc:relators` as the source.
/// This is typically the case for EPUB 2.
///
/// # See Also
/// - [`DetachedEpubMetaEntry::creator`] to create a **creator** metadata entry.
/// - [`DetachedEpubMetaEntry::contributor`] to create a **contributor** metadata entry.
/// - [`DetachedEpubMetaEntry::publisher`] to create a **publisher** metadata entry.
/// - [`Self::as_meta`] to access finer details such as attributes and refinements.
#[derive(Copy, Clone, Debug)]
pub struct EpubContributor<'ebook> {
    entry: EpubMetaEntry<'ebook>,
}

impl_meta_entry_abstraction!(EpubContributor);

impl<'ebook> EpubContributor<'ebook> {
    fn new(entry: EpubMetaEntry<'ebook>) -> Self {
        Self { entry }
    }

    /// Fallback when [`Self::get_modern_roles`] is not available.
    fn get_legacy_role(&self) -> Option<Scheme<'ebook>> {
        self.entry
            .data
            .attributes
            .by_name(opf::OPF_ROLE)
            .map(|role| Scheme::new(None, role.value()))
    }

    fn get_modern_roles(&self) -> impl Iterator<Item = Scheme<'ebook>> + 'ebook {
        self.entry.data.refinements.get_schemes(opf::ROLE)
    }

    /// The primary role of a contributor or [`None`] if unspecified.
    #[doc = util::inherent_doc!(Contributor, main_role)]
    pub fn main_role(&self) -> Option<Scheme<'ebook>> {
        self.roles().next()
    }

    /// Returns an iterator over **all** roles by the order of importance (display sequence).
    #[doc = util::inherent_doc!(Contributor, roles)]
    pub fn roles(&self) -> impl Iterator<Item = Scheme<'ebook>> + 'ebook {
        let roles = self.get_modern_roles().map(Some);
        // If the size hint is 0, attempt to retrieve legacy `opf:role` attribute
        // Note: `size_hint` **works** here as the underlying source is based
        // on a collection (Vec) with a reliable hint.
        let fallback = (roles.size_hint().0 == 0).then(|| self.get_legacy_role());

        roles.chain(std::iter::once(fallback.flatten())).flatten()
    }
}

impl<'ebook> Contributor<'ebook> for EpubContributor<'ebook> {
    fn main_role(&self) -> Option<Scheme<'ebook>> {
        self.main_role()
    }

    fn roles(&self) -> impl Iterator<Item = Scheme<'ebook>> + 'ebook {
        self.roles()
    }
}

impl PartialEq for EpubContributor<'_> {
    fn eq(&self, other: &Self) -> bool {
        self.entry == other.entry
    }
}

/// Implementation of [`Language`].
///
/// EPUB language tags are always treated as `BCP 47` (EPUB 3),
/// even when originating from `RFC 3066` (EPUB 2) as:
/// 1. EPUB 3 requires the language scheme as BCP 47.
/// 2. EPUB 2 requires a subset of BCP 47, RFC 3066.
///
/// For simplicity, `rbook` treats RFC 3066 ***as*** BCP 47.
/// Both [`EpubLanguage::scheme`] and [`EpubLanguage::kind`]
/// will always report `BCP 47`.
///
/// # See Also
/// - [`DetachedEpubMetaEntry::language`] to create a language metadata entry.
/// - [`Self::as_meta`] to access finer details such as attributes and refinements.
#[derive(Copy, Clone, Debug)]
pub struct EpubLanguage<'ebook> {
    entry: EpubMetaEntry<'ebook>,
}

impl_meta_entry_abstraction!(EpubLanguage);

impl<'ebook> EpubLanguage<'ebook> {
    fn new(entry: EpubMetaEntry<'ebook>) -> Self {
        Self { entry }
    }
}

impl<'ebook> EpubLanguage<'ebook> {
    /// Always returns with [`Scheme::source`] always exactly as `BCP 47`.
    ///
    /// See [`EpubLanguage`] for more information.
    #[doc = util::inherent_doc!(Language, scheme)]
    /// # See Also
    /// - [`Language::scheme`]
    pub fn scheme(&self) -> Scheme<'ebook> {
        Scheme::new(Some(LanguageKind::Bcp47.as_str()), &self.entry.data.value)
    }

    /// Always returns [`LanguageKind::Bcp47`].
    ///
    /// See [`EpubLanguage`] for more information.
    #[doc = util::inherent_doc!(Language, kind)]
    /// # See Also
    /// - [`Language::kind`]
    pub fn kind(&self) -> LanguageKind {
        // Normalize both as BCP 47:
        // - EPUB-2 requires RFC 3066 (a subset of BCP 47)
        // - EPUB-3 requires BCP 47
        LanguageKind::Bcp47
    }
}

impl<'ebook> Language<'ebook> for EpubLanguage<'ebook> {
    fn scheme(&self) -> Scheme<'ebook> {
        self.scheme()
    }

    fn kind(&self) -> LanguageKind {
        self.kind()
    }
}

impl PartialEq for EpubLanguage<'_> {
    fn eq(&self, other: &Self) -> bool {
        self.entry == other.entry
    }
}

mod macros {
    macro_rules! impl_meta_entry_abstraction {
        ($implementation:ident) => {
            impl<'ebook> $implementation<'ebook> {
                // `data` and `index` provide a common accessor between
                // `EpubMetaEntry` and more specialized views (e.g., EpubContributor).
                //
                // Mainly assists within the `impl_meta_entry` macro.
                fn data(&self) -> &'ebook EpubMetaEntryData {
                    &self.entry.data
                }

                fn index(&self) -> usize {
                    self.entry.index
                }

                /// Returns the [`EpubMetaEntry`] form to access additional
                /// metadata entry details, such as attributes and refinements.
                pub fn as_meta(&self) -> EpubMetaEntry<'ebook> {
                    self.entry
                }
            }

            impl PartialEq<EpubMetaEntry<'_>> for $implementation<'_> {
                fn eq(&self, other_entry: &EpubMetaEntry<'_>) -> bool {
                    &self.entry == other_entry
                }
            }

            impl<'ebook> std::ops::Deref for $implementation<'ebook> {
                type Target = EpubMetaEntry<'ebook>;

                fn deref(&self) -> &Self::Target {
                    &self.entry
                }
            }

            impl_meta_entry!($implementation);
        };
    }

    /// Implements [`MetaEntry`](super::MetaEntry) for the specified type.
    macro_rules! impl_meta_entry {
        ($implementation:ident) => {
            impl<'ebook> $implementation<'ebook> {
                fn get_modern_alt_script(
                    &self,
                ) -> impl Iterator<Item = AlternateScript<'ebook>> + 'ebook {
                    self.data()
                        .refinements
                        .by_refinements(opf::ALTERNATE_SCRIPT)
                        .map(|(_, script)| {
                            // xml:lang **should** be present on an alternate-script refinement.
                            // Otherwise, the EPUB is malformed.
                            let code = script.language().unwrap_or_default();
                            AlternateScript::new(
                                &script.value,
                                LanguageTag::new(code, LanguageKind::Bcp47),
                            )
                        })
                }

                fn get_legacy_alt_script(&self) -> Option<AlternateScript<'ebook>> {
                    let attributes = &self.data().attributes;
                    let script = attributes.by_name(opf::OPF_ALT_REP)?.value();
                    let code = attributes.by_name(opf::OPF_ALT_REP_LANG)?.value();
                    Some(AlternateScript::new(
                        script,
                        LanguageTag::new(code, LanguageKind::Bcp47),
                    ))
                }

                /// The plain text value of an entry.
                #[doc = util::inherent_doc!(MetaEntry, value)]
                pub fn value(&self) -> &'ebook str {
                    &self.data().value
                }

                /// The (0-based) order/display-sequence of an entry relative to another associated entry.
                #[doc = util::inherent_doc!(MetaEntry, order)]
                pub fn order(&self) -> usize {
                    self.index()
                }

                /// The `file-as` sort key, if present.
                #[doc = util::inherent_doc!(MetaEntry, file_as)]
                pub fn file_as(&self) -> Option<&'ebook str> {
                    self.data()
                        .refinements
                        .by_refinement(opf::FILE_AS)
                        .map(|refinement| refinement.value.as_str())
                        // Fallback to legacy `opf:file-as` attribute
                        .or_else(|| {
                            self.data()
                                .attributes
                                .by_name(opf::OPF_FILE_AS)
                                .map(|attribute| attribute.value())
                        })
                }

                /// Returns an iterator over **all** [`AlternateScript`].
                #[doc = util::inherent_doc!(MetaEntry, alternate_scripts)]
                pub fn alternate_scripts(
                    &self,
                ) -> impl Iterator<Item = AlternateScript<'ebook>> + 'ebook {
                    let mut scripts = self.get_modern_alt_script().peekable();

                    let fallback = scripts
                        .peek()
                        .is_none()
                        .then(|| self.get_legacy_alt_script())
                        .flatten();

                    scripts.chain(std::iter::once(fallback).flatten())
                }
            }

            impl Sealed for $implementation<'_> {}

            impl<'ebook> MetaEntry<'ebook> for $implementation<'ebook> {
                fn value(&self) -> &'ebook str {
                    self.value()
                }

                fn order(&self) -> usize {
                    self.order()
                }

                fn file_as(&self) -> Option<&'ebook str> {
                    self.file_as()
                }

                fn alternate_scripts(
                    &self,
                ) -> impl Iterator<Item = AlternateScript<'ebook>> + 'ebook {
                    self.alternate_scripts()
                }
            }
        };
    }

    pub(super) use {impl_meta_entry, impl_meta_entry_abstraction};
}
