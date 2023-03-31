use std::borrow::Borrow;

use crate::formats::epub::constants;
use crate::formats::xml::{self, Element, Find};
use crate::utility::{self, Shared};

/// Retrieve associated metadata information about the epub.
///
/// For convenience when `meta` elements are encountered,
/// the value of the `name`/`property` and `content` attributes
/// are the `name` and `value` fields of the element.
///
/// # Examples
/// Using metadata and manifest together:
/// ```
/// use rbook::Ebook;
///
/// let epub = rbook::Epub::new("tests/ebooks/moby-dick.epub").unwrap();
///
/// assert_eq!("Moby-Dick", epub.metadata().title().unwrap().value());
///
/// // The cover is optional metadata
/// let cover1 = epub.manifest().by_property("cover-image").unwrap();
/// let cover2 = epub.cover_image().unwrap();
///
/// // The following is also possible if the epub has a cover metadata element:
/// // let cover_id = epub.metadata().cover().unwrap().value();
/// // let cover3 = epub.manifest().by_id(&cover_id).unwrap();
///
/// assert_eq!(cover1, cover2);
///
/// assert_eq!("images/9780316000000.jpg", cover1.value());
/// ```
/// Accessing metadata attributes and child elements:
/// ```
/// # use rbook::Ebook;
/// # let epub = rbook::Epub::new("tests/ebooks/moby-dick.epub").unwrap();
/// // Retrieving the first creator metadata element
/// let creators = epub.metadata().creators();
/// let creator1 = creators.first().unwrap();
///
/// // Retrieving an attribute
/// let id = creator1.get_attribute("id").unwrap();
/// assert_eq!("creator", id);
///
/// // Retrieving a child element
/// let role = creator1.get_child("role").unwrap();
/// assert_eq!("aut", role.value());
///
/// let scheme = role.get_attribute("scheme").unwrap();
/// assert_eq!("marc:relators", scheme);
/// ```
#[derive(Debug)]
pub struct Metadata {
    package: Element,
    element_groups: Vec<(String, Vec<Shared<Element>>)>,
}

impl Metadata {
    pub(crate) fn new(
        package: Element,
        element_groups: Vec<(String, Vec<Shared<Element>>)>,
    ) -> Self {
        Self {
            package,
            element_groups,
        }
    }

    /// Retrieve all metadata elements
    pub fn elements(&self) -> Vec<&Element> {
        self.element_groups
            .iter()
            .map(|(_, elements)| elements)
            .flat_map(|elements| elements.iter().map(Shared::borrow))
            .collect()
    }

    /// Retrieve the epub version associated with the ebook
    pub fn version(&self) -> &str {
        self.package
            .get_attribute(constants::VERSION)
            .expect("Package should have an epub 'version' attribute")
    }

    // Convenient DCMES Required Metadata methods
    // Although rare, some epubs may not contain the metadata.
    // Having them as optional broadens support.
    /// Retrieve the title of ebook.
    ///
    /// If the ebook contains multiple titles, using the method
    /// [get("title")](Self::get) can be used to retrieve them all.
    pub fn title(&self) -> Option<&Element> {
        self.get_element(constants::TITLE)
    }

    /// Language the ebook supports.
    ///
    /// If the ebook contains multiple languages, using the method
    /// [get("language")](Self::get) can be used to retrieve them all.
    ///
    /// Values conform to the **BCP47** standard.
    pub fn language(&self) -> Option<&Element> {
        self.get_element(constants::LANGUAGE)
    }

    // Although rare, some ebooks may not have the identifier metadata entry
    /// Unique identifier associated with the ebook.
    ///
    /// If the ebook contains multiple identifiers, the method
    /// [get("identifier")](Self::get) can be used to retrieve them all.
    ///
    /// Some possible identifiers are:
    /// - UUID
    /// - DOI
    /// - ISBN
    /// - URL
    pub fn unique_identifier(&self) -> Option<&Element> {
        // Retrieve uid from root package element
        let target_id = self.package.get_attribute(constants::UNIQUE_ID)?;
        let identifiers = self.get_elements(constants::IDENTIFIER);

        // Find identifier metadata element that matches
        identifiers
            .iter()
            .find(|element| xml::utility::equals_attribute_by_value(element, xml::ID, target_id))
            .copied()
    }

