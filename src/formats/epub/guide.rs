use crate::formats::xml::{self, Element};

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
/// let epub = rbook::Epub::new("example.epub").unwrap();
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
pub struct Guide(pub(crate) Vec<Element>);

impl Guide {
    pub fn elements(&self) -> &[Element] {
        &self.0
    }

    /// Retrieve a certain element by the value of its `type`
    /// from the guide
    pub fn by_type(&self, property: &str) -> Option<&Element> {
        self.find_attribute_by_value("type", property)
    }

    /// Retrieve all elements that match a given `type` value
    /// from the guide. The returned vector contains at least
    /// one element.
    pub fn all_by_type(&self, property: &str) -> Option<Vec<&Element>> {
        self.find_attributes_by_value("type", property)
    }

    fn find_attribute_by_value(&self, field: &str, value: &str) -> Option<&Element> {
        self.elements().iter().find(|element| {
            xml::utility::equals_attribute_by_value(element, field, value)
        })
    }

    fn find_attributes_by_value(&self, field: &str, value: &str) -> Option<Vec<&Element>> {
        let vec: Vec<_> = self.elements().iter().filter(|element| {
            xml::utility::equals_attribute_by_value(element, field, value)
        }).collect();

        if vec.is_empty() {
            None
        } else {
            Some(vec)
        }
    }
}