//! EPUB package content.

#[cfg(feature = "write")]
mod write;

use crate::ebook::element::{Attributes, AttributesData, Href, TextDirection};
use crate::ebook::epub::metadata::{
    EpubMetaEntry, EpubMetaEntryData, EpubRefinements, EpubRefinementsData, EpubVersion,
};
use crate::util::collection::{Keyed, KeyedVec};
use crate::util::uri;

#[cfg(feature = "write")]
pub use write::{EpubPackageMut, PrefixesMutIter};

////////////////////////////////////////////////////////////////////////////////
// PRIVATE API
////////////////////////////////////////////////////////////////////////////////

#[derive(Debug, PartialEq)]
pub(super) struct EpubPackageData {
    /// ***Absolute percent-encoded*** `.opf` package file location
    /// (e.g. `/OEBPS/package.opf`).
    pub(super) location: String,
    pub(super) version: EpubVersionData,
    /// The `id` of the primary unique identifier.
    pub(super) unique_identifier: String,
    pub(super) prefixes: Prefixes,
    /// Default package document language
    pub(super) language: Option<String>,
    /// Default package document text directionality
    pub(super) text_direction: TextDirection,
    pub(super) attributes: AttributesData,
}

/// Contains the raw and parsed epub version
#[derive(Debug, PartialEq)]
pub(super) struct EpubVersionData {
    pub(super) raw: String,
    pub(super) parsed: EpubVersion,
}

impl From<EpubVersion> for EpubVersionData {
    fn from(parsed: EpubVersion) -> Self {
        Self {
            raw: parsed.to_string(),
            parsed,
        }
    }
}

impl<'ebook> From<&'ebook EpubPackageData> for EpubPackageMetaContext<'ebook> {
    fn from(value: &'ebook EpubPackageData) -> Self {
        Self::new(value)
    }
}

#[derive(Copy, Clone, Debug)]
pub(super) struct EpubPackageMetaContext<'ebook>(Option<&'ebook EpubPackageData>);

impl<'ebook> EpubPackageMetaContext<'ebook> {
    #[cfg(feature = "write")]
    pub(super) const EMPTY: EpubPackageMetaContext<'static> = EpubPackageMetaContext(None);

    pub(super) fn new(package: &'ebook EpubPackageData) -> Self {
        Self(Some(package))
    }

    pub(super) fn package_language(&self) -> Option<&'ebook str> {
        self.0?.language.as_deref()
    }

    pub(super) fn package_text_direction(&self) -> TextDirection {
        self.0
            .as_ref()
            .map_or(TextDirection::Auto, |pkg| pkg.text_direction)
    }

    pub(super) fn create_entry(
        self,
        data: &'ebook EpubMetaEntryData,
        index: usize,
    ) -> EpubMetaEntry<'ebook> {
        EpubMetaEntry::new(self, None, data, index)
    }

    pub(super) fn create_refining_entry(
        self,
        parent_id: Option<&'ebook str>,
        data: &'ebook EpubMetaEntryData,
        index: usize,
    ) -> EpubMetaEntry<'ebook> {
        EpubMetaEntry::new(self, parent_id, data, index)
    }

    pub(super) fn create_refinements(
        self,
        parent_id: Option<&'ebook str>,
        data: &'ebook EpubRefinementsData,
    ) -> EpubRefinements<'ebook> {
        EpubRefinements::new(self, parent_id, data)
    }
}

////////////////////////////////////////////////////////////////////////////////
// PUBLIC API
////////////////////////////////////////////////////////////////////////////////

/// The EPUB package, mapped to the `<package>` element,
/// accessible via [`Epub::package`](super::Epub::package).
///
/// # See Also
/// - [`EpubPackageMut`] for a mutable view.
#[derive(Copy, Clone, Debug, PartialEq)]
pub struct EpubPackage<'ebook>(&'ebook EpubPackageData);

impl<'ebook> EpubPackage<'ebook> {
    pub(super) fn new(data: &'ebook EpubPackageData) -> Self {
        Self(data)
    }

