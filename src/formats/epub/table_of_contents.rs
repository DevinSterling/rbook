use std::collections::HashMap;

use crate::formats::xml::Element;

/// Table of contents (toc) for the ebook.
///
/// For convenience the value of the `text`/`label` and
/// `href`/`src` attributes are the `name` and `value`
/// fields of the element.
///
/// If an ebook offers both variants of the navigation
/// `table of contents` document, the epub3 version
/// will take priority over the legacy `ncx` epub2 variant.
///
/// # Examples
/// Getting an item from nested toc:
/// ```
/// use rbook::Ebook;
///
/// let epub = rbook::Epub::new("tests/ebooks/childrens-literature.epub").unwrap();
///
/// let toc_nested = epub.toc().elements();
///
/// // get element from toc
/// let element = toc_nested.first().unwrap();
///
/// assert_eq!("SECTION IV FAIRY STORIES—MODERN FANTASTIC TALES", element.name());
/// assert_eq!("s04.xhtml#pgepubid00492", element.value());
///
/// // Get nested child element
/// let nested_element1 = element.children().unwrap().get(10).unwrap();
///
/// assert_eq!("John Ruskin", nested_element1.name());
/// assert_eq!("", nested_element1.value());
///
/// // Get further nested child element
/// let nested_element2 = nested_element1.children().unwrap().first().unwrap();
///
/// assert_eq!("204 THE KING OF THE GOLDEN RIVER OR THE BLACK BROTHERS", nested_element2.name());
/// assert_eq!("s04.xhtml#pgepubid00602", nested_element2.value());
/// ```
///
/// Getting an item from flattened toc
/// ```
/// # use rbook::Ebook;
/// # let epub = rbook::Epub::new("tests/ebooks/childrens-literature.epub").unwrap();
/// let toc_flat = epub.toc().elements_flat();
///
/// // get element from flattened toc
/// let element = toc_flat.get(30).unwrap();
///
/// assert_eq!("204 THE KING OF THE GOLDEN RIVER OR THE BLACK BROTHERS", element.name());
/// assert_eq!("s04.xhtml#pgepubid00602", element.value());
/// ```
#[derive(Debug)]
pub struct Toc(pub(crate) HashMap<String, Element>);

impl Toc {
    /// Retrieve toc elements in its nested form
    pub fn elements(&self) -> &[Element] {
        self.get_elements("toc")
            .expect("Should have a toc element")
    }

    /// Retrieve toc elements in flattened form
    pub fn elements_flat(&self) -> Vec<&Element> {
        self.get_elements_flat("toc")
            .expect("Should have a toc element")
    }

    pub fn landmarks(&self) -> Option<Vec<&Element>> {
        self.get_elements_flat("landmarks")
    }

    /// Retrieve page list elements that represent physical pages
    pub fn page_list(&self) -> Option<Vec<&Element>> {
        self.get_elements_flat("page-list")
    }

    fn get_elements(&self, name: &str) -> Option<&[Element]> {
        if let Some(elements) = self.0.get(name) {
            Some(elements.children().expect("Should have nav children elements"))
        } else {
            None
        }
    }

    fn get_elements_flat(&self, name: &str) -> Option<Vec<&Element>> {
        if let Some(elements) = self.get_elements(name) {
            let mut output = Vec::new();
            recursive_flatten(elements, &mut output);

            Some(output)
        } else {
            None
        }
    }
}

fn recursive_flatten<'a>(elements: &'a [Element], output: &mut Vec<&'a Element>) {
    for element in elements {
        output.push(element);

        if let Some(children) = element.children() {
            recursive_flatten(children, output);
        }
    }
}