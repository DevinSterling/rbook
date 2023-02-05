use std::borrow::Borrow;
use std::rc::Rc;

use crate::formats::xml::{Attribute, Element};
use crate::xml::Find;

/// Access the order of resources for the ebook.
///
/// For convenience the value of the `idref` attribute is the `name`
/// field of the element.
///
/// # Examples
/// Getting an item from the spine:
/// ```
/// use rbook::Ebook;
///
/// let epub = rbook::Epub::new("tests/ebooks/moby-dick.epub").unwrap();
///
/// // Get element in the spine
/// let spine_elements = epub.spine().elements();
/// let element = spine_elements.get(31).unwrap();
///
/// // Get idref from the element
/// let idref = element.name();
///
/// assert_eq!("xchapter_026", idref);
/// ```
#[derive(Debug)]
pub struct Spine(Rc<Element>);

impl Spine {
    pub(crate) fn new(spine_element: Rc<Element>) -> Self {
        Self(spine_element)
    }

    /// Retrieve all spine `itemref` elements
    pub fn elements(&self) -> Vec<&Element> {
        self.0
            .children
            .as_ref()
            .map(|elements| elements.iter().map(Rc::borrow).collect())
            .unwrap_or_default()
    }

    /// Retrieve all the attributes of the root spine element
    pub fn attributes(&self) -> &[Attribute] {
        self.0.attributes()
    }

    /// Retrieve the value from a certain attribute from the root spine element
    pub fn get_attribute(&self, name: &str) -> Option<&str> {
        self.0.get_attribute(name)
    }

    /// Check if an attribute in the root spine element exists
    pub fn contains_attribute(&self, name: &str) -> bool {
        self.0.contains_attribute(name)
    }
}

impl Find for Spine {
    fn find_fallback(&self, _name: &str, _is_wild: bool) -> Option<Vec<&Element>> {
        Some(self.elements())
    }
}