    /// The absolute percent-encoded location of the package `.opf` file.
    ///
    /// This is ***not*** a filesystem path.
    /// It always starts with `/` to indicate the EPUB container root,
    /// and is percent encoded (e.g., `/my%20dir/my%20pkg.opf`).
    ///
    /// # See Also
    /// - [`Href::decode`] to retrieve the percent-decoded form.
    /// - [`Href::name`] to retrieve the filename.
    ///
    /// # Examples
    /// - Retrieving the package file:
    /// ```
    /// # use rbook::Epub;
    /// # fn main() -> rbook::ebook::errors::EbookResult<()> {
    /// let epub = Epub::open("tests/ebooks/example_epub")?;
    ///
    /// assert_eq!("/EPUB/example.opf", epub.package().location().as_str());
    /// # Ok(())
    /// # }
    /// ```
    pub fn location(&self) -> Href<'ebook> {
        Href::new(&self.0.location)
    }

    /// The absolute percent-encoded directory, the package [file](Self::location) resides in.
    ///
    /// This is ***not*** a filesystem path.
    /// It always starts with `/` to indicate the EPUB container root,
    /// and is percent encoded (e.g., `/my%20dir`).
    ///
    /// [`Resources`](crate::ebook::resource::Resource)
    /// referenced in the package file are resolved relative to the package directory.
    ///
    /// # See Also
    /// - [`Href::decode`] to retrieve the percent-decoded form.
    ///
    /// # Examples
    /// - Retrieving the package file and directory:
    /// ```
    /// # use rbook::Epub;
    /// # fn main() -> rbook::ebook::errors::EbookResult<()> {
    /// let epub = Epub::open("tests/ebooks/example_epub")?;
    /// let package_dir = epub.package().directory().as_str();
    /// let package_file = epub.package().location().as_str();
    ///
    /// assert_eq!("/EPUB", package_dir);
    /// assert_eq!(format!("{package_dir}/example.opf"), package_file);
    /// # Ok(())
    /// # }
    /// ```
    pub fn directory(&self) -> Href<'ebook> {
        Href::new(uri::parent(&self.0.location))
    }

    /// The [`Epub`](super::Epub) version (e.g., `2.0`, `3.2`, etc.).
    ///
    /// The returned version may be [`EpubVersion::Unknown`] if
    /// [`EpubOpenOptions::strict`](super::EpubOpenOptions::strict) is disabled.
    ///
    /// See [`Self::version_str`] for the original representation.
    ///
    /// # Note
    /// This method is equivalent to calling
    /// [`EpubMetadata::version`](super::metadata::EpubMetadata::version).
    pub fn version(&self) -> EpubVersion {
        self.0.version.parsed
    }

    /// The underlying [`Epub`](super::Epub) version string.
    ///
    /// # Note
    /// This method is equivalent to calling
    /// [`EpubMetadata::version_str`](super::metadata::EpubMetadata::version_str).
    pub fn version_str(&self) -> &'ebook str {
        self.0.version.raw.as_str()
    }

    /// The `id` of the packages's unique identifier metadata entry.
    ///
    /// This is a lower-level call than
    /// [`EpubMetadata::identifier`](super::metadata::EpubMetadata::identifier).
    ///
    /// # Examples
    /// - Comparing the unique identifier:
    /// ```
    /// # use rbook::Epub;
    /// # fn main() -> rbook::ebook::errors::EbookResult<()> {
    /// let epub = Epub::open("tests/ebooks/example_epub")?;
    /// let unique_identifier = epub.package().unique_identifier();
    /// let identifier = epub.metadata().identifier().unwrap();
    ///
    /// assert_eq!("uid", unique_identifier);
    /// assert_eq!(Some("uid"), identifier.id());
    /// assert_eq!(unique_identifier, identifier.id().unwrap());
    /// # Ok(())
    /// # }
    /// ```
    pub fn unique_identifier(&self) -> &'ebook str {
        &self.0.unique_identifier
    }

    /// The package-level [text direction](TextDirection) (`dir`).
    ///
    /// All metadata entries within an [`Epub`](super::Epub) implicitly inherit this
    /// field if their [`EpubMetaEntry::text_direction`](EpubMetaEntry::text_direction)
    /// is set to [`TextDirection::Auto`].
    pub fn text_direction(&self) -> TextDirection {
        self.0.text_direction
    }

    /// The package-level language code (`xml:lang`) in `BCP 47` format, if present.
    ///
    /// All metadata entries within an [`Epub`](super::Epub) implicitly inherit this
    /// field if their [`EpubMetaEntry::xml_language`](EpubMetaEntry::xml_language)
    /// is set to [`None`].
    pub fn xml_language(&self) -> Option<&'ebook str> {
        self.0.language.as_deref()
    }

    /// The package-level prefixes, defining prefix mappings for use in
    /// [`property`](EpubMetaEntry::property) values.
    pub fn prefixes(&self) -> &'ebook Prefixes {
        &self.0.prefixes
    }

    /// All additional XML [`Attributes`] of the `<package>` element.
    ///
    /// This method provides access to non-standard or vendor-specific attributes
    /// not explicitly handled by this struct.
    ///
    /// # Omitted Attributes
    /// The following attributes will **not** be found within the returned collection:
    /// - [`version`](Self::version)
    /// - [`unique-identifier`](Self::unique_identifier)
    /// - [`dir`](Self::text_direction)
    /// - [`xml:lang`](Self::xml_language)
    /// - [`prefix`](Self::prefixes)
    pub fn attributes(&self) -> &'ebook Attributes {
        &self.0.attributes
    }
}

