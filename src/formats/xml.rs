pub(super) const ID: &str = "id";
pub(super) const HREF: &str = "href";
pub(super) const SRC: &str = "src";

/// Representation of an xml element, where its attributes,
/// children, values, and name are accessible.
///
/// # Examples
/// Basic Usage:
/// ```
/// # use rbook::Ebook;
/// # let epub = rbook::Epub::new("tests/ebooks/moby-dick.epub").unwrap();
/// // Retrieving an element from the metadata of an epub
/// let element = epub.metadata().creators().unwrap().first().unwrap();
///
/// // Retrieving an attribute
/// let attribute = element.get_attribute("id").unwrap();
///
/// // Retrieving a child element
/// let child_element = element.get_child("role").unwrap();
///
/// assert_eq!("creator", attribute.value());
/// assert_eq!("aut", child_element.value());
/// ```
#[derive(Debug, PartialEq)]
pub struct Element {
    pub(super) name: String,
    pub(super) attributes: Vec<Attribute>,
    pub(super) value: String,
    pub(super) children: Option<Vec<Element>>
}

impl Element {
    pub fn name(&self) -> &str {
        &self.name
    }

    pub fn value(&self) -> &str {
        &self.value
    }

    /// Retrieve all attributes
    pub fn attributes(&self) -> &[Attribute] {
        &self.attributes
    }

    /// Retrieve the specified attribute. Namespace/prefix
    /// may be omitted from the argument.
    pub fn get_attribute(&self, name: &str) -> Option<&Attribute> {
        self.attributes.iter()
            .find(|attribute| attribute.name().ends_with(&name.to_lowercase()))
    }

    /// Check if the element contains the specified attribute.
    /// Namespace/prefix may be omitted from the argument.
    pub fn contains_attribute(&self, name: &str) -> bool {
        self.attributes.iter()
            .any(|attribute| attribute.name().ends_with(&name.to_lowercase()))
    }

    /// Retrieve all child elements
    pub fn children(&self) -> Option<&[Element]> {
        self.children.as_deref()
    }

    /// Retrieve the specified child element. Namespace/prefix
    /// may be omitted from the argument.
    pub fn get_child(&self, name: &str) -> Option<&Element> {
        self.children()
            .and_then(|children| children.iter()
                .find(|child| child.name().ends_with(&name.to_lowercase())))
    }

    /// Check if the element contains the specified child element.
    /// Namespace/prefix may be omitted from the argument.
    pub fn contains_child(&self, name: &str) -> bool {
        self.children()
            .map_or(false, |children| children.iter()
                .any(|child| child.name().ends_with(&name.to_lowercase())))
    }
}

/// Representation of an xml attribute, where its name and
/// value are accessible.
///
/// # Examples
/// Basic Usage:
/// ```
/// # use rbook::Ebook;
/// # let epub = rbook::Epub::new("tests/ebooks/moby-dick.epub").unwrap();
/// // Retrieving an element from the manifest of an epub
/// let element = epub.manifest().by_id("xchapter_009").unwrap();
///
/// // Retrieving attributes
/// let attribute = element.get_attribute("media-type").unwrap();
///
/// assert_eq!("media-type", attribute.name());
/// assert_eq!("application/xhtml+xml", attribute.value());
/// ```
#[derive(Debug, PartialEq, Eq)]
pub struct Attribute {
    pub(super) name: String,
    pub(super) value: String,
}

impl Attribute {
    pub fn name(&self) -> &str {
        &self.name
    }

    pub fn value(&self) -> &str {
        &self.value
    }
}

// Utility functions module
pub(crate) mod utility {
    use lol_html::html_content::Attribute as LolAttribute;
    use super::{Element, Attribute};

    pub(crate) fn equals_attribute_by_value(element: &Element, field: &str, value: &str) -> bool {
        element.get_attribute(field)
            .map_or(false, |attribute| attribute.value()
                .split_whitespace()
                .any(|slice| slice == value))
    }

    pub(crate) fn copy_attributes(old_attributes: &[LolAttribute]) -> Vec<Attribute> {
        old_attributes.iter().map(|attr| Attribute {
            name: attr.name(),
            value: attr.value(),
        }).collect()
    }

    pub(crate) fn take_attribute(attributes: &mut Vec<Attribute>, field: &str) -> Option<Attribute> {
        attributes.iter()
            .position(|attribute| attribute.name() == field)
            .map(|index| attributes.remove(index))
    }
}