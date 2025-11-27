//! EPUB metadata-related content.

use crate::ebook::element::{AttributeData, Attributes, Href, Name, Properties, TextDirection};
use crate::ebook::epub::consts;
use crate::ebook::epub::metadata::macros::{impl_meta_entry, impl_meta_entry_abstraction};
use crate::ebook::metadata::{AlternateScript, DateTime, Scheme, Version};
use crate::ebook::metadata::{
    Contributor, Identifier, Language, LanguageKind, LanguageTag, MetaEntry, Tag, Title, TitleKind,
};
use crate::ebook::resource::ResourceKind;
use crate::ebook::{Metadata, element};
use crate::util::sync::Shared;
use std::collections::HashMap;
use std::fmt::{Display, Formatter};
use std::hash::{Hash, Hasher};
use std::iter;
use std::ops::{Deref, DerefMut};
use std::slice::Iter as SliceIter;

////////////////////////////////////////////////////////////////////////////////
// PRIVATE API
////////////////////////////////////////////////////////////////////////////////

pub(super) type EpubMetaGroups = HashMap<String, Vec<EpubMetaEntryData>>;

#[derive(Debug)]
pub(super) struct EpubMetadataData {
    /// The `id` of the primary unique identifier.
    /// Retrieved from the `<package>` element by the `unique-identifier` attribute.
    primary_identifier: String,
    version: EpubVersionData,
    entries: EpubMetaGroups,
}

impl EpubMetadataData {
    pub(super) fn new(
        primary_identifier: String,
        version: EpubVersionData,
        entries: EpubMetaGroups,
    ) -> Self {
        Self {
            primary_identifier,
            version,
            entries,
        }
    }

    pub(super) fn by_group_mut(&mut self, group: &str) -> Option<&mut Vec<EpubMetaEntryData>> {
        self.entries.get_mut(group)
    }
}

/// Contains the raw and parsed epub version
#[derive(Debug)]
pub(super) struct EpubVersionData {
    pub(super) raw: String,
    pub(super) parsed: EpubVersion,
}

#[derive(Debug, Default, Hash, PartialEq, Eq)]
pub(super) struct EpubRefinementsData(Vec<EpubMetaEntryData>);

impl EpubRefinementsData {
    pub(crate) fn new(refinements: Vec<EpubMetaEntryData>) -> Self {
        Self(refinements)
    }

    fn get_schemes(&self, key: &str) -> impl Iterator<Item = Scheme<'_>> {
        self.by_refinements(key).map(|key_item| {
            // Note: There is usually a `scheme` associated with the `value`,
            // although it's not always guaranteed
            let scheme = element::get_attribute(&key_item.attributes, consts::SCHEME);
            Scheme::new(scheme, &key_item.value)
        })
    }

    fn by_refinements(&self, property: &str) -> impl Iterator<Item = &EpubMetaEntryData> {
        self.0
            .iter()
            .filter(move |refinement| refinement.property == property)
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

impl Deref for EpubRefinementsData {
    type Target = Vec<EpubMetaEntryData>;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl DerefMut for EpubRefinementsData {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.0
    }
}

#[derive(Debug, PartialEq, Eq, Hash)]
pub(super) struct EpubMetaEntryData {
    pub(super) order: usize,
    pub(super) id: Option<String>,
    pub(super) refines: Option<String>,
    pub(super) property: String,
    pub(super) value: String,
    pub(super) language: Option<Shared<String>>,
    pub(super) text_direction: TextDirection,
    pub(super) attributes: Vec<AttributeData>,
    pub(super) refinements: EpubRefinementsData,
    pub(super) kind: EpubMetaEntryKind,
}

impl Default for EpubMetaEntryData {
    fn default() -> Self {
        Self {
            order: 0,
            id: None,
            refines: None,
            property: String::new(),
            value: String::new(),
            language: None,
            text_direction: TextDirection::Auto,
            attributes: Vec::new(),
            refinements: EpubRefinementsData::default(),
            // This is merely a placeholder
            kind: EpubMetaEntryKind::DublinCore {},
        }
    }
}

impl EpubMetaEntryData {
    fn language(&self) -> Option<&str> {
        self.language.as_ref().map(|language| language.as_str())
    }

    fn attributes(&self) -> Attributes<'_> {
        (&self.attributes).into()
    }

    /// Recursively search for an entry based on a predicate.
    fn by_predicate(&self, predicate: impl Copy + Fn(&Self) -> bool) -> Option<&Self> {
        if predicate(self) {
            return Some(self);
        }

        for refinement in &self.refinements.0 {
            if let Some(found) = refinement.by_predicate(predicate) {
                return Some(found);
            }
        }
        None
    }
}

////////////////////////////////////////////////////////////////////////////////
// PUBLIC API
////////////////////////////////////////////////////////////////////////////////

/// EPUB metadata, see [`Metadata`] for more details.
///
/// # Behavior
/// `rbook` supports arbitrary refinements for any metadata element,
/// including `item` and `itemref` elements.
/// Refinement chains of arbitrary length are supported as well.
///
/// Elements and id references within the `<package>` element are **not** required
/// to be in chronological order.
/// Meaning, refinements may come before parent elements.
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
#[derive(Copy, Clone, Debug)]
pub struct EpubMetadata<'ebook> {
    data: &'ebook EpubMetadataData,
}

