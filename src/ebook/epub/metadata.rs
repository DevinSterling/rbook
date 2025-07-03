//! EPUB metadata-related content.

use crate::ebook::element::{AttributeData, Attributes, Name, TextDirection};
use crate::ebook::epub::consts;
use crate::ebook::epub::metadata::macros::{impl_meta_entry, impl_meta_entry_abstraction};
use crate::ebook::metadata::{AlternateScript, DateTime, Scheme, Version};
use crate::ebook::metadata::{
    Contributor, Identifier, Language, LanguageKind, LanguageTag, MetaEntry, Tag, Title, TitleKind,
};
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

    fn get_schemes(&self, key: &str) -> impl Iterator<Item = Scheme> {
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

#[derive(Debug, Default, PartialEq, Eq, Hash)]
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
}

impl EpubMetaEntryData {
    fn language(&self) -> Option<&str> {
        self.language.as_ref().map(|language| language.as_str())
    }

    fn attributes(&self) -> Attributes {
        (&self.attributes).into()
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
            .map(Vec::as_slice)
            .unwrap_or(&[])
            .iter()
    }

    /// Returns an iterator over **all** non-refining [`entries`](EpubMetaEntry) whose
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
    /// # Note
    /// Refining entries, `<meta>` elements with a `refines` field, are excluded:
    /// ```xhtml
    /// <meta refines="#parent-id">...</meta>
    /// ```
    ///
    /// # See Also
    /// - [`Self::entries`]
    pub fn by_property(
        &self,
        property: &str,
    ) -> impl Iterator<Item = EpubMetaEntry<'ebook>> + 'ebook {
        self.data_by_property(property).map(EpubMetaEntry)
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
                .map(EpubIdentifier)
        })
    }

    fn identifiers(&self) -> impl Iterator<Item = EpubIdentifier<'ebook>> + 'ebook {
        self.data_by_property(consts::IDENTIFIER)
            .map(EpubIdentifier)
    }

    /// The main [`language`](EpubLanguage) of an [`Epub`](super::Epub).
    ///
    /// Returns [`None`] if there is no language specified when
    /// [`EpubSettings::strict`](super::EpubSettings::strict) is disabled.
    fn language(&self) -> Option<EpubLanguage<'ebook>> {
        self.languages().next()
    }

    fn languages(&self) -> impl Iterator<Item = EpubLanguage<'ebook>> + 'ebook {
        self.data_by_property(consts::LANGUAGE).map(EpubLanguage)
    }

    /// The main [`title`](EpubTitle) of an [`Epub`](super::Epub).
    ///
    /// Returns [`None`] if there is no title specified when
    /// [`EpubSettings::strict`](super::EpubSettings::strict) is disabled.
    fn title(&self) -> Option<EpubTitle<'ebook>> {
        self.data_by_property(consts::TITLE)
            // First try to find if a `main` title exists
            .find(|title| {
                title
                    .refinements
                    .by_refinement(consts::TITLE_TYPE)
                    .is_some_and(|title_type| title_type.value == consts::MAIN_TITLE_TYPE)
            })
            // If not, retrieve the first title
            .map_or_else(|| self.titles().next(), |data| Some(EpubTitle(data)))
    }

    fn titles(&self) -> impl Iterator<Item = EpubTitle<'ebook>> + 'ebook {
        self.data_by_property(consts::TITLE).map(EpubTitle)
    }

    fn description(&self) -> Option<EpubMetaEntry<'ebook>> {
        self.descriptions().next()
    }

    fn descriptions(&self) -> impl Iterator<Item = EpubMetaEntry<'ebook>> + 'ebook {
        self.data_by_property(consts::DESCRIPTION)
            .map(EpubMetaEntry)
    }

    fn creators(&self) -> impl Iterator<Item = EpubContributor<'ebook>> + 'ebook {
        self.data_by_property(consts::CREATOR).map(EpubContributor)
    }

    fn contributors(&self) -> impl Iterator<Item = EpubContributor<'ebook>> + 'ebook {
        self.data_by_property(consts::CONTRIBUTOR)
            .map(EpubContributor)
    }

    fn publishers(&self) -> impl Iterator<Item = EpubContributor<'ebook>> + 'ebook {
        self.data_by_property(consts::PUBLISHER)
            .map(EpubContributor)
    }

    fn tags(&self) -> impl Iterator<Item = EpubTag<'ebook>> + 'ebook {
        self.data_by_property(consts::SUBJECT).map(EpubTag)
    }

    /// Returns an iterator over the top-level (non-refining) metadata entries.
    ///
    /// # Note
    /// Refining entries, `<meta>` elements with a `refines` field, are excluded:
    /// ```xhtml
    /// <meta refines="#parent-id">...</meta>
    /// ```
    ///
    /// # See Also
    /// - [`Self::by_property`]
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
        self.data.entries.values().flatten().map(EpubMetaEntry)
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
        self.0.get(index).map(EpubMetaEntry)
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
        self.0.by_refinements(property).map(EpubMetaEntry)
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
        self.0.next().map(EpubMetaEntry)
    }
}

