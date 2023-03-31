use std::borrow::Borrow;
use std::ops::Deref;

use crate::utility::{Shared, Weak};

pub(crate) mod utility;

pub(super) const ID: &str = "id";
pub(super) const HREF: &str = "href";
pub(super) const SRC: &str = "src";

/// Conveniently find elements or their value using very
/// basic CSS selector-like strings.
///
/// - [Self::find] Finds the first element that matches.
/// - [Self::find_all] Finds all elements that match.
/// - [Self::find_value] Finds the first element that matches
/// and returns its value.
/// - [Self::final_all_value] Finds all elements that match and
/// returns their values.
///
/// # Basic syntax:
/// `{element name}[attribute=value][...] > {direct descendant
/// element} > {...}`
///
/// - [Element]s are separated by a greater than symbol `>`
/// and are identified by their [name](Element::name).
///     - Element names can be foregone using a wildcard
/// symbol `*`.
/// - Elements that follow any other elements must be its
/// direct descendant.
/// - [Attribute]s are optional and may be encased within
/// brackets after the name of an element.
///     - Supplying only an attribute [name](Attribute::name),
/// only checks if the element contains the attribute.
///     - Supplying the attribute name and
/// [value](Attribute::value) separated by an equals symbol `=`,
/// will then check if the attribute equals the supplied value.
///
/// # Examples:
/// Searching using [element names](Element::name) and wildcards `*`:
/// ```
/// # use rbook::Ebook;
/// # let epub = rbook::Epub::new("tests/ebooks/childrens-literature.epub").unwrap();
/// # use rbook::xml::Find;
/// // Find all `creator` elements
/// let _creator = epub.metadata().find_all("creator");
///
/// // Find the first `creator` element that has a child `file-as` element
/// let creator1 = epub.metadata().find("creator > file-as").unwrap();
///
/// // Find the first element that has a child `file-as` element
/// let creator2 = epub.metadata().find("* > file-as").unwrap();
/// assert_eq!(creator1, creator2)
/// ```
/// Searching using attribute [names](Attribute::name) and
/// [values](Attribute::value) encased within square brackets:
/// ```
/// # use rbook::Ebook;
/// # let epub = rbook::Epub::new("tests/ebooks/childrens-literature.epub").unwrap();
/// # use rbook::xml::Find;
/// // Find the `creator` element that has an `id` attribute that equals `clippinger` and a
/// // child `file-as` element where its `refines` attribute equals `#clippinger`
/// let creator1 = epub.metadata().find("creator[id=clippinger] > file-as[refines=#clippinger]").unwrap();
///
/// // Find any element that has an `id` attribute and a child `file-as` element that has a
/// // property attribute with any value a `refines` attribute that equals `#clippinger`
/// let creator2 = epub.metadata().find("*[id] > file-as[property][refines=#clippinger]").unwrap();
/// assert_eq!(creator1, creator2)
/// ```
pub trait Find {
    /// Retrieve the first [Element] that matches the given input.
    /// Prefixes/namespaces are optional.
    ///
    /// To retrieve all found elements, use
    /// [find_all(...)](Self::find_all) instead.
    ///
    /// Alternatively, [find_value(...)](Self::find_value)
    /// can be used to retrieve the [value](Element::value)
    /// of a single element directly.
    ///
    /// # Examples
    /// Basic Usage:
    /// ```
    /// # use rbook::Ebook;
    /// # let epub = rbook::Epub::new("tests/ebooks/moby-dick.epub").unwrap();
    /// use rbook::xml::Find;
    ///
    /// let creator = epub.metadata().find("creator > role").unwrap();
    /// assert_eq!("aut", creator.value());
    /// ```
    fn find(&self, input: &str) -> Option<&Element> {
        self.find_all(input).into_iter().next()
    }

    /// Retrieve all [Element]s that match the given input.
    /// Prefixes/namespaces are optional.
    ///
    /// To retrieve a single element, use [find(...)](Self::find)
    /// instead.
    ///
    /// Alternatively, [find_all_value(...)](Self::final_all_value)
    /// can be used to retrieve the [values](Element::value) of all
    /// elements directly.
    fn find_all(&self, input: &str) -> Vec<&Element> {
        utility::find_helper(input, |name, is_wild| self.__find_fallback(name, is_wild))
            .unwrap_or_default()
    }

    /// Retrieve the first [value](Element::value) of an [Element]
    /// directly that matches the given input in the form of a string.
    /// Prefixes/namespaces are optional.
    ///
    /// To retrieve all found strings values, use
    /// [find_all_value(...)](Self::final_all_value) instead.
    ///
    /// Alternatively, [find(...)](Self::find) can be used to retrieve
    /// a single [Element].
    ///
    /// # Examples
    /// Basic usage:
    /// ```
    /// use rbook::Ebook;
    /// let epub = rbook::Epub::new("tests/ebooks/moby-dick.epub").unwrap();
    /// use rbook::xml::Find;
    ///
    /// let creator = epub.metadata().find_value("creator").unwrap();
    /// let role = epub.metadata().find_value("creator > role").unwrap();
    ///
    /// assert_eq!("Herman Melville", creator);
    /// assert_eq!("aut", role);
    /// ```
    fn find_value(&self, input: &str) -> Option<&str> {
        utility::find_helper(input, |name, is_wild| self.__find_fallback(name, is_wild))
            .and_then(|vec| vec.first().map(|element| element.value()))
    }