    /// Retrieve the concatenation of the unique identifier and
    /// modified date separated by an '@'. Since the modified
    /// is not required for epub2, forming a release identifier
    /// is not guaranteed.
    ///
    /// # Examples
    /// Basic Usage:
    /// ```
    /// # use rbook::Ebook;
    /// let epub = rbook::Epub::new("tests/ebooks/moby-dick.epub").unwrap();
    /// let r_id = epub.metadata().release_identifier().unwrap();
    ///
    /// assert_eq!(
    ///     "code.google.com.epub-samples.moby-dick-basic@2012-01-18T12:47:00Z",
    ///     r_id
    /// );
    /// ```
    pub fn release_identifier(&self) -> Option<String> {
        let identifier = self.unique_identifier()?;
        let modified = self.modified()?;
        Some(identifier.value().to_string() + "@" + modified.value())
    }

    /// The date of when the ebook rendition was last modified
    pub fn modified(&self) -> Option<&Element> {
        self.get_element(constants::MODIFIED)
    }

    // Convenient DCMES Optional Metadata methods
    /// Contributors of the ebook, such as editors
    pub fn contributors(&self) -> Vec<&Element> {
        self.get_elements(constants::CONTRIBUTOR)
    }

    /// Creators of the ebook, such as authors
    pub fn creators(&self) -> Vec<&Element> {
        self.get_elements(constants::CREATOR)
    }

    /// The date of publication date for an ebook
    pub fn date(&self) -> Option<&Element> {
        self.get_element(constants::DATE)
    }

    /// Retrieve the title of ebook.
    ///
    /// If the ebook contains multiple descriptions, the method
    /// [get("description")](Self::get) can be used to retrieve them all.
    pub fn description(&self) -> Option<&Element> {
        self.get_element(constants::DESCRIPTION)
    }

    pub fn publisher(&self) -> Vec<&Element> {
        self.get_elements(constants::PUBLISHER)
    }

    /// Indicates the subject of the ebook, such as genre.
    /// May contain **BISAC** codes to specify genres.
    pub fn subject(&self) -> Vec<&Element> {
        self.get_elements(constants::SUBJECT)
    }

    /// Indicates whether the ebook is a specialized type. Types
    /// can be used to specify if the ebook is in the form of a
    /// dictionary, annotations, etc.
    pub fn r#type(&self) -> Vec<&Element> {
        self.get_elements(constants::TYPE)
    }

    /// Retrieve the name and id values of the cover meta
    /// element. The retrieved id from this function can
    /// also be used to retrieve the image path by using
    /// the [by_id(...)](super::Manifest::by_id) method
    /// in [Manifest](super::Manifest).
    pub fn cover(&self) -> Option<&Element> {
        self.get_element(constants::COVER)
    }

    /// Retrieve metadata fields not explicitly provided by the API.
    ///
    /// Prefixes/namespaces for metadata entries are ignored.
    ///
    /// The given string will retrieve all metadata whose
    /// `name` or `property` field matches it.
    pub fn get(&self, mut input: &str) -> Vec<&Element> {
        // Ignore namespace if provided
        if let Some((_, right)) = utility::split_where(input, ':') {
            input = right
        }

        self.get_elements(input)
    }

    fn get_element(&self, meta_name: &str) -> Option<&Element> {
        self.element_groups
            .iter()
            .find(|(group, _)| group == meta_name.trim())
            .map(|(_, elements)| {
                elements
                    .first()
                    .map(Shared::borrow)
                    .expect("Vector should not be empty; missing child elements")
            })
    }

    fn get_elements(&self, meta_name: &str) -> Vec<&Element> {
        self.element_groups
            .iter()
            .find(|(group, _)| group == meta_name.trim())
            .map(|(_, elements)| elements.iter().map(Shared::borrow).collect())
            .unwrap_or_default()
    }
}

impl Find for Metadata {
    fn __find_fallback(&self, field: &str, is_wildcard: bool) -> Vec<&Element> {
        match is_wildcard {
            true => self.elements(),
            false => self.get_elements(field),
        }
    }
}
