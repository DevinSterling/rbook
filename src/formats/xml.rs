pub(crate) mod utility;

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
/// let id = element.get_attribute("id").unwrap();
/// assert_eq!("creator", id);
///
/// // Retrieving a child element
/// let child_element = element.get_child("role").unwrap();
/// assert_eq!("aut", child_element.value());
/// ```
#[derive(Debug, PartialEq, Default)]
pub struct Element {
    pub(super) name: String,
    pub(super) value: String,
    pub(super) attributes: Vec<Attribute>,
    pub(super) children: Option<Vec<Element>>,
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

    /// Retrieve the value from a specified attribute. Namespace/prefix
    /// may be omitted from the argument.
    pub fn get_attribute(&self, name: &str) -> Option<&str> {
        self.attributes
            .iter()
            .find(|attribute| attribute.name().ends_with(&name.to_lowercase()))
            .map(|attribute| attribute.value.as_str())
    }

    /// Check if the element contains the specified attribute.
    /// Namespace/prefix may be omitted from the argument.
    pub fn contains_attribute(&self, name: &str) -> bool {
        self.attributes
            .iter()
            .any(|attribute| attribute.name().ends_with(&name.to_lowercase()))
    }

    /// Retrieve all child elements
    pub fn children(&self) -> Option<&[Element]> {
        self.children.as_deref()
    }

    /// Retrieve the specified child element. Namespace/prefix
    /// may be omitted from the argument.
    pub fn get_child(&self, name: &str) -> Option<&Element> {
        self.children().and_then(|children| {
            children
                .iter()
                .find(|child| child.name().ends_with(&name.trim().to_lowercase()))
        })
    }

    /// Check if the element contains the specified child element.
    /// Namespace/prefix may be omitted from the argument.
    pub fn contains_child(&self, name: &str) -> bool {
        self.children().map_or(false, |children| {
            children
                .iter()
                .any(|child| child.name().ends_with(&name.trim().to_lowercase()))
        })
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
/// let value = element.get_attribute("media-type").unwrap();
///
/// assert_eq!("application/xhtml+xml", value);
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
