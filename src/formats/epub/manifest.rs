use std::collections::HashMap;

use crate::formats::epub::constants;
use crate::formats::xml::{self, Element};
use crate::xml::Find;

/// Access all resources for the ebook, such as images, files, etc.
///
/// For convenience the value of the `id` and `href` attributes are the
/// `name` and `value` fields of the element.
///
/// # Examples
/// Getting an item from the manifest:
/// ```
/// use rbook::Ebook;
///
/// let epub = rbook::Epub::new("tests/ebooks/moby-dick.epub").unwrap();
///
/// // Get element in the manifest
/// let element = epub.manifest().by_id("xchapter_009").unwrap();
///
/// // Get id and href from the element
/// let id = element.name();
/// let href = element.value();
///
/// assert_eq!("xchapter_009", id);
/// assert_eq!("chapter_009.xhtml", href);
/// ```
#[derive(Debug)]
pub struct Manifest(HashMap<String, Element>);

impl Manifest {
    pub(crate) fn new(element_map: HashMap<String, Element>) -> Self {
        Self(element_map)
    }

    /// Retrieve all manifest `item` elements.
    pub fn elements(&self) -> Vec<&Element> {
        let mut sorted_elements: Vec<_> = self.0.values().collect();
        sorted_elements.sort_by_key(|e| &e.name);
        sorted_elements
    }

    /// Retrieve all elements that reference an image media type file.
    /// The returned vector contains at least one element.
    ///
    /// Image types retrieved may be:
    /// - svg
    /// - png
    /// - jpeg
    ///
    /// # Examples
    /// Basic usage:
    /// ```
    /// # use rbook::Ebook;
    /// # let epub = rbook::Epub::new("tests/ebooks/moby-dick.epub").unwrap();
    /// // Retrieving elements
    /// let images = epub.manifest().images();
    /// let cover_image = epub.cover_image().unwrap();
    ///
    /// assert!(images.contains(&cover_image));
    /// ```
    ///
    /// Retrieving image data:
    /// ```
    /// # use rbook::Ebook;
    /// # let epub = rbook::Epub::new("tests/ebooks/childrens-literature.epub").unwrap();
    /// let images = epub.manifest().images();
    ///
    /// for image_element in images {
    ///     let image_href = image_element.value();
    ///     let image_data = epub.read_bytes_file(image_href).unwrap();
    /// }
    /// ```
    pub fn images(&self) -> Vec<&Element> {
        self.0
            .values()
            .filter(|element| {
                element
                    .get_attribute(constants::MEDIA_TYPE)
                    .map_or(false, |attribute| attribute.starts_with("image"))
            })
            .collect()
    }

    /// Retrieve a certain element by the value of its `id` from the manifest
    pub fn by_id(&self, id: &str) -> Option<&Element> {
        self.0.get(id)
    }

    /// Retrieve a certain element by the value of its `href` from the manifest
    pub fn by_href(&self, href: &str) -> Option<&Element> {
        xml::utility::find_attribute_by_value(&self.elements(), xml::HREF, href)
    }

    /// Check if an element with a certain `id` exists in the manifest
    pub fn contains_id(&self, id: &str) -> bool {
        self.0.contains_key(id)
    }

    /// Retrieve a certain element by the value of its
    /// `media type` from the manifest
    pub fn by_media_type(&self, media_type: &str) -> Option<&Element> {
        xml::utility::find_attribute_by_value(&self.elements(), constants::MEDIA_TYPE, media_type)
    }

    /// Retrieve all elements that match a given `media type`
    /// from the manifest. The returned vector contains at
    /// least one element.
    pub fn all_by_media_type(&self, media_type: &str) -> Vec<&Element> {
        xml::utility::find_attributes_by_value(&self.elements(), constants::MEDIA_TYPE, media_type)
    }

    /// Retrieve a certain element by the value of its `property`
    /// from the manifest
    pub fn by_property(&self, property: &str) -> Option<&Element> {
        xml::utility::find_attribute_by_value(&self.elements(), constants::PROPERTIES, property)
    }

    /// Retrieve all elements that match a given `property` value
    /// from the manifest. The returned vector contains at least
    /// one element.
    pub fn all_by_property(&self, property: &str) -> Vec<&Element> {
        xml::utility::find_attributes_by_value(&self.elements(), constants::PROPERTIES, property)
    }
}

impl Find for Manifest {
    fn __find_fallback(&self, field: &str, is_wildcard: bool) -> Vec<&Element> {
        match is_wildcard {
            true => self.elements(),
            false => self
                .by_id(field)
                .map(|field| vec![field])
                .unwrap_or_default(),
        }
    }
}