    /// Retrieve all string values from [Element]s directly that
    /// match the given input.
    /// Prefixes/namespaces are optional.
    ///
    /// To retrieve a single value, use
    /// [find_value(...)](Self::find_value) instead.
    ///
    /// Alternatively, [find_all(...)](Self::find_all)
    /// can be used to retrieve all [Element]s.
    fn final_all_value(&self, input: &str) -> Vec<&str> {
        utility::find_helper(input, |name, is_wild| self.__find_fallback(name, is_wild))
            .and_then(|vec| {
                vec.into_iter()
                    .map(|element| Some(element.value()))
                    .collect()
            })
            .unwrap_or_default()
    }

    // field: Name of element to search for
    // is_wild: Whether to check field names of elements
    fn __find_fallback(&self, name: &str, is_wild: bool) -> Vec<&Element>;
}

// Temporary container for mutable elements during construction before
// conversion to their immutable counter-part, `Element`.
#[derive(Debug, Default)]
pub(crate) struct TempElement {
    pub(super) name: String,
    pub(super) value: String,
    pub(super) attributes: Vec<Attribute>,
    pub(super) children: Option<Vec<TempElement>>,
}

impl TempElement {
    pub(crate) fn get_attribute(&self, name: &str) -> Option<&str> {
        utility::get_attribute(&self.attributes, name)
    }

    pub(crate) fn contains_attribute(&self, name: &str) -> bool {
        utility::contains_attribute(&self.attributes, name)
    }

    pub(crate) fn convert_to_shared(self, parent: Weak<Element>) -> Shared<Element> {
        Shared::new_cyclic(|weak| {
            let children = self.children.map(|vec| {
                vec.into_iter()
                    .map(|child| child.convert_to_shared(weak.clone()))
                    .collect()
            });

            Element {
                name: self.name,
                value: self.value,
                attributes: self.attributes,
                children,
                parent,
            }
        })
    }
}

/// Representation of an xml element, where its attributes,
/// children, values, and name are accessible.
///
/// # Examples
/// Basic Usage:
/// ```
/// # use rbook::Ebook;
/// # let epub = rbook::Epub::new("tests/ebooks/moby-dick.epub").unwrap();
/// // Retrieving an element from the metadata of an epub
/// let creators = epub.metadata().creators();
/// let element = creators.first().unwrap();
///
/// // Retrieving an attribute
/// let id = element.get_attribute("id").unwrap();
/// assert_eq!("creator", id);
///
/// // Retrieving a child element
/// let child_element = element.get_child("role").unwrap();
/// assert_eq!("aut", child_element.value());
/// ```
#[derive(Debug, Default)]
pub struct Element {
    pub(super) name: String,
    pub(super) value: String,
    pub(super) attributes: Vec<Attribute>,
    pub(super) children: Option<Vec<Shared<Element>>>,
    pub(super) parent: Weak<Element>,
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
        utility::get_attribute(self.attributes(), &name.to_lowercase())
    }

    /// Check if the element contains the specified attribute.
    /// Namespace/prefix may be omitted from the argument.
    pub fn contains_attribute(&self, name: &str) -> bool {
        utility::contains_attribute(self.attributes(), &name.to_lowercase())
    }

    /// Retrieve the parent element
    pub fn parent(&self) -> Option<Parent> {
        self.parent.upgrade().map(Parent)
    }

    /// Retrieve all child elements
    pub fn children(&self) -> Vec<&Element> {
        self.children
            .as_ref()
            .map(|elements| elements.iter().map(Shared::borrow).collect())
            .unwrap_or_default()
    }

    /// Retrieve the specified child element. Namespace/prefix
    /// may be omitted from the argument.
    pub fn get_child(&self, name: &str) -> Option<&Element> {
        let name = name.trim().to_lowercase();

        self.children()
            .into_iter()
            .find(|child| child.name().to_lowercase().ends_with(&name))
    }

    /// Check if the element contains the specified child element.
    /// Namespace/prefix may be omitted from the argument.
    pub fn contains_child(&self, name: &str) -> bool {
        let name = name.trim().to_lowercase();

        self.children()
            .into_iter()
            .any(|child| child.name().to_lowercase().ends_with(&name))
    }
}

impl PartialEq for Element {
    fn eq(&self, other: &Self) -> bool {
        self.name() == other.name()
            && self.value() == other.value()
            && self.children() == other.children()
            && self.attributes() == other.attributes()
    }
}

impl Find for Element {
    fn __find_fallback(&self, name: &str, is_wildcard: bool) -> Vec<&Element> {
        match is_wildcard {
            true => self.children(),
            false => self
                .get_child(name)
                .map(|child| vec![child])
                .unwrap_or_default(),
        }
    }
}

// Wrapper struct for abstraction. Hides Shared<T>
/// Parent element that is retrieved from a child element.
///
/// Basic Usage:
/// ```
/// # use rbook::Ebook;
/// # use rbook::xml::Find;
/// # let epub = rbook::Epub::new("tests/ebooks/moby-dick.epub").unwrap();
/// // Retrieving an element from the metadata of an epub
/// let element = epub.metadata().find("creator > file-as").unwrap();
/// assert_eq!("MELVILLE, HERMAN", element.value());
///
/// // Retrieving the parent element
/// let parent = element.parent().unwrap();
/// assert_eq!("Herman Melville", parent.value());
///
/// let element2 = epub.metadata().find("creator").unwrap();
/// assert_eq!(&*parent, element2);
/// ```
#[derive(Debug, PartialEq)]
pub struct Parent(Shared<Element>);

impl Deref for Parent {
    type Target = Element;

    fn deref(&self) -> &Element {
        self.0.as_ref()
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
    name: String,
    value: String,
}

impl Attribute {
    pub(crate) fn new(name: String, value: String) -> Self {
        Self { name, value }
    }

    pub fn name(&self) -> &str {
        &self.name
    }

    pub fn value(&self) -> &str {
        &self.value
    }
}