/// A [`MetaEntry`] within [`EpubMetadata`].
#[derive(Copy, Clone, Debug, PartialEq)]
pub struct EpubMetaEntry<'ebook>(&'ebook EpubMetaEntryData);

impl_meta_entry!(EpubMetaEntry);

impl<'ebook> EpubMetaEntry<'ebook> {
    /// The unique `id` of a metadata entry.
    pub fn id(&self) -> Option<&'ebook str> {
        self.0.id.as_deref()
    }

    /// The `id` a metadata entry refines.
    ///
    /// Returns [`Some`] if an entry is refining another, otherwise [`None`].
    pub fn refines(&self) -> Option<&'ebook str> {
        self.0.refines.as_deref()
    }

    /// The `property`, such as `dc:title`, `media:duration`, `file-as`, etc.
    ///
    /// # Property Mapping
    /// Depending on the `XML` element type and EPUB version, this field may be mapped
    /// differently:
    ///
    /// | `XML` Element Type   | Mapped From                                                        |
    /// |----------------------|--------------------------------------------------------------------|
    /// | Dublin Core (`dc:*`) | element tag (`<dc:title>...</dc:title>`)                           |
    /// | EPUB 2 `<meta>`      | `name` attribute (`<meta name="cover" content="..."/>`)            |
    /// | EPUB 3 `<meta>`      | `property` attribute (`<meta property="media:duration">...</meta>`)|
    pub fn property(&self) -> Name<'ebook> {
        self.0.property.as_str().into()
    }

    /// The [`Scheme`] of an entry.
    ///
    /// # Scheme Mapping
    /// The behavior of this method changes depending on an entry’s immediate attributes
    /// ([`Self::attributes`]).
    ///
    /// | Attribute presence  | `Scheme::source` Mapping | `Scheme::code` Mapping |
    /// |---------------------|--------------------------|------------------------|
    /// | Legacy `opf:scheme` | [`None`]                 | value of `opf:scheme`  |
    /// | `scheme`            | value of `scheme`        | [`Self::value`]        |
    /// | none                | [`None`]                 | [`Self::value`]        |
    ///
    /// # Examples
    /// - Legacy `opf:scheme` attribute present:
    /// ```xhtml
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
    /// ```xhtml
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
    /// ```xhtml
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
        let attributes = self.0.attributes();

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
    /// [`None`] is returned if the `<package>` element contains no `xml:lang` attribute.
    pub fn language(&self) -> Option<LanguageTag<'ebook>> {
        self.0
            .language()
            .map(|code| LanguageTag::new(code, LanguageKind::Bcp47))
    }

    /// The specified or inherited text direction (`ltr`, `rtl`, or `auto`).
    ///
    /// [`TextDirection::Auto`] is returned if the `<package>` and specified element
    /// contains no `dir` attribute.
    pub fn text_direction(&self) -> TextDirection {
        self.0.text_direction
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
        (&self.0.attributes).into()
    }

    /// Complementary refinement metadata entries.
    pub fn refinements(&self) -> EpubRefinements<'ebook> {
        (&self.0.refinements).into()
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
    /// See [`EpubSettings`](super::EpubSettings) for preferences between versions 2 and 3.
    Epub3(Version),
    /// Unknown [`Epub`](super::Epub) version
    ///
    /// An [`Epub`](super::Epub) may contain this version if
    /// [`EpubSettings::strict`](super::EpubSettings::strict) is disabled.
    Unknown(Version),
}

