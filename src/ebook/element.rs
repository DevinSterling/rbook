//! General `XML` element-related types.

use crate::util::{StringExt, uri};
use std::borrow::Cow;
use std::fmt::{Debug, Display, Formatter};
use std::hash::Hash;

/// The percent-encoded `href` of an element, pointing to a location.
///
/// To retrieve the percent-decoded form, see [`Href::decode`].
#[derive(Copy, Clone, Debug, Hash, PartialEq, Eq)]
pub struct Href<'a>(&'a str);

// Methods most relevant for EPUBs
impl<'a> Href<'a> {
    /// Returns the percent-decoded form.
    ///
    /// # Examples
    /// - Decoding an href:
    /// ```
    /// # use rbook::{Ebook, Epub};
    /// # use rbook::ebook::errors::EbookResult;
    /// # fn main() -> EbookResult<()> {
    /// let epub = Epub::open("tests/ebooks/example_epub")?;
    /// let href = epub.manifest().by_id("style").unwrap().href();
    ///
    /// assert_eq!("/file%20name%20with%20spaces.css", href.as_str());
    /// assert_eq!("/file name with spaces.css", href.decode());
    /// # Ok(())
    /// # }
    /// ```
    pub fn decode(&self) -> Cow<'a, str> {
        uri::decode(self.0)
    }

    /// The href with **only** the query (`?`) and fragment (`#`) omitted.
    ///
    /// An href such as `s04.xhtml#pgepubid00588` will become `s04.xhtml`.
    ///
    /// See also:
    /// - [`Self::fragment`]
    /// - [`Self::query`]
    ///
    /// # Examples
    /// - Omitting the query and fragment:
    /// ```
    /// # use rbook::{Ebook, Epub};
    /// # use rbook::ebook::errors::EbookResult;
    /// # use rbook::ebook::toc::{Toc, TocChildren, TocEntry};
    /// # fn main() -> EbookResult<()> {
    /// let epub = Epub::open("tests/ebooks/example_epub")?;
    /// let contents = epub.toc().contents().unwrap();
    /// let toc_entry = contents.children().get(1).unwrap();
    /// let href = toc_entry.href().unwrap();
    ///
    /// assert_eq!("/EPUB/c1.xhtml?q=1#start", href.as_str());
    /// assert_eq!("/EPUB/c1.xhtml", href.path().as_str());
    /// # Ok(())
    /// # }
    /// ```
    pub fn path(&self) -> Self {
        self.0
            .find(['#', '?'])
            .map_or(self.0, |index| &self.0[..index])
            .into()
    }

    /// The content of a fragment (`#`) within an href.
    ///
    /// # Examples
    /// - Retrieving the fragment content:
    /// ```
    /// # use rbook::{Ebook, Epub};
    /// # use rbook::ebook::errors::EbookResult;
    /// # use rbook::ebook::toc::{Toc, TocChildren, TocEntry};
    /// # fn main() -> EbookResult<()> {
    /// # let epub = Epub::open("tests/ebooks/example_epub")?;
    /// # let contents = epub.toc().contents().unwrap();
    /// # let toc_entry = contents.children().get(1).unwrap();
    /// let href = toc_entry.href().unwrap();
    ///
    /// assert_eq!("/EPUB/c1.xhtml?q=1#start", href.as_str());
    /// assert_eq!(Some("start"), href.fragment());
    /// # Ok(())
    /// # }
    /// ```
    pub fn fragment(&self) -> Option<&str> {
        self.0.find('#').map(|index| &self.0[index + 1..])
    }

    /// The content of a query (`?`) within an href.
    ///
    /// # Examples
    /// - Retrieving the query content:
    /// ```
    /// # use rbook::{Ebook, Epub};
    /// # use rbook::ebook::errors::EbookResult;
    /// # use rbook::ebook::toc::{Toc, TocChildren, TocEntry};
    /// # fn main() -> EbookResult<()> {
    /// # let epub = Epub::open("tests/ebooks/example_epub")?;
    /// # let contents = epub.toc().contents().unwrap();
    /// # let toc_entry = contents.children().get(1).unwrap();
    /// let href = toc_entry.href().unwrap();
    ///
    /// assert_eq!("/EPUB/c1.xhtml?q=1#start", href.as_str());
    /// assert_eq!(Some("q=1"), href.query());
    /// # Ok(())
    /// # }
    /// ```
    pub fn query(&self) -> Option<&str> {
        self.0
            .find('?')
            .and_then(|query| self.0[query + 1..].split('#').next())
    }

    /// The underlying `href` string.
    pub fn as_str(&self) -> &'a str {
        self.0
    }
}