impl<'ebook> EpubMetadata<'ebook> {
    pub(super) fn new(data: &'ebook EpubMetadataData) -> Self {
        Self { data }
    }

    fn data_by_property(
        &self,
        property: &str,
    ) -> impl Iterator<Item = &'ebook EpubMetaEntryData> + 'ebook {
        self.data
            .entries
            .get(property)
            .map_or::<&[_], _>(&[], Vec::as_slice)
            .iter()
    }

    /// Returns an iterator over non-refining [`entries`](EpubMetaEntry) whose
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
    /// <meta refines="#parent-id">...</meta>
    /// ```
    ///
    /// # See Also
    /// - [`Self::entries`]
    pub fn by_property(
        &self,
        property: &str,
    ) -> impl Iterator<Item = EpubMetaEntry<'ebook>> + 'ebook {
        self.data_by_property(property).map(EpubMetaEntry::new)
    }

    /// Searches the entire metadata hierarchy, including refinements, and returns the
    /// [`EpubMetaEntry`] that matches the given `id` if present, otherwise [`None`].
    ///
    /// # Performance Implications
    /// This is a linear operation (`O(N)`) as the underlying source
    /// is ***not*** a hashmap with `id` as the key.
    /// Generally, the number of metadata entries is small (<20).
    /// However, for larger sets, frequent lookups can impede performance.
    ///
    /// # Examples
    /// - Retrieving a creator by id:
    /// ```
    /// # use rbook::ebook::errors::EbookResult;
    /// # use rbook::ebook::toc::{Toc, TocEntryKind};
    /// # use rbook::ebook::metadata::Metadata;
    /// # use rbook::{Ebook, Epub};
    /// # fn main() -> EbookResult<()> {
    /// let epub = Epub::open("tests/ebooks/example_epub")?;
    /// let metadata = epub.metadata();
    ///
    /// // Retrieve the author
    /// let author = metadata.creators().next().unwrap();
    /// // Retrieve the same entry by id
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
        self.data
            .entries
            .values()
            .flatten()
            .find_map(|parent| parent.by_predicate(|data| data.id.as_deref() == Some(id)))
            .map(EpubMetaEntry::new)
    }

    /// The [`Epub`](super::Epub) version (e.g., `2.0`, `3.2`, etc.).
    ///
    /// The returned version may be [`EpubVersion::Unknown`] if
    /// [`EpubSettings::strict`](super::EpubSettings::strict) is disabled.
    ///
    /// See [`EpubMetadata::version_str`] for the original representation.
    pub fn version(&self) -> EpubVersion {
        self.data.version.parsed
    }

    /// The underlying [`Epub`](super::Epub) version string.
    pub fn version_str(&self) -> &'ebook str {
        self.data.version.raw.as_str()
    }

    /// Returns an iterator over non-refining link entries.
    /// Only entries which have a [`kind`](EpubMetaEntry::kind) of [`EpubMetaEntryKind::Link`]
    /// are included in the iterator.
    ///
    /// # Excluded Entries
    /// Refining entries, `<link>` elements with a `refines` field are excluded, for example:
    /// ```xml
    /// <link refines="#parent-id" ...>...</meta>
    /// ```
    ///
    /// # See Also
    /// - [`Self::entries`] for iterating over **non-refining** metadata entries, including links.
    pub fn links(&self) -> impl Iterator<Item = EpubLink<'ebook>> + 'ebook {
        self.entries().filter_map(|entry| entry.as_link())
    }
}