impl EpubVersion {
    /// [`EpubVersion::Epub2`] constant with a predefined version of `2.0`.
    pub const EPUB2: EpubVersion = EpubVersion::Epub2(Version(2, 0));

    /// [`EpubVersion::Epub3`] constant with a predefined version of `3.0`.
    pub const EPUB3: EpubVersion = EpubVersion::Epub3(Version(3, 0));

    /// Returns the major form of an epub version.
    ///
    /// If the contained [`Version`] is `3.3`, then the returned [`EpubVersion`]
    /// will have a contained value of `3.0`.
    pub fn as_major(&self) -> EpubVersion {
        match self {
            EpubVersion::Epub2(_) => EpubVersion::EPUB2,
            EpubVersion::Epub3(_) => EpubVersion::EPUB3,
            EpubVersion::Unknown(version) => EpubVersion::Unknown(Version(version.0, 0)),
        }
    }

    /// The encapsulated version information.
    ///
    /// # Note
    /// If [`EpubSettings::strict`](super::EpubSettings::strict) is set to `false`,
    /// the returned [`Version`] may not be within the valid range: `2 <= version < 4`.
    pub fn version(&self) -> Version {
        match self {
            EpubVersion::Epub2(version) => *version,
            EpubVersion::Epub3(version) => *version,
            EpubVersion::Unknown(version) => *version,
        }
    }

    /// Returns `true` if the variant is [`EpubVersion::Epub2`].
    pub fn is_epub2(&self) -> bool {
        matches!(self, EpubVersion::Epub2(_))
    }

    /// Returns `true` if the variant is [`EpubVersion::Epub3`].
    pub fn is_epub3(&self) -> bool {
        matches!(self, EpubVersion::Epub3(_))
    }

    /// Returns `true` if the variant is [`EpubVersion::Unknown`].
    pub fn is_unknown(&self) -> bool {
        matches!(self, EpubVersion::Unknown(_))
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

impl_meta_entry_abstraction! {
    impl Identifier as EpubIdentifier;
}

impl<'ebook> EpubIdentifier<'ebook> {
    /// Fallback when [`Self::get_modern_identifier_type`] is not available.
    fn get_legacy_identifier_type(&self) -> Option<Scheme<'ebook>> {
        self.0
            .refinements
            .get_schemes(consts::IDENTIFIER_TYPE)
            .next()
    }

    fn get_modern_identifier_type(&self) -> Option<Scheme<'ebook>> {
        self.0
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

impl_meta_entry_abstraction! {
    #[derive(PartialEq)]
    impl Title as EpubTitle;
}

impl<'ebook> Title<'ebook> for EpubTitle<'ebook> {
    fn scheme(&self) -> Option<Scheme<'ebook>> {
        self.0
            .refinements
            .by_refinement(consts::TITLE_TYPE)
            .map(|title_type| Scheme::new(None, &title_type.value))
    }

    fn kind(&self) -> TitleKind {
        self.0.refinements.by_refinement(consts::TITLE_TYPE).map_or(
            TitleKind::Unknown,
            |title_type| match title_type.value.as_str() {
                // There is no From<&str> method for TitleKind because
                // other ebook formats may have different
                // (and potentially conflicting) mappings
                // (i.e., main-title, primary, etc.)
                "main" => TitleKind::Main,
                "subtitle" => TitleKind::Subtitle,
                "short" => TitleKind::Short,
                "collection" => TitleKind::Collection,
                "edition" => TitleKind::Edition,
                "expanded" => TitleKind::Expanded,
                _ => TitleKind::Unknown,
            },
        )
    }
}

impl_meta_entry_abstraction! {
    #[derive(PartialEq)]
    impl Tag as EpubTag;
}