impl<'a> AsRef<str> for Href<'a> {
    fn as_ref(&self) -> &'a str {
        self.0
    }
}

impl<'a> From<&'a str> for Href<'a> {
    fn from(value: &'a str) -> Self {
        Self(value)
    }
}

/// A collection of properties associated with an element.
///
/// # Examples
/// - Retrieving the properties from a navigation resource:
/// ```
/// # use rbook::{Ebook, Epub};
/// # use rbook::ebook::errors::EbookResult;
/// # use rbook::ebook::manifest::Manifest;
/// # fn main() -> EbookResult<()> {
/// let epub = Epub::open("tests/ebooks/example_epub")?;
/// let nav_xhtml = epub.manifest().by_property("nav").next().unwrap();
/// let properties = nav_xhtml.properties();
///
/// assert_eq!("scripted nav", properties.as_str());
/// assert_eq!(true, properties.has_property("nav"));
/// assert_eq!(true, properties.has_property("scripted"));
/// assert_eq!(false, properties.has_property("ncx"));
/// # Ok(())
/// # }
/// ```
#[derive(Copy, Clone, Debug, Hash, PartialEq, Eq)]
pub struct Properties<'a>(&'a PropertiesData);

impl<'a> Properties<'a> {
    /// The number of property entries contained within.
    pub fn len(&self) -> usize {
        self.iter().count()
    }

    /// Returns `true` if there are no properties, otherwise `false`.
    pub fn is_empty(&self) -> bool {
        self.as_str().trim().is_empty()
    }

    /// Returns the associated property if the provided `index` is less than
    /// [`Self::len`], otherwise [`None`].
    pub fn get(&self, index: usize) -> Option<&'a str> {
        self.0.iter().nth(index)
    }

    /// Returns an iterator over **all** properties.
    pub fn iter(&self) -> impl Iterator<Item = &'a str> + 'a {
        self.0.iter()
    }

    /// Returns `true` if the provided property is present, otherwise `false`.
    pub fn has_property(&self, property: &str) -> bool {
        self.0.has_property(property)
    }

    /// The underlying raw properties.
    pub fn as_str(&self) -> &str {
        self.0.0.as_str()
    }
}

impl<'a> AsRef<str> for Properties<'a> {
    fn as_ref(&self) -> &'a str {
        self.0.0.as_str()
    }
}

impl<'a> From<&'a PropertiesData> for Properties<'a> {
    fn from(properties: &'a PropertiesData) -> Self {
        Self(properties)
    }
}

/// A collection of [`Attribute`] entries associated with an element.
#[derive(Copy, Clone, Debug, Hash, PartialEq, Eq)]
pub struct Attributes<'a>(&'a [AttributeData]);

impl<'a> Attributes<'a> {
    /// The number of attribute entries contained within.
    pub fn len(&self) -> usize {
        self.0.len()
    }

    /// Returns `true` if there are no attributes, otherwise `false`.
    pub fn is_empty(&self) -> bool {
        self.0.is_empty()
    }

