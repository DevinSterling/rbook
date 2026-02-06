use crate::ebook::element::{Attributes, AttributesData, TextDirection};
use crate::ebook::epub::archive::EpubArchive;
use crate::ebook::epub::metadata::EpubVersion;
use crate::ebook::epub::package::{
    EpubPackage, EpubPackageData, EpubVersionData, Prefix, Prefixes,
};
use crate::input::IntoOption;
use crate::util::borrow::CowExt;
use crate::util::uri;

impl EpubPackageData {
    pub(in crate::ebook::epub) fn new(location: String, version: EpubVersion) -> Self {
        Self {
            location,
            version: EpubVersionData::from(version),
            unique_identifier: String::new(),
            prefixes: Prefixes::EMPTY,
            language: None,
            text_direction: TextDirection::Auto,
            attributes: AttributesData::default(),
        }
    }
}

/// Mutable view of [`EpubPackage`] accessible via
/// [`Epub::package_mut`](crate::epub::Epub::package_mut).
///
/// Allows the management of package data, including
/// setting the package location and changing the EPUB version.
pub struct EpubPackageMut<'ebook> {
    archive: &'ebook mut EpubArchive,
    package: &'ebook mut EpubPackageData,
}

impl<'ebook> EpubPackageMut<'ebook> {
    pub(in crate::ebook::epub) fn new(
        archive: &'ebook mut EpubArchive,
        package: &'ebook mut EpubPackageData,
    ) -> Self {
        EpubPackageMut { archive, package }
    }

    /// Sets the package file location and returns the previous location.
    ///
    /// This method effectively moves the package file.
    /// **Any existing references are not updated.**
    ///
    /// When creating an [`Epub`](crate::epub::Epub), this method should be called
    /// before adding resources.
    /// This ensures that all subsequently added resources are resolved relative
    /// to the new package directory location.
    /// Changing the location after adding resources will result in potentially
    /// incorrect relative paths and files being stored at the previous directory.
    ///
    /// # Percent Encoding
    /// The given `location` is expected to already be percent encoded.
    ///
    /// For maximum compatibility with reading systems,
    /// it is recommended to only use alphanumeric characters,
    /// dashes (`-`), and underscores (`_`) in directory and file names.
    ///
    /// - **Malformed**: `my parent/My package #1.opf` (Invalid; Not percent-encoded)
    /// - Not recommended: `my%20parent/My%20package%20%231.opf` (Valid; percent-encoded)
    /// - Recommended: `my-parent/my-package-1.opf` (Valid)
    ///
    /// # Normalization
    /// If the location is relative, it is treated as relative to the container root
    /// (effectively absolute). If needed, the given location is normalized.
    ///
    /// # Examples
    /// - Setting the package file location:
    /// ```
    /// # use rbook::Epub;
    /// let mut epub = Epub::new();
    /// let previous = epub.package_mut().set_location("EPUB/my_package.opf");
    ///
    /// assert_eq!("/OEBPS/package.opf", previous);
    /// assert_eq!("/EPUB/my_package.opf", epub.package().location());
    /// ```
    /// - Setting a package file location that must be normalized:
    /// ```
    /// # use rbook::Epub;
    /// let mut epub = Epub::new();
    /// epub.package_mut().set_location("/.//dir-1/./my-file.opf");
    ///
    /// let new_location = epub.package().location();
    /// assert_eq!("/dir-1/my-file.opf", new_location);
    /// ```
    pub fn set_location(&mut self, location: impl Into<String>) -> String {
        let location = location.into();
        let normalized = uri::normalize(&location).take_owned().unwrap_or(location);
        // Make location absolute to ensure consistency with the rest of the API
        let absolute = uri::into_absolute(normalized);

        // Update location
        let previous = std::mem::replace(&mut self.package.location, absolute);
        self.archive.relocate(&previous, &self.package.location);
        previous
    }

    /// Sets the EPUB version and returns the previous version.
    pub fn set_version(&mut self, version: impl Into<EpubVersion>) -> EpubVersion {
        let version = version.into();
        let previous = self.package.version.parsed;

        self.package.version = EpubVersionData::from(version);
        previous
    }