impl<'ebook> EpubTag<'ebook> {
    /// Fallback when [`Self::get_modern_scheme`] is not available.
    fn get_legacy_scheme(&self) -> Option<Scheme<'ebook>> {
        let refinements = &self.0.refinements;
        let auth = refinements.by_refinement(consts::AUTHORITY)?;
        let term = refinements.by_refinement(consts::TERM)?;
        Some(Scheme::new(Some(&auth.value), &term.value))
    }

    fn get_modern_scheme(&self) -> Option<Scheme<'ebook>> {
        let attributes = self.0.attributes();
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

impl_meta_entry_abstraction! {
    #[derive(PartialEq)]
    impl Contributor as EpubContributor;
}

impl<'ebook> EpubContributor<'ebook> {
    /// Fallback when [`Self::get_modern_roles`] is not available.
    fn get_legacy_role(&self) -> Option<Scheme<'ebook>> {
        self.0
            .attributes()
            .by_name(consts::OPF_ROLE)
            .map(|role| Scheme::new(None, role.value()))
    }

    fn get_modern_roles(&self) -> impl Iterator<Item = Scheme<'ebook>> + 'ebook {
        self.0.refinements.get_schemes(consts::ROLE)
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

impl_meta_entry_abstraction! {
    /// EPUB language tags are always treated as `BCP 47` (EPUB 3),
    /// even when originating from `RFC 3066` (EPUB 2).
    ///
    /// - EPUB 3 requires the language scheme as BCP 47.
    /// - EPUB 2 requires a subset of BCP 47, RFC 3066.
    /// - For simplicity, `rbook` normalizes RFC 3066 ***into*** BCP 47.
    ///   Both [`EpubLanguage::scheme`] and [`EpubLanguage::kind`]
    ///   will always report `BCP 47`.
    #[derive(PartialEq)]
    impl Language as EpubLanguage;
}

impl<'ebook> Language<'ebook> for EpubLanguage<'ebook> {
    /// Always returns with [`Scheme::source`] always exactly as `BCP 47`.
    ///
    /// See [`EpubLanguage`] for more information.
    ///
    /// # See Also
    /// - [`Language::scheme`]
    fn scheme(&self) -> Scheme<'ebook> {
        Scheme::new(Some(LanguageKind::Bcp47.as_str()), &self.0.value)
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
        { $(#[$attr:meta])* impl $_trait:path as $implementation:ident; } => {
            /// Implementation of
            #[doc = concat!("[`", stringify!($_trait), "`]")]
            /// for [`EpubMetadata`].
            ///
            /// For access to finer details such as attributes, and refinements,
            /// this struct may be observed in the form of an [`EpubMetaEntry`]
            /// through [`Self::as_meta`].
            ///
            $(#[$attr])*
            #[derive(Copy, Clone, Debug)]
            pub struct $implementation<'ebook>(&'ebook EpubMetaEntryData);

            impl<'ebook> $implementation<'ebook> {
                /// Returns the [`EpubMetaEntry`] form of this instance to access additional
                /// meta details, such as attributes and refinements.
                pub fn as_meta(&self) -> EpubMetaEntry<'ebook> {
                    EpubMetaEntry(self.0)
                }
            }

            impl_meta_entry!($implementation);
        }
    }

    /// Implements [`MetaEntry`](super::MetaEntry) for the specified type.
    macro_rules! impl_meta_entry {
        ($implementation:ident) => {
            impl<'ebook> $implementation<'ebook> {
                fn get_modern_alt_script(
                    &self,
                ) -> impl Iterator<Item = AlternateScript<'ebook>> + 'ebook {
                    self.0
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
                    let attributes = self.0.attributes();
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
                    &self.0.value
                }

                fn order(&self) -> usize {
                    self.0.order
                }

                fn file_as(&self) -> Option<&'ebook str> {
                    self.0
                        .refinements
                        .by_refinement(consts::FILE_AS)
                        .map(|refinement| refinement.value.as_str())
                        // Fallback to legacy `opf:file-as` attribute
                        .or_else(|| {
                            self.0
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