    /// Returns the associated [`Attribute`] if the provided `index` is less than
    /// [`Self::len`], otherwise [`None`].
    pub fn get(&self, index: usize) -> Option<Attribute<'a>> {
        self.0.get(index).map(Attribute)
    }

    /// Returns an iterator over **all** [`Attribute`] entries.
    pub fn iter(&self) -> impl Iterator<Item = Attribute<'a>> + 'a {
        // AttributeData is not returned directly here
        // to allow greater flexibility in the future.
        self.0.iter().map(Attribute)
    }

    /// Returns the [`Attribute`] with the given `name` if present,
    /// otherwise [`None`].
    pub fn by_name(&self, name: &str) -> Option<Attribute<'a>> {
        self.0
            .iter()
            .find(|data| data.name.as_str().eq_ignore_ascii_case(name))
            .map(Attribute)
    }

    /// Returns `true` if an [`Attribute`] with the given `name` is present, otherwise `false`.
    pub fn has_name(&self, name: &str) -> bool {
        self.0
            .iter()
            .any(|data| data.name.as_str().eq_ignore_ascii_case(name))
    }
}

impl<'a> From<&'a Vec<AttributeData>> for Attributes<'a> {
    fn from(attributes: &'a Vec<AttributeData>) -> Self {
        Self(attributes)
    }
}

/// An attribute, containing details about an element.
///
/// An attribute encompasses a [`Name`] as `key` and a string as `value`.
/// The name and value have their leading and trailing ASCII whitespace trimmed at parse time.
#[derive(Copy, Clone, Debug, Hash, PartialEq, Eq)]
pub struct Attribute<'a>(
    /// Wraps [`AttributeData`] **for now**.
    /// However, this could change in the future.
    &'a AttributeData,
);

impl<'a> Attribute<'a> {
    /// The attribute name/key.
    pub fn name(&self) -> Name<'a> {
        Name(&self.0.name)
    }

    /// The attribute value.
    pub fn value(&self) -> &'a str {
        &self.0.value
    }
}

/// The name of an element or [`Attribute`].
#[derive(Copy, Clone, Debug, Hash, PartialEq, Eq)]
pub struct Name<'a>(&'a str);

impl<'a> Name<'a> {
    fn index(&self) -> Option<usize> {
        self.0.find(':')
    }

    /// The prefix of a name.
    ///
    /// For example, the name `dcterms:modified` has a prefix of `dcterms`.
    pub fn prefix(&self) -> Option<&str> {
        self.index().map(|i| &self.0[..i])
    }

    /// The name with its prefix stripped.
    ///
    /// For example, the name `dcterms:modified` with its prefix stripped is `modified`.
    pub fn local(&self) -> &str {
        self.index().map_or(self.0, |i| &self.0[i + 1..])
    }

    /// The raw name with its prefix intact (e.g., `dcterms:modified`).
    pub fn as_str(&self) -> &str {
        self.0
    }
}

impl AsRef<str> for Name<'_> {
    fn as_ref(&self) -> &str {
        self.0
    }
}

impl<'a> From<&'a str> for Name<'a> {
    fn from(name: &'a str) -> Self {
        Self(name)
    }
}

impl Display for Name<'_> {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        f.write_str(self.0)
    }
}

/// A hint for text directionality, indicating whether content **should** be
/// read from left-to-right (`ltr`), right-to-left (`rtl`), or automatically
/// (`auto`).
///
/// [`TextDirection::as_str`] can be used to retrieve the string form.
///
/// Default: [`TextDirection::Auto`]
#[derive(Copy, Clone, Debug, Default, Hash, PartialEq, Eq)]
pub enum TextDirection {
    /// Process textual content from left-to-right (`ltr`).
    LeftToRight,
    /// Process textual content from right-to-left (`rtl`).
    RightToLeft,
    /// No text directionality specified (`auto`).
    ///
    /// In this state, an application may rely on directionality
    /// determined by the Unicode Bidi Algorithm.
    #[default]
    Auto,
}

impl TextDirection {
    const AUTO: &'static str = "auto";
    const LEFT_TO_RIGHT: &'static str = "ltr";
    const RIGHT_TO_LEFT: &'static str = "rtl";

