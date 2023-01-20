use std::collections::HashMap;

use crate::formats::xml::Element;

/// Table of contents (toc) for the ebook.
///
/// For convenience, when nav elements are encountered,
/// the value of the text/label and href/src attributes
/// are the name and value fields of the element.
///
/// If an ebook offers both variants of the navigation
/// "table of contents" document, the Epub version 3 navigation
/// document will take priority over the legacy opf epub2 variant.
///
/// # Examples
/// Getting an item from the toc:
/// ```
/// use rbook::Ebook;
///
/// let epub = rbook::Epub::new("example.epub").unwrap();
///
/// // get element in the manifest
/// let element = epub.toc().elements().get(10).unwrap();
///
/// // Get label and href from the element
/// let label = element.name();
/// let href = element.value();
///
/// assert_eq!("Chapter 6", label);
/// assert_eq!("chapter006.xhtml", href);
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

        if let Some(children) = &element.children {
            recursive_flatten(children, output);
        }
    }
}