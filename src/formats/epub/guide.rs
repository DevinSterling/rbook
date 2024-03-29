use crate::formats::epub::constants;
use crate::formats::xml::{self, Element};
use crate::xml::Find;

/// Access important structural portions of the ebook.
///
/// Primarily used by epub2. Access to epub3 landmarks is
/// accessible using the [landmarks()](super::Toc::landmarks) method in [Toc](super::Toc).
///
/// For convenience the value of the `title` and `href` attributes are
/// the `name` and `value` fields of the element.
///
/// # Examples
/// Accessing the Guide:
/// ```
/// use rbook::Ebook;
///
/// let epub = rbook::Epub::new("tests/ebooks/moby-dick.epub").unwrap();
///
/// // get element in the manifest
/// let guide = epub.guide().elements();
///
/// // Print title and href of all guide elements
/// for element in guide {
///     println!("title:{}, href:{}", element.name(), element.value());
/// }
/// ```
#[derive(Debug)]
pub struct Guide(Vec<Element>);

impl Guide {
    pub(crate) fn new(elements: Vec<Element>) -> Self {
        Self(elements)
    }

    /// Retrieve all `guide` elements
    pub fn elements(&self) -> Vec<&Element> {
        self.0.iter().collect()
    }

    /// Retrieve a certain element by the value of its `type`
    /// from the guide
    pub fn by_type(&self, property: &str) -> Option<&Element> {
        xml::utility::find_attribute_by_value(&self.elements(), constants::TYPE, property)
    }

    /// Retrieve all elements that match a given `type` value
    /// from the guide. The returned vector contains at least
    /// one element.
    pub fn all_by_type(&self, property: &str) -> Vec<&Element> {
        xml::utility::find_attributes_by_value(&self.elements(), constants::TYPE, property)
    }
}

impl Find for Guide {
    fn __find_fallback(&self, _name: &str, _is_wildcard: bool) -> Vec<&Element> {
        self.elements()
    }
}