    /// Sets the `id` reference of the package-level unique identifier
    /// and returns the previous `id`.
    ///
    /// **This does not correspond to the identifier
    /// [value](crate::ebook::metadata::MetaEntry::value) itself.**
    /// This maps to the `unique-identifier` attribute on the `<package>`
    /// element and must reference the [`id`](crate::epub::metadata::DetachedEpubMetaEntry::id)
    /// of an existing `dc:identifier` metadata entry.
    ///
    /// # See Also
    /// - [`EpubEditor::identifier`](crate::epub::EpubEditor::identifier) to conveniently
    ///   set the unique identifier of a newly created [`Epub`](crate::epub::Epub).
    ///
    /// # Examples
    /// - Updating the unique identifier of an [`Epub`](crate::epub::Epub):
    /// ```
    /// # use rbook::Epub;
    /// # use rbook::epub::metadata::DetachedEpubMetaEntry;
    /// # fn main() -> rbook::ebook::errors::EbookResult<()> {
    /// let mut epub = Epub::open("tests/ebooks/example_epub")?;
    ///
    /// // Retrieving the current unique identifier
    /// let mut current_uid = epub.metadata().identifier().unwrap();
    /// assert_eq!(Some("uid"), current_uid.id());
    /// assert_eq!("https://github.com/devinsterling/rbook", current_uid.value());
    ///
    /// // Creating a new unique identifier
    /// let xml_id = String::from("new-uid");
    /// let new_identifier = DetachedEpubMetaEntry::identifier("urn:doi:10.1234/abc").id(&xml_id);
    /// epub.metadata_mut().push(new_identifier);
    ///
    /// // Replacing the previous unique identifier
    /// let replaced_id = epub.package_mut().set_unique_identifier(xml_id);
    /// assert_eq!("uid", replaced_id);
    ///
    /// // Retrieving the new unique identifier
    /// let mut new_uid = epub.metadata().identifier().unwrap();
    /// assert_eq!(Some("new-uid"), new_uid.id());
    /// assert_eq!("urn:doi:10.1234/abc", new_uid.value());
    ///
    /// // Removing the old identifier
    /// epub.metadata_mut().remove_by_id(&replaced_id);
    /// # Ok(())
    /// # }
    /// ```
    pub fn set_unique_identifier(&mut self, idref: impl Into<String>) -> String {
        std::mem::replace(&mut self.package.unique_identifier, idref.into())
    }

    /// Sets the package-level [`TextDirection`] (`dir`) and returns the previous direction.
    ///
    /// # Note
    /// This is an EPUB 3 feature.
    /// When [writing](crate::Epub::write) an EPUB 2 ebook, this field is ignored.
    ///
    /// # See Also
    /// - [`EpubMetaEntryMut::set_text_direction`](crate::epub::metadata::EpubMetaEntryMut::set_xml_language)
    ///   to set the direction for a specific entry.
    pub fn set_text_direction(&mut self, direction: TextDirection) -> TextDirection {
        std::mem::replace(&mut self.package.text_direction, direction)
    }

    /// Sets the package-level language code (`xml:lang`) and returns the previous code, if any.
    ///
    /// The given code is not validated and should be a valid
    /// [BCP 47](https://tools.ietf.org/html/bcp47) tag (e.g. `en`, `ja`, `fr-CA`).
    ///
    /// # Note
    /// This is an EPUB 3 feature.
    /// When [writing](crate::Epub::write) an EPUB 2 ebook, this field is ignored.
    ///
    /// # See Also
    /// - [`EpubMetaEntryMut::set_xml_language`](crate::epub::metadata::EpubMetaEntryMut::set_xml_language)
    ///   to set the language for a specific entry.
    pub fn set_xml_language(&mut self, code: impl IntoOption<String>) -> Option<String> {
        std::mem::replace(&mut self.package.language, code.into_option())
    }

    /// Mutable view of all package-level prefixes, defining prefix mappings for use in
    /// [`property`](crate::epub::metadata::EpubMetaEntry::property) values.
    ///
    /// # Note
    /// This is an EPUB 3 feature.
    /// When [writing](crate::Epub::write) an EPUB 2 ebook, this field is ignored.
    pub fn prefixes_mut(&mut self) -> &mut Prefixes {
        &mut self.package.prefixes
    }

    /// Mutable view of all additional `XML` attributes of the `<package>` element.
    ///
    /// # See Also
    /// - [`EpubPackage::attributes`] for important details.
    pub fn attributes_mut(&mut self) -> &mut Attributes {
        &mut self.package.attributes
    }

    /// Returns a read-only view, useful for inspecting state before applying modifications.
    pub fn as_view(&self) -> EpubPackage<'_> {
        EpubPackage::new(self.package)
    }
}