/// A collection of [`Prefix`] entries.
#[derive(Debug, PartialEq)]
pub struct Prefixes(KeyedVec<Prefix>);

impl Prefixes {
    pub(super) const EMPTY: Prefixes = Self::new(Vec::new());

    pub(super) const fn new(prefixes: Vec<Prefix>) -> Self {
        Self(KeyedVec(prefixes))
    }

    /// The number of [`Prefix`] entries contained within.
    pub fn len(&self) -> usize {
        self.0.len()
    }

    /// Returns `true` if there are no prefixes.
    pub fn is_empty(&self) -> bool {
        self.0.is_empty()
    }

    /// Returns the associated [`Prefix`] if the given `index` is less than
    /// [`Self::len`], otherwise [`None`].
    pub fn get(&self, index: usize) -> Option<&Prefix> {
        self.0.get(index)
    }

    /// Returns an iterator over **all** [`Prefix`] entries.
    pub fn iter(&self) -> PrefixesIter<'_> {
        PrefixesIter(self.0.0.iter())
    }

    /// Returns the [`Prefix`] with the given `name` if present, otherwise [`None`].
    pub fn by_name(&self, name: &str) -> Option<&Prefix> {
        self.0.by_key(name)
    }

    /// Returns `true` if a [`Prefix`] with the given `name` is present.
    pub fn has_name(&self, name: &str) -> bool {
        self.0.has_key(name)
    }
}

impl<'ebook> IntoIterator for &'ebook Prefixes {
    type Item = &'ebook Prefix;
    type IntoIter = PrefixesIter<'ebook>;

    fn into_iter(self) -> Self::IntoIter {
        self.iter()
    }
}

/// An iterator over all [`Prefix`] entries within [`Prefixes`].
///
/// # See Also
/// - [`Prefixes::iter`] to create an instance of this struct.
#[derive(Clone, Debug)]
pub struct PrefixesIter<'ebook>(std::slice::Iter<'ebook, Prefix>);

impl<'ebook> Iterator for PrefixesIter<'ebook> {
    type Item = &'ebook Prefix;

    fn next(&mut self) -> Option<Self::Item> {
        self.0.next()
    }
}

/// A prefix, defining a mapping for use in [`property`](EpubMetaEntry::property) values.
///
/// A prefix encompasses a [`name`](Prefix::name)
/// as **key** and a [`URI`](Prefix::uri) as **value**.
///
/// # Note
/// When the `write` feature flag is enabled, only modification of the URI is allowed.
/// **The name cannot be modified once a prefix is created.**
/// This prevents duplicate keys within [`Prefixes`].
#[derive(Clone, Debug, PartialEq)]
pub struct Prefix {
    name: String,
    uri: String,
}

impl Prefix {
    /// Internal constructor.
    ///
    /// The public constructor [`Self::new`] is available when the `write`
    /// feature is enabled.
    pub fn create(name: impl Into<String>, uri: impl Into<String>) -> Self {
        Self {
            name: name.into(),
            uri: uri.into(),
        }
    }

    /// The prefix name/key.
    pub fn name(&self) -> &str {
        &self.name
    }

    /// The prefix URI.
    pub fn uri(&self) -> &str {
        &self.uri
    }
}

impl Keyed for Prefix {
    type Key = str;

    fn key(&self) -> &Self::Key {
        &self.name
    }
}