#[allow(refining_impl_trait)]
impl<'ebook> Metadata<'ebook> for EpubMetadata<'ebook> {
    fn version_str(&self) -> Option<&'ebook str> {
        Some(self.version_str())
    }

    fn version(&self) -> Option<Version> {
        Some(self.data.version.parsed.version())
    }

    fn publication_date(&self) -> Option<DateTime<'ebook>> {
        self.data_by_property(consts::DATE)
            .next()
            .map(|publication_date| DateTime::new(&publication_date.value))
    }

    fn modified_date(&self) -> Option<DateTime<'ebook>> {
        self.data_by_property(consts::MODIFIED)
            .next()
            .map(|modified_date| DateTime::new(&modified_date.value))
    }

    /// The primary unique [`identifier`](EpubIdentifier) of an [`Epub`](super::Epub).
    ///
    /// Returns [`None`] if there is no primary unique identifier when
    /// [`EpubSettings::strict`](super::EpubSettings::strict) is disabled.
    ///
    /// As EPUBs may include multiple identifiers (`<dc:identifier>` elements),
    /// this method is for retrieval of the primary identifier as declared in
    /// the `<package>` element by the `unique-identifier` attribute.
    fn identifier(&self) -> Option<EpubIdentifier<'ebook>> {
        self.data.entries.get(consts::IDENTIFIER).and_then(|group| {
            group
                .iter()
                .find(|data| data.id.as_ref() == Some(&self.data.primary_identifier))
                .map(EpubIdentifier::new)
        })
    }

    fn identifiers(&self) -> impl Iterator<Item = EpubIdentifier<'ebook>> + 'ebook {
        self.data_by_property(consts::IDENTIFIER)
            .map(EpubIdentifier::new)
    }

    /// The main [`language`](EpubLanguage) of an [`Epub`](super::Epub).
    ///
    /// Returns [`None`] if there is no language specified when
    /// [`EpubSettings::strict`](super::EpubSettings::strict) is disabled.
    fn language(&self) -> Option<EpubLanguage<'ebook>> {
        self.languages().next()
    }

    fn languages(&self) -> impl Iterator<Item = EpubLanguage<'ebook>> + 'ebook {
        self.data_by_property(consts::LANGUAGE)
            .map(EpubLanguage::new)
    }

    /// The main [`title`](EpubTitle) of an [`Epub`](super::Epub).
    ///
    /// Returns [`None`] if there is no title specified when
    /// [`EpubSettings::strict`](super::EpubSettings::strict) is disabled.
    fn title(&self) -> Option<EpubTitle<'ebook>> {
        self.titles().find(|title| title.is_main_title)
    }

    fn titles(&self) -> impl Iterator<Item = EpubTitle<'ebook>> + 'ebook {
        // Although caching the inferred main title is possible,
        // this is generally inexpensive (nearly all publications only have one title),
        // and caching would complicate future write-back of EPUB metadata.
        let inferred_main_title_index = self
            .data_by_property(consts::TITLE)
            .enumerate()
            // First, try to find if a main `title-type` exists
            .find_map(|(i, title)| {
                title
                    .refinements
                    .by_refinement(consts::TITLE_TYPE)
                    .is_some_and(|title_type| title_type.value == consts::MAIN_TITLE_TYPE)
                    .then_some(i)
            })
            // If not, retrieve the first title
            .unwrap_or(0);

        self.data_by_property(consts::TITLE)
            .enumerate()
            .map(move |(i, data)| EpubTitle::new(data, i == inferred_main_title_index))
    }

    fn description(&self) -> Option<EpubMetaEntry<'ebook>> {
        self.descriptions().next()
    }

    fn descriptions(&self) -> impl Iterator<Item = EpubMetaEntry<'ebook>> + 'ebook {
        self.data_by_property(consts::DESCRIPTION)
            .map(EpubMetaEntry::new)
    }

    fn creators(&self) -> impl Iterator<Item = EpubContributor<'ebook>> + 'ebook {
        self.data_by_property(consts::CREATOR)
            .map(EpubContributor::new)
    }

    fn contributors(&self) -> impl Iterator<Item = EpubContributor<'ebook>> + 'ebook {
        self.data_by_property(consts::CONTRIBUTOR)
            .map(EpubContributor::new)
    }

    fn publishers(&self) -> impl Iterator<Item = EpubContributor<'ebook>> + 'ebook {
        self.data_by_property(consts::PUBLISHER)
            .map(EpubContributor::new)
    }

    fn tags(&self) -> impl Iterator<Item = EpubTag<'ebook>> + 'ebook {
        self.data_by_property(consts::SUBJECT).map(EpubTag::new)
    }

    /// Returns an iterator over non-refining metadata entries.
    ///
    /// Each entry is first grouped by its [`property`](Self::by_property) then
    /// [`order`](MetaEntry::order), before being flattened into a single iterator.
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
    /// <meta refines="#parent-id" ...>...</meta>
    /// <link refines="#parent-id" ...>...</meta>
    /// ```
    ///
    /// # See Also
    /// - [`Self::by_property`] for iterating over **non-refining** metadata entries by property.
    /// - [`Self::links`] for iterating over **non-refining** link entries.
    ///
    /// # Examples
    /// - Iterating over metadata entries:
    /// ```
    /// # use rbook::ebook::errors::EbookResult;
    /// # use rbook::ebook::metadata::Metadata;
    /// # use rbook::{Ebook, Epub};
    /// # fn main() -> EbookResult<()> {
    /// let epub = Epub::open("tests/ebooks/example_epub")?;
    ///
    /// // Iterate over all non-refining metadata elements
    /// for entry in epub.metadata().entries() {
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
    fn entries(&self) -> impl Iterator<Item = EpubMetaEntry<'ebook>> + 'ebook {
        self.data.entries.values().flatten().map(EpubMetaEntry::new)
    }
}

impl PartialEq for EpubMetadata<'_> {
    fn eq(&self, other: &Self) -> bool {
        self.version() == other.version() && self.identifier() == other.identifier()
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
#[derive(Copy, Clone, Debug, Hash, PartialEq, Eq)]
pub struct EpubRefinements<'ebook>(&'ebook EpubRefinementsData);

impl<'ebook> EpubRefinements<'ebook> {
    /// The number of refinement entries contained within.
    pub fn len(&self) -> usize {
        self.0.len()
    }

    /// Returns `true` if there are no refinements.
    pub fn is_empty(&self) -> bool {
        self.0.is_empty()
    }

