use std::collections::HashMap;

use crate::formats::{epub::constants, xml::Element};
use crate::utility::Shared;
use crate::xml::Find;

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
/// let nested_children1 = element.children();
/// let nested_element1 = nested_children1.get(10).unwrap();
///
/// assert_eq!("John Ruskin", nested_element1.name());
/// assert_eq!("", nested_element1.value());
///
/// // Get further nested child element
/// let nested_children2 = nested_element1.children();
/// let nested_element2 = nested_children2.first().unwrap();
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
pub struct Toc(HashMap<String, Shared<Element>>);

impl Toc {
    pub(crate) fn new(element_map: HashMap<String, Shared<Element>>) -> Self {
        Self(element_map)
    }

    /// Retrieve toc elements in its nested form.
    ///
    /// # Epub2 navPoint
    /// The `playOrder` of `navPoint` elements from an `.ncx` file
    /// will not be checked if this method is called. However, on
    /// most cases `navPoint` elements are in proper order by default.
    ///
    /// To ensure `playOrder` for `navPoint` elements, use
    /// [elements_flat()](Self::elements_flat) instead.
    pub fn elements(&self) -> Vec<&Element> {
        self.get_elements(constants::TOC)
    }

    /// Retrieve toc elements in flattened form.
    pub fn elements_flat(&self) -> Vec<&Element> {
        let elements = self.get_elements_flat(constants::TOC);

        // Order navPoint elements
        if elements.first().map_or(false, |element| {
            element.contains_attribute(constants::PLAY_ORDER)
        }) {
            sort_nav_points(elements)
        } else {
            elements
        }
    }

    /// Retrieve landmark toc elements.
    pub fn landmarks(&self) -> Vec<&Element> {
        self.get_elements_flat(constants::LANDMARKS)
    }

    /// Retrieve page list toc elements that represent physical pages.
    pub fn page_list(&self) -> Vec<&Element> {
        self.get_elements_flat(constants::PAGE_LIST3)
    }

    // Gets the children elements from toc, page-list, landmarks, etc. elements.
    fn get_elements(&self, name: &str) -> Vec<&Element> {
        self.0
            .get(name)
            .map(|element| element.children())
            .unwrap_or_default()
    }

    fn get_elements_flat(&self, name: &str) -> Vec<&Element> {
        flatten(&self.get_elements(name))
    }
}

impl Find for Toc {
    fn __find_fallback(&self, _name: &str, _is_wild: bool) -> Vec<&Element> {
        self.0
            .values()
            .flat_map(|element| flatten(&element.children()))
            .collect()
    }
}

fn sort_nav_points(nav_points: Vec<&Element>) -> Vec<&Element> {
    let mut ordered_element: Vec<_> = nav_points
        .into_iter()
        .map(|nav_point| {
            let value: usize = nav_point
                .get_attribute(constants::PLAY_ORDER)
                .and_then(|play_order| play_order.parse().ok())
                .unwrap_or_default();

            (value, nav_point)
        })
        .collect();

    // Sort by nav point play order
    ordered_element.sort_by(|(order1, _), (order2, _)| order1.cmp(order2));
    ordered_element
        .into_iter()
        .map(|(_, nav_point)| nav_point)
        .collect()
}

fn flatten<'a>(elements: &[&'a Element]) -> Vec<&'a Element> {
    let mut output = Vec::new();
    let mut stack: Vec<_> = elements.iter().copied().rev().collect();

    while let Some(element) = stack.pop() {
        output.push(element);
        stack.extend(element.children().into_iter().rev());
    }

    output
}
