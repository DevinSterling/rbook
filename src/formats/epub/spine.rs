use crate::formats::xml::{Attribute, Element};

/// For convenience the value of the idref attribute is the name
/// field of the element
///
/// # Examples
/// Getting an item from the spine:
/// ```
/// use rbook::Ebook;
///
/// let epub = rbook::Epub::new("example.epub").unwrap();
///
/// // get element in the manifest
/// let element = epub.spine().elements().get(31).unwrap();
///
/// // Get idref from the element
/// let idref = element.name();
///
/// assert_eq!("chapter009a", idref);
/// ```
#[derive(Debug)]
pub struct Spine(pub(crate) Element);

impl Spine {
    /// Retrieve all spine `itemref` elements
    pub fn elements(&self) -> &[Element] {
        self.0.children.as_deref().unwrap_or_default()
    }

    /// Retrieve all the attributes of the root spine element
    pub fn attributes(&self) -> &[Attribute] {
        self.0.attributes()
    }

    /// Retrieve a certain attribute from the root spine element
    pub fn get_attribute(&self, name: &str) -> Option<&Attribute> {
        self.0.get_attribute(name)
    }

    /// Check if an attribute in the root spine element exists
    pub fn contains_attribute(&self, name: &str) -> bool {
        self.0.contains_attribute(name)
    }
}