    /// Returns the associated [`EpubMetaEntry`] if the provided `index` is less than
    /// [`Self::len`], otherwise [`None`].
    pub fn get(&self, index: usize) -> Option<EpubMetaEntry<'ebook>> {
        self.0.get(index).map(EpubMetaEntry::new)
    }

    /// Returns an iterator over **all** refining [`EpubMetaEntry`] entries.
    pub fn iter(&self) -> EpubRefinementsIter<'ebook> {
        self.into_iter()
    }

    /// Returns an iterator over **all** refinements that match the specified `property`.
    ///
    /// # Examples
    /// - Retrieving the `role` refinement for a creator:
    /// ```
    /// # use rbook::ebook::errors::EbookResult;
    /// # use rbook::ebook::metadata::{Metadata, MetaEntry};
    /// # use rbook::{Ebook, Epub};
    /// # fn main() -> EbookResult<()> {
    /// let epub = Epub::open("tests/ebooks/example_epub")?;
    /// let author = epub.metadata().creators().next().unwrap();
    /// let role = author.as_meta().refinements().by_property("role").next().unwrap();
    ///
    /// assert_eq!("aut", role.value());
    /// # Ok(())
    /// # }
    /// ```
    pub fn by_property(
        &self,
        property: &'ebook str,
    ) -> impl Iterator<Item = EpubMetaEntry<'ebook>> + 'ebook {
        self.0.by_refinements(property).map(EpubMetaEntry::new)
    }

    /// Returns `true` if the `property` is present.
    ///
    /// # Examples
    /// - Checking if a refinement exists:
    /// ```
    /// # use rbook::ebook::errors::EbookResult;
    /// # use rbook::ebook::metadata::{Metadata, MetaEntry};
    /// # use rbook::{Ebook, Epub};
    /// # fn main() -> EbookResult<()> {
    /// let epub = Epub::open("tests/ebooks/example_epub")?;
    /// let author = epub.metadata().creators().next().unwrap();
    /// let refinements = author.as_meta().refinements();
    ///
    /// assert!(refinements.has_property("file-as"));
    /// # Ok(())
    /// # }
    /// ```
    pub fn has_property(&self, property: &str) -> bool {
        self.0.has_refinement(property)
    }
}

impl<'ebook> From<&'ebook EpubRefinementsData> for EpubRefinements<'ebook> {
    fn from(refinements: &'ebook EpubRefinementsData) -> Self {
        Self(refinements)
    }
}

impl<'ebook> IntoIterator for &EpubRefinements<'ebook> {
    type Item = EpubMetaEntry<'ebook>;
    type IntoIter = EpubRefinementsIter<'ebook>;

    fn into_iter(self) -> Self::IntoIter {
        EpubRefinementsIter(self.0.iter())
    }
}

impl<'ebook> IntoIterator for EpubRefinements<'ebook> {
    type Item = EpubMetaEntry<'ebook>;
    type IntoIter = EpubRefinementsIter<'ebook>;

    fn into_iter(self) -> Self::IntoIter {
        (&self).into_iter()
    }
}

/// An iterator over all [`entries`](EpubMetaEntry) within [`EpubRefinements`].
///
/// # See Also
/// - [`EpubRefinements::iter`]
///
/// # Examples
/// - Iterating over all refinements of a metadata entry:
/// ```
/// # use rbook::ebook::errors::EbookResult;
/// # use rbook::{Ebook, Epub};
/// # fn main() -> EbookResult<()> {
/// let epub = Epub::open("tests/ebooks/example_epub")?;
/// let source = epub.metadata().by_property("dc:source").next().unwrap();
///
/// for refinement in source.refinements() {
///     // process refinement //
/// }
/// # Ok(())
/// # }
/// ```
pub struct EpubRefinementsIter<'ebook>(SliceIter<'ebook, EpubMetaEntryData>);

impl<'ebook> Iterator for EpubRefinementsIter<'ebook> {
    type Item = EpubMetaEntry<'ebook>;

    fn next(&mut self) -> Option<Self::Item> {
        self.0.next().map(EpubMetaEntry::new)
    }
}

/// A [`MetaEntry`] within [`EpubMetadata`].
///
/// A metadata entry is an element that appears within `<metadata>`
/// inside an EPUB's package.opf file, specifically:
/// - Dublin core elements: `<dc:*>`
/// - `meta` elements: `<meta>`
/// - `link` elements: `<link>`
///
/// # See Also
/// - [`EpubMetaEntryKind`] for more info regarding the different kinds of metadata entries.
/// - [`EpubLink`] for metadata regarding potentially linked external content.
#[derive(Copy, Clone, Debug, PartialEq)]
pub struct EpubMetaEntry<'ebook> {
    data: &'ebook EpubMetaEntryData,
}

impl_meta_entry!(EpubMetaEntry);

impl<'ebook> EpubMetaEntry<'ebook> {
    fn new(data: &'ebook EpubMetaEntryData) -> Self {
        Self { data }
    }