    /// Returns the string representation of a [`TextDirection`] hint.
    ///
    /// # Examples
    /// - Observing the string representations:
    /// ```
    /// # use rbook::ebook::element::TextDirection;
    /// assert_eq!("ltr", TextDirection::LeftToRight.as_str());
    /// assert_eq!("rtl", TextDirection::RightToLeft.as_str());
    /// assert_eq!("auto", TextDirection::Auto.as_str());
    /// ```
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::LeftToRight => Self::LEFT_TO_RIGHT,
            Self::RightToLeft => Self::RIGHT_TO_LEFT,
            Self::Auto => Self::AUTO,
        }
    }
}

impl<A: AsRef<str>> From<A> for TextDirection {
    fn from(value: A) -> Self {
        match value.as_ref() {
            Self::LEFT_TO_RIGHT => Self::LeftToRight,
            Self::RIGHT_TO_LEFT => Self::RightToLeft,
            _ => Self::Auto,
        }
    }
}

#[derive(Clone, Debug, Default, Hash, PartialEq, Eq)]
pub(crate) struct PropertiesData(String);

impl PropertiesData {
    /// Adds the given property if it is not contained within.
    pub(crate) fn add_property(&mut self, property: &str) {
        if !self.has_property(property) {
            self.0.push(' ');
            self.0.push_str(property);
        }
    }

    pub fn iter(&self) -> impl Iterator<Item = &str> {
        self.0.split_whitespace()
    }

    pub fn has_property(&self, property: &str) -> bool {
        self.iter().any(|value| value == property)
    }
}

impl From<Option<String>> for PropertiesData {
    fn from(value: Option<String>) -> Self {
        value.map(PropertiesData).unwrap_or_default()
    }
}

#[derive(Clone, Debug, Hash, PartialEq, Eq)]
pub(crate) struct AttributeData {
    name: String,
    value: String,
}

impl AttributeData {
    pub(crate) fn new<'a>(name: impl Into<Cow<'a, str>>, value: impl Into<Cow<'a, str>>) -> Self {
        let mut value = value.into().into_owned();
        value.trim_in_place();

        AttributeData {
            name: name.into().into_owned(),
            value,
        }
    }
}

pub(crate) fn get_attribute<'a>(attributes: &'a [AttributeData], key: &str) -> Option<&'a str> {
    attributes
        .iter()
        .find_map(|attribute| (attribute.name.as_str() == key).then_some(attribute.value.as_str()))
}

#[cfg(test)]
mod tests {
    use crate::ebook::element::Href;

    #[test]
    fn test_href_path() {
        let expected = [
            ("s04.xhtml", "s04.xhtml#pgepubid00588"),
            ("/EPUB/c1.xhtml", "/EPUB/c1.xhtml?q=1#start"),
            ("a/b/c", "a/b/c"),
            ("", ""),
            ("/", "/"),
            ("", "?q=1#start"),
        ];

        for (path, href) in expected {
            assert_eq!(Href(path), Href(href).path())
        }
    }

    #[test]
    fn test_href_fragment() {
        let expected = [
            (Some("pgepubid00588"), "s04.xhtml#pgepubid00588"),
            (Some("hello%20world"), "/EPUB/c1.xhtml?q=1#hello%20world"),
            (None, "a/b/c"),
            (None, ""),
            (None, "/"),
            (Some(""), "   #"),
            (Some("start"), "   #start"),
        ];

        for (fragment, href) in expected {
            assert_eq!(fragment, Href(href).fragment())
        }
    }

    #[test]
    fn test_href_query() {
        let expected = [
            (None, "s04.xhtml#pgepubid00588"),
            (Some("q=1"), "/EPUB/c1.xhtml?q=1#hello%20world"),
            (None, "a/b/c"),
            (Some(""), "?#"),
            (Some(""), "?"),
            (Some("hello=world"), " ?hello=world"),
            (None, "   #start"),
        ];

        for (query, href) in expected {
            assert_eq!(query, Href(href).query())
        }
    }
}