impl Prefix {
    /// Creates a new prefix with the given [`name`](Self::name) and [`uri`](Self::uri).
    pub fn new(name: impl Into<String>, uri: impl Into<String>) -> Self {
        Self {
            name: name.into(),
            uri: uri.into(),
        }
    }

    /// Sets the prefix URI and returns the previous value.
    pub fn set_uri(&mut self, uri: impl Into<String>) -> String {
        std::mem::replace(&mut self.uri, uri.into())
    }
}

impl<N: Into<String>, U: Into<String>> From<(N, U)> for Prefix {
    fn from((name, uri): (N, U)) -> Self {
        Self::new(name.into(), uri.into())
    }
}

impl Prefixes {
    /// Inserts the given prefix and returns the previous prefix with the same name, if any.
    ///
    /// # Examples
    /// - Inserting a prefix:
    /// ```
    /// # use rbook::Epub;
    /// let mut epub = Epub::new();
    /// let mut package = epub.package_mut();
    /// let prefixes = package.prefixes_mut();
    ///
    /// prefixes.insert(("foaf", "http://xmlns.com/foaf/0.1/"));
    ///
    /// let foaf = prefixes.by_name("foaf").unwrap();
    /// assert_eq!("foaf", foaf.name());
    /// assert_eq!("http://xmlns.com/foaf/0.1/", foaf.uri());
    /// ```
    pub fn insert(&mut self, prefix: impl Into<Prefix>) -> Option<Prefix> {
        self.0.insert(prefix.into())
    }

    /// Returns the mutable [`Prefix`] matching the given `name` if present, otherwise [`None`].
    pub fn by_name_mut(&mut self, name: &str) -> Option<&mut Prefix> {
        self.0.by_key_mut(name)
    }

    /// Returns an iterator over all mutable [`Prefix`] entries.
    pub fn iter_mut(&mut self) -> PrefixesMutIter<'_> {
        PrefixesMutIter(self.0.0.iter_mut())
    }

    /// Removes and returns the prefix with the given name, if present.
    pub fn remove(&mut self, name: &str) -> Option<Prefix> {
        self.0.remove(name)
    }

    /// Retains only the prefixes specified by the predicate.
    ///
    /// If the closure returns `false`, the prefix is retained.
    /// Otherwise, the prefix is removed.
    ///
    /// This method operates in place and visits every prefix exactly once.
    ///
    /// # See Also
    /// - [`Self::extract_if`] to retrieve an iterator of the removed prefixes.
    pub fn retain(&mut self, f: impl FnMut(&Prefix) -> bool) {
        self.0.retain(f)
    }

    /// Removes and returns only the prefixes specified by the predicate.
    ///
    /// If the closure returns `true`, the prefix is removed and yielded.
    /// Otherwise, the prefix is retained.
    ///
    /// # Drop
    /// If the returned iterator is not exhausted,
    /// (e.g. dropped without iterating or iteration short-circuits),
    /// then the remaining prefixes are retained.
    ///
    /// Prefer [`Self::retain`] with a negated predicate if the returned iterator is not needed.
    pub fn extract_if(&mut self, f: impl FnMut(&Prefix) -> bool) -> impl Iterator<Item = Prefix> {
        self.0.extract_if(f)
    }

    /// Removes and returns all prefixes.
    pub fn drain(&mut self) -> impl Iterator<Item = Prefix> {
        self.0.drain()
    }

    /// Removes all prefixes.
    ///
    /// # See Also
    /// - [`Self::drain`] to retrieve an iterator of the removed prefixes.
    pub fn clear(&mut self) {
        self.0.clear();
    }
}

impl Extend<Prefix> for Prefixes {
    fn extend<I: IntoIterator<Item = Prefix>>(&mut self, iter: I) {
        for prefix in iter {
            self.insert(prefix);
        }
    }
}

impl<'ebook> IntoIterator for &'ebook mut Prefixes {
    type Item = &'ebook mut Prefix;
    type IntoIter = PrefixesMutIter<'ebook>;

    fn into_iter(self) -> Self::IntoIter {
        self.iter_mut()
    }
}

/// An iterator over all mutable [`Prefix`] entries within [`Prefixes`].
///
/// # See Also
/// - [`Prefixes::iter_mut`] to create an instance of this struct.
pub struct PrefixesMutIter<'ebook>(std::slice::IterMut<'ebook, Prefix>);

impl<'ebook> Iterator for PrefixesMutIter<'ebook> {
    type Item = &'ebook mut Prefix;

    fn next(&mut self) -> Option<Self::Item> {
        self.0.next()
    }
}