    /// The unique `id` of a metadata entry.
    pub fn id(&self) -> Option<&'ebook str> {
        self.data.id.as_deref()
    }

    /// The `id` a metadata entry refines.
    ///
    /// Returns [`Some`] if an entry is refining another, otherwise [`None`].
    pub fn refines(&self) -> Option<&'ebook str> {
        self.data.refines.as_deref()
    }

    /// The `property`, such as `dc:title`, `media:duration`, `file-as`, etc.
    ///
    /// # Property Mapping
    /// Depending on the `XML` element type and EPUB version, this field may be mapped
    /// differently:
    ///
    /// | `XML` Element Type   | Mapped From                                                        |
    /// |----------------------|--------------------------------------------------------------------|
    /// | Dublin Core `<dc:*>` | element tag (`<dc:title>...</dc:title>`)                           |
    /// | EPUB 2 `<meta>`      | `name` attribute (`<meta name="cover" content="..."/>`)            |
    /// | EPUB 3 `<meta>`      | `property` attribute (`<meta property="media:duration">...</meta>`)|
    pub fn property(&self) -> Name<'ebook> {
        self.data.property.as_str().into()
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
    /// # use rbook::ebook::errors::EbookResult;
    /// # use rbook::ebook::metadata::Metadata;
    /// # use rbook::{Ebook, Epub};
    /// # fn main() -> EbookResult<()> {
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
    /// # use rbook::ebook::errors::EbookResult;
    /// # use rbook::ebook::metadata::Metadata;
    /// # use rbook::{Ebook, Epub};
    /// # fn main() -> EbookResult<()> {
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
    /// # use rbook::ebook::errors::EbookResult;
    /// # use rbook::ebook::metadata::Metadata;
    /// # use rbook::{Ebook, Epub};
    /// # fn main() -> EbookResult<()> {
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
        let attributes = self.data.attributes();

        if let Some(attribute) = attributes.by_name(consts::OPF_SCHEME) {
            return Scheme::new(None, attribute.value());
        }

        Scheme::new(
            attributes
                .by_name(consts::SCHEME)
                .map(|attribute| attribute.value()),
            self.value(),
        )
    }

    /// The specified or inherited language in `BCP 47` format.
    ///
    /// [`None`] is returned if the element contains no `xml:lang` attribute
    /// (and the `<package>` has none).
    ///
    /// # See Also
    /// - [`EpubLink::href_lang`] for the language of the linked resource.
    ///   `<link>` elements do not formally support the `xml:lang` attribute.
    pub fn language(&self) -> Option<LanguageTag<'ebook>> {
        self.data
            .language()
            .map(|code| LanguageTag::new(code, LanguageKind::Bcp47))
    }

    /// The specified or inherited text direction (`ltr`, `rtl`, or `auto`).
    ///
    /// [`TextDirection::Auto`] is returned if any of the following conditions are met:
    /// - The `<package>` and specified element contains no `dir` attribute.
    /// - [`Self::kind`] is [`EpubMetaEntryKind::Link`], as `<link>`
    ///   elements do not formally support the `dir` attribute.
    pub fn text_direction(&self) -> TextDirection {
        self.data.text_direction
    }

    /// All additional `XML` [`Attributes`].
    ///
    /// # Omitted Attributes
    /// The following attributes will **not** be found within the returned collection:
    /// - [`id`](Self::id)
    /// - [`xml:lang`](Self::language)
    /// - [`dir`](Self::text_direction)
    /// - [`property`](Self::property)
    /// - [`refines`](Self::refines)
    /// - [`name`](Self::property) (EPUB 2; legacy)
    /// - [`content`](Self::value) (EPUB 2; legacy)
    pub fn attributes(&self) -> Attributes<'ebook> {
        (&self.data.attributes).into()
    }

    /// Complementary refinement metadata entries.
    pub fn refinements(&self) -> EpubRefinements<'ebook> {
        (&self.data.refinements).into()
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
    /// - [`EpubMetadata::links`] for iterating over non-refining link entries.
    ///
    /// # Examples
    /// - Converting to [`EpubLink`]:
    /// ```
    /// # use rbook::ebook::element::Href;
    /// # use rbook::ebook::errors::EbookResult;
    /// # use rbook::ebook::metadata::Metadata;
    /// # use rbook::{Ebook, Epub};
    /// # fn main() -> EbookResult<()> {
    /// let epub = Epub::open("tests/ebooks/example_epub")?;
    /// let meta_entry = epub.metadata().by_id("example-link").unwrap();
    ///
    /// // Convert to link view
    /// let link = meta_entry.as_link().unwrap();
    /// let href = link.href().unwrap();
    ///
    /// assert_eq!(href.as_str(), "https://github.com/devinsterling/rbook");
    /// # Ok(())
    /// # }
    /// ```
    pub fn as_link(&self) -> Option<EpubLink<'ebook>> {
        self.kind()
            .is_link()
            .then_some(EpubLink { data: self.data })
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
/// # use rbook::ebook::errors::EbookResult;
/// # use rbook::ebook::metadata::Metadata;
/// # use rbook::{Ebook, Epub};
/// # fn main() -> EbookResult<()> {
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
/// - [`EpubMetaEntry::as_link`] for converting from [`EpubMetaEntry`].
/// - <https://www.w3.org/TR/epub/#sec-link-elem> for official EPUB `<link>` documentation
pub struct EpubLink<'ebook> {
    data: &'ebook EpubMetaEntryData,
}

impl<'ebook> EpubLink<'ebook> {
    /// The location of the specified resource a link points to.
    ///
    /// Returns [`None`] if not present.
    pub fn href(&self) -> Option<Href<'ebook>> {
        self.data
            .attributes()
            .by_name(consts::HREF)
            .map(|attribute| attribute.value().into())
    }

    /// The language of the resource referenced by [`Self::href`].
    ///
    /// Returns [`None`] if not present.
    pub fn href_lang(&self) -> Option<LanguageTag<'ebook>> {
        self.data
            .attributes()
            .by_name(consts::HREFLANG)
            .map(|attribute| LanguageTag::new(attribute.value(), LanguageKind::Bcp47))
    }

    /// The **non-capitalized** `MIME` identifying the media type
    /// of the resource referenced by [`Self::href`].
    ///
    /// Returns [`None`] if not present.
    ///
    /// This method is a lower-level call than [`Self::resource_kind`].
    pub fn media_type(&self) -> Option<&'ebook str> {
        self.data
            .attributes()
            .by_name(consts::MEDIA_TYPE)
            .map(|attribute| attribute.value())
    }

    /// The [`ResourceKind`] identifying the media type
    /// of the resource referenced by [`Self::href`].
    ///
    /// Returns [`None`] if not present.
    pub fn resource_kind(&self) -> Option<ResourceKind<'ebook>> {
        self.media_type().map(Into::into)
    }

    /// List of property values.
    pub fn properties(&self) -> Properties<'ebook> {
        self.data
            .attributes()
            .by_name(consts::PROPERTIES)
            .map_or(Properties::EMPTY, Into::into)
    }

    /// List of relationship values describing the linked resource.
    ///
    /// Common values include:
    /// - `alternate`: Alternate representation
    /// - `record`: Metadata record
    /// - `xml-signature`: XML signature
    pub fn rel(&self) -> Properties<'ebook> {
        self.data
            .attributes()
            .by_name(consts::REL)
            .map_or(Properties::EMPTY, Into::into)
    }

    /// Returns the underlying [`EpubMetaEntry`] to access generic metadata details
    /// such as id, refinements, and attributes.
    pub fn as_meta(&self) -> EpubMetaEntry<'_> {
        EpubMetaEntry::new(self.data)
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
    /// <https://www.w3.org/TR/epub/#sec-opf-dcmes-hd> for official EPUB dublin core documentation.
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
    /// - Pattern Matching
    /// ```
    /// # use rbook::epub::metadata::{EpubMetaEntryKind, EpubVersion};
    /// # use rbook::ebook::errors::EbookResult;
    /// # use rbook::ebook::metadata::Metadata;
    /// # use rbook::{Ebook, Epub};
    /// # fn main() -> EbookResult<()> {
    /// let epub = Epub::open("tests/ebooks/example_epub")?;
    /// let source = epub.metadata().by_property("dc:source").next().unwrap();
    ///
    /// match source.kind() {
    ///     EpubMetaEntryKind::Meta { version: EpubVersion::EPUB3, ..} => {},
    ///     EpubMetaEntryKind::Meta { version: EpubVersion::EPUB2, ..} => {},
    ///     EpubMetaEntryKind::Meta { .. } => {},
    ///     _ => {/* Other kind (i.e., dublin core, link) */},
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
    /// - Assessing a dublin core (`<dc:*>`) element:
    /// ```
    /// # use rbook::epub::metadata::EpubMetaEntryKind;
    /// # use rbook::ebook::errors::EbookResult;
    /// # use rbook::ebook::metadata::Metadata;
    /// # use rbook::{Ebook, Epub};
    /// # fn main() -> EbookResult<()> {
    /// let epub = Epub::open("tests/ebooks/example_epub")?;
    /// let title_meta = epub.metadata().title().unwrap().as_meta();
    /// let kind = title_meta.kind();
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
    ///
    /// # Examples
    /// - Assessing a `meta` element:
    /// ```
    /// # use rbook::epub::metadata::{EpubMetaEntryKind, EpubVersion};
    /// # use rbook::ebook::errors::EbookResult;
    /// # use rbook::ebook::metadata::Metadata;
    /// # use rbook::{Ebook, Epub};
    /// # fn main() -> EbookResult<()> {
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

    /// Returns `true` if the kind is [`Self::Link`].
    ///
    /// # Examples
    /// - Assessing a `link` element:
    /// ```
    /// # use rbook::epub::metadata::EpubMetaEntryKind;
    /// # use rbook::ebook::errors::EbookResult;
    /// # use rbook::ebook::metadata::Metadata;
    /// # use rbook::{Ebook, Epub};
    /// # fn main() -> EbookResult<()> {
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
    /// # use rbook::epub::metadata::{EpubMetaEntryKind, EpubVersion};
    /// # use rbook::ebook::errors::EbookResult;
    /// # use rbook::ebook::metadata::Metadata;
    /// # use rbook::{Ebook, Epub};
    /// # fn main() -> EbookResult<()> {
    /// let epub = Epub::open("tests/ebooks/example_epub")?;
    /// let metadata = epub.metadata();
    /// let cover = metadata.by_property("cover").next().unwrap();
    /// let title = metadata.title().unwrap().as_meta();
    /// let link = metadata.by_id("example-link").unwrap();
    ///
    /// // `<meta>` elements always have a structural version
    /// assert_eq!(cover.kind().version(), Some(EpubVersion::EPUB2));
    ///
    /// // Dublin core `<dc:*>` elements never have an associated structural version
    /// assert_eq!(title.kind().version(), None);
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
#[derive(Copy, Clone, Debug, Hash, PartialEq, Eq, PartialOrd, Ord)]
pub enum EpubVersion {
    /// [`Epub`](super::Epub) Version `2.*` **(Legacy)**
    Epub2(Version),
    /// [`Epub`](super::Epub) Version `3.*`
    ///
    /// Epubs with this version may be backwards compatible with version 2,
    /// `rbook` handles such scenarios behind-the-scenes.
    ///
    /// # See Also
    /// - [`EpubSettings`](super::EpubSettings) for preferences between versions 2 and 3.
    Epub3(Version),
    /// An unknown [`Epub`](super::Epub) version
    ///
    /// An [`Epub`](super::Epub) may contain this version when
    /// [`EpubSettings::strict`](super::EpubSettings::strict) is set to `false`.
    Unknown(Version),
}

impl EpubVersion {
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
    /// let epub_version = EpubVersion::from(Version(3, 3));
    ///
    /// assert_eq!(Version(3, 0), epub_version.as_major().version());
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
    /// If [`EpubSettings::strict`](super::EpubSettings::strict) is set to `false`,
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

impl From<Version> for EpubVersion {
    fn from(version: Version) -> Self {
        match version.0 {
            2 => Self::Epub2(version),
            3 => Self::Epub3(version),
            _ => Self::Unknown(version),
        }
    }
}

impl Display for EpubVersion {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        f.write_str(&self.version().to_string())
    }
}

/// Implementation of [`Identifier`].
///
/// # See Also
/// - [`Self::as_meta`] to access finer details such as attributes and refinements.
#[derive(Copy, Clone, Debug)]
pub struct EpubIdentifier<'ebook> {
    data: &'ebook EpubMetaEntryData,
}

impl_meta_entry_abstraction!(EpubIdentifier);

impl<'ebook> EpubIdentifier<'ebook> {
    fn new(data: &'ebook EpubMetaEntryData) -> Self {
        Self { data }
    }

    /// Fallback when [`Self::get_modern_identifier_type`] is not available.
    fn get_legacy_identifier_type(&self) -> Option<Scheme<'ebook>> {
        self.data
            .refinements
            .get_schemes(consts::IDENTIFIER_TYPE)
            .next()
    }

    fn get_modern_identifier_type(&self) -> Option<Scheme<'ebook>> {
        self.data
            .attributes()
            .by_name(consts::OPF_SCHEME)
            .map(|identifier_type| Scheme::new(None, identifier_type.value()))
    }
}

impl<'ebook> Identifier<'ebook> for EpubIdentifier<'ebook> {
    fn scheme(&self) -> Option<Scheme<'ebook>> {
        self.get_modern_identifier_type()
            .or_else(|| self.get_legacy_identifier_type())
    }
}

impl Eq for EpubIdentifier<'_> {}

impl PartialEq<Self> for EpubIdentifier<'_> {
    fn eq(&self, other: &Self) -> bool {
        self.scheme() == other.scheme() && self.value() == other.value()
    }
}

impl Hash for EpubIdentifier<'_> {
    fn hash<H: Hasher>(&self, state: &mut H) {
        self.value().hash(state);
        self.scheme().hash(state);
    }
}

/// Implementation of [`Title`].
///
/// # See Also
/// - [`Self::as_meta`] to access finer details such as attributes and refinements.
#[derive(Copy, Clone, Debug, PartialEq)]
pub struct EpubTitle<'ebook> {
    data: &'ebook EpubMetaEntryData,
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
    fn new(data: &'ebook EpubMetaEntryData, is_main_title: bool) -> Self {
        Self {
            data,
            is_main_title,
        }
    }
}

impl<'ebook> Title<'ebook> for EpubTitle<'ebook> {
    fn scheme(&self) -> Option<Scheme<'ebook>> {
        self.data
            .refinements
            .by_refinement(consts::TITLE_TYPE)
            .map(|title_type| Scheme::new(None, &title_type.value))
    }

    fn kind(&self) -> TitleKind {
        if self.is_main_title {
            return TitleKind::Main;
        }
        self.data
            .refinements
            .by_refinement(consts::TITLE_TYPE)
            .map_or(TitleKind::Unknown, |title_type| {
                TitleKind::from(&title_type.value)
            })
    }
}

/// Implementation of [`Tag`].
///
/// # See Also
/// - [`Self::as_meta`] to access finer details such as attributes and refinements.
#[derive(Copy, Clone, Debug, PartialEq)]
pub struct EpubTag<'ebook> {
    data: &'ebook EpubMetaEntryData,
}

impl_meta_entry_abstraction!(EpubTag);

impl<'ebook> EpubTag<'ebook> {
    fn new(data: &'ebook EpubMetaEntryData) -> Self {
        Self { data }
    }

    /// Fallback when [`Self::get_modern_scheme`] is not available.
    fn get_legacy_scheme(&self) -> Option<Scheme<'ebook>> {
        let refinements = &self.data.refinements;
        let auth = refinements.by_refinement(consts::AUTHORITY)?;
        let term = refinements.by_refinement(consts::TERM)?;
        Some(Scheme::new(Some(&auth.value), &term.value))
    }

    fn get_modern_scheme(&self) -> Option<Scheme<'ebook>> {
        let attributes = self.data.attributes();
        let authority = attributes.by_name(consts::OPF_AUTHORITY)?;
        let term = attributes.by_name(consts::OPF_TERM)?;
        Some(Scheme::new(Some(authority.value()), term.value()))
    }
}

impl<'ebook> Tag<'ebook> for EpubTag<'ebook> {
    fn scheme(&self) -> Option<Scheme<'ebook>> {
        // A `term` must be present if there's an `authority`.
        // Otherwise, the element does not follow spec/malformed.
        self.get_modern_scheme()
            .or_else(|| self.get_legacy_scheme())
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
/// - [`Self::as_meta`] to access finer details such as attributes and refinements.
#[derive(Copy, Clone, Debug, PartialEq)]
pub struct EpubContributor<'ebook> {
    data: &'ebook EpubMetaEntryData,
}

impl_meta_entry_abstraction!(EpubContributor);

impl<'ebook> EpubContributor<'ebook> {
    fn new(data: &'ebook EpubMetaEntryData) -> Self {
        Self { data }
    }

    /// Fallback when [`Self::get_modern_roles`] is not available.
    fn get_legacy_role(&self) -> Option<Scheme<'ebook>> {
        self.data
            .attributes()
            .by_name(consts::OPF_ROLE)
            .map(|role| Scheme::new(None, role.value()))
    }

    fn get_modern_roles(&self) -> impl Iterator<Item = Scheme<'ebook>> + 'ebook {
        self.data.refinements.get_schemes(consts::ROLE)
    }
}

impl<'ebook> Contributor<'ebook> for EpubContributor<'ebook> {
    fn main_role(&self) -> Option<Scheme<'ebook>> {
        self.roles().next()
    }

    fn roles(&self) -> impl Iterator<Item = Scheme<'ebook>> + 'ebook {
        let roles = self.get_modern_roles().map(Some);
        // If the size hint is 0, attempt to retrieve legacy `opf:role` attribute
        // Note: `size_hint` **works** here as the underlying source is based
        // on a collection (Vec) with a reliable hint.
        let fallback = (roles.size_hint().0 == 0).then(|| self.get_legacy_role());

        roles.chain(iter::once(fallback.flatten())).flatten()
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
/// - [`Self::as_meta`] to access finer details such as attributes and refinements.
#[derive(Copy, Clone, Debug, PartialEq)]
pub struct EpubLanguage<'ebook> {
    data: &'ebook EpubMetaEntryData,
}

impl_meta_entry_abstraction!(EpubLanguage);

impl<'ebook> EpubLanguage<'ebook> {
    fn new(data: &'ebook EpubMetaEntryData) -> Self {
        Self { data }
    }
}

impl<'ebook> Language<'ebook> for EpubLanguage<'ebook> {
    /// Always returns with [`Scheme::source`] always exactly as `BCP 47`.
    ///
    /// See [`EpubLanguage`] for more information.
    ///
    /// # See Also
    /// - [`Language::scheme`]
    fn scheme(&self) -> Scheme<'ebook> {
        Scheme::new(Some(LanguageKind::Bcp47.as_str()), &self.data.value)
    }

    /// Always returns [`LanguageKind::Bcp47`].
    ///
    /// See [`EpubLanguage`] for more information.
    ///
    /// # See Also
    /// - [`Language::kind`]
    fn kind(&self) -> LanguageKind {
        // Normalize both as BCP 47:
        // - EPUB-2 requires RFC 3066 (a subset of BCP 47)
        // - EPUB-3 requires BCP 47
        LanguageKind::Bcp47
    }
}

mod macros {
    macro_rules! impl_meta_entry_abstraction {
        ($implementation:ident) => {
            impl<'ebook> $implementation<'ebook> {
                /// Returns the [`EpubMetaEntry`] form to access additional
                /// metadata entry details, such as attributes and refinements.
                pub fn as_meta(&self) -> EpubMetaEntry<'ebook> {
                    EpubMetaEntry::new(self.data)
                }
            }

            impl PartialEq<EpubMetaEntry<'_>> for $implementation<'_> {
                fn eq(&self, _: &EpubMetaEntry<'_>) -> bool {
                    &self.data == &self.data
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
                    self.data
                        .refinements
                        .by_refinements(consts::ALTERNATE_SCRIPT)
                        .map(|script| {
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
                    let attributes = self.data.attributes();
                    let script = attributes.by_name(consts::OPF_ALT_REP)?.value();
                    let code = attributes.by_name(consts::OPF_ALT_REP_LANG)?.value();
                    Some(AlternateScript::new(
                        script,
                        LanguageTag::new(code, LanguageKind::Bcp47),
                    ))
                }
            }

            impl<'ebook> MetaEntry<'ebook> for $implementation<'ebook> {
                fn value(&self) -> &'ebook str {
                    &self.data.value
                }

                fn order(&self) -> usize {
                    self.data.order
                }

                fn file_as(&self) -> Option<&'ebook str> {
                    self.data
                        .refinements
                        .by_refinement(consts::FILE_AS)
                        .map(|refinement| refinement.value.as_str())
                        // Fallback to legacy `opf:file-as` attribute
                        .or_else(|| {
                            self.data
                                .attributes()
                                .by_name(consts::OPF_FILE_AS)
                                .map(|attribute| attribute.value())
                        })
                }

                fn alternate_scripts(
                    &self,
                ) -> impl Iterator<Item = AlternateScript<'ebook>> + 'ebook {
                    let scripts = self.get_modern_alt_script().map(Some);
                    // If the size hint is 0, attempt to retrieve legacy `opf:alt_*` attribute
                    // Note: `size_hint` **works** here as the underlying source is based
                    // on a collection (Vec) with a reliable hint.
                    let fallback =
                        (scripts.size_hint().0 == 0).then(|| self.get_legacy_alt_script());

                    scripts.chain(iter::once(fallback.flatten())).flatten()
                }
            }
        };
    }

    pub(super) use {impl_meta_entry, impl_meta_entry_abstraction};
}
