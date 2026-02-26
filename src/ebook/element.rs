//! General XML element-related types.

#[cfg(feature = "write")]
mod write;

use crate::util::collection::{Keyed, KeyedVec};
use crate::util::str::StringExt;
use crate::util::uri;
use std::borrow::Cow;
use std::fmt::{Debug, Display};
use std::hash::Hash;
use std::ops::{Deref, DerefMut};
use std::str::SplitWhitespace;

#[cfg(feature = "write")]
pub use write::AttributesIterMut;

/// The percent-encoded `href` of an element, pointing to a location.
///
/// To retrieve the percent-decoded form, see [`Href::decode`].
#[derive(Copy, Clone, Debug, Hash, PartialEq, Eq)]
pub struct Href<'a>(&'a str);

// Methods most relevant for EPUBs
impl<'a> Href<'a> {
    pub(crate) fn new(href: &'a str) -> Self {
        Self(href)
    }

    /// Returns the percent-decoded form.
    ///
    /// # Examples
    /// - Decoding an href:
    /// ```
    /// # use rbook::Epub;
    /// # fn main() -> rbook::ebook::errors::EbookResult<()> {
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

    /// The parent directory, if present.
    ///
    /// # Examples
    /// - Retrieving the parent of an href:
    /// ```
    /// # use rbook::Epub;
    /// # fn main() -> rbook::ebook::errors::EbookResult<()> {
    /// let epub = Epub::open("tests/ebooks/example_epub")?;
    ///
    /// let mut href = epub.package().location();
    /// assert_eq!("/EPUB/example.opf", href.as_str());
    ///
    /// href = href.parent().unwrap();
    /// assert_eq!("/EPUB", href.as_str());
    ///
    /// href = href.parent().unwrap();
    /// assert_eq!("/", href.as_str());
    ///
    /// assert_eq!(None, href.parent());
    /// # Ok(())
    /// # }
    /// ```
    pub fn parent(&self) -> Option<Self> {
        let parent = uri::parent(self.0);
        (!parent.is_empty() && self.0 != parent).then_some(Self(parent))
    }

    /// The file extension, if present (e.g., `.css`, `.xhtml`).
    ///
    /// # See Also
    /// [`ManifestEntry::kind`](super::manifest::ManifestEntry::kind) to
    /// inspect the kind of resource in greater detail.
    ///
    /// # Examples
    /// - Retrieving the extension from an href:
    /// ```
    /// # use rbook::Epub;
    /// # fn main() -> rbook::ebook::errors::EbookResult<()> {
    /// let epub = Epub::open("tests/ebooks/example_epub")?;
    ///
    /// let file = epub.package().location();
    /// assert_eq!("/EPUB/example.opf", file.as_str());
    /// assert_eq!(Some("opf"), file.extension());
    ///
    /// let dir = epub.package().directory();
    /// assert_eq!("/EPUB", dir.as_str());
    /// assert_eq!(None, dir.extension());
    /// # Ok(())
    /// # }
    /// ```
    pub fn extension(&self) -> Option<&'a str> {
        uri::file_extension(self.0)
    }

    /// The href with **only** the query (`?`) and fragment (`#`) omitted.
    ///
    /// An href such as `s04.xhtml#pgepubid00588` will become `s04.xhtml`.
    ///
    /// # Percent Encoding
    /// Paths **may** be percent-encoded.
    /// [`Self::decode`] can be called directly after invoking this method
    /// to retrieve the percent-decoded form.
    ///
    /// # See Also
    /// - [`Self::fragment`]
    /// - [`Self::query`]
    ///
    /// # Examples
    /// - Omitting the query and fragment:
    /// ```
    /// # use rbook::Epub;
    /// # fn main() -> rbook::ebook::errors::EbookResult<()> {
    /// let epub = Epub::open("tests/ebooks/example_epub")?;
    /// let contents = epub.toc().contents().unwrap();
    /// let toc_entry = contents.get(1).unwrap();
    /// let href = toc_entry.href().unwrap();
    ///
    /// assert_eq!("/EPUB/c1.xhtml?q=1#start", href.as_str());
    /// assert_eq!("/EPUB/c1.xhtml", href.path().as_str());
    /// # Ok(())
    /// # }
    /// ```
    pub fn path(&self) -> Self {
        Self(uri::path(self.0))
    }

    /// The filename of an href.
    ///
    /// # Percent Encoding
    /// Filenames **may** be percent-encoded.
    /// [`Self::decode`] can be called directly after invoking this method
    /// to retrieve the percent-decoded form.
    ///
    /// # Examples
    /// - Retrieving the filename:
    /// ```
    /// # use rbook::Epub;
    /// # fn main() -> rbook::ebook::errors::EbookResult<()> {
    /// let epub = Epub::open("tests/ebooks/example_epub")?;
    /// let contents = epub.toc().contents().unwrap();
    /// let toc_entry = contents.get(1).unwrap();
    /// let href = toc_entry.href().unwrap();
    ///
    /// assert_eq!("/EPUB/c1.xhtml?q=1#start", href.as_str());
    /// assert_eq!("c1.xhtml", href.name().as_str());
    /// # Ok(())
    /// # }
    /// ```
    pub fn name(&self) -> Self {
        Self(uri::filename(self.0))
    }

    /// The content of a fragment (`#`) within an href.
    ///
    /// # Examples
    /// - Retrieving the fragment content:
    /// ```
    /// # use rbook::Epub;
    /// # fn main() -> rbook::ebook::errors::EbookResult<()> {
    /// # let epub = Epub::open("tests/ebooks/example_epub")?;
    /// # let contents = epub.toc().contents().unwrap();
    /// # let toc_entry = contents.get(1).unwrap();
    /// let href = toc_entry.href().unwrap();
    ///
    /// assert_eq!("/EPUB/c1.xhtml?q=1#start", href.as_str());
    /// assert_eq!(Some("start"), href.fragment());
    /// # Ok(())
    /// # }
    /// ```
    pub fn fragment(&self) -> Option<&'a str> {
        self.0.find('#').map(|index| &self.0[index + 1..])
    }

    /// The content of a query (`?`) within an href.
    ///
    /// # Examples
    /// - Retrieving the query content:
    /// ```
    /// # use rbook::Epub;
    /// # fn main() -> rbook::ebook::errors::EbookResult<()> {
    /// # let epub = Epub::open("tests/ebooks/example_epub")?;
    /// # let contents = epub.toc().contents().unwrap();
    /// # let toc_entry = contents.get(1).unwrap();
    /// let href = toc_entry.href().unwrap();
    ///
    /// assert_eq!("/EPUB/c1.xhtml?q=1#start", href.as_str());
    /// assert_eq!(Some("q=1"), href.query());
    /// # Ok(())
    /// # }
    /// ```
    pub fn query(&self) -> Option<&'a str> {
        self.0
            .find('?')
            .and_then(|query| self.0[query + 1..].split('#').next())
    }

    /// The underlying `href` string.
    pub fn as_str(&self) -> &'a str {
        self.0
    }
}

impl PartialEq<&str> for Href<'_> {
    fn eq(&self, other: &&str) -> bool {
        self.0 == *other
    }
}

impl PartialEq<Href<'_>> for &str {
    fn eq(&self, other: &Href<'_>) -> bool {
        *self == other.0
    }
}

impl<'a> AsRef<str> for Href<'a> {
    fn as_ref(&self) -> &'a str {
        self.0
    }
}

impl Display for Href<'_> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(self.0)
    }
}

/// A collection of properties associated with an element,
/// where each property is separated by a single ASCII whitespace.
///
/// # Examples
/// - Retrieving the properties from a navigation resource:
/// ```
/// # use rbook::Epub;
/// # fn main() -> rbook::ebook::errors::EbookResult<()> {
/// let epub = Epub::open("tests/ebooks/example_epub")?;
/// let nav_xhtml = epub.manifest().by_property("nav").next().unwrap();
/// let properties = nav_xhtml.properties();
///
/// assert_eq!("scripted nav", properties.as_str());
/// assert!(properties.has_property("nav"));
/// assert!(properties.has_property("scripted"));
/// assert!(!properties.has_property("ncx"));
/// # Ok(())
/// # }
/// ```
#[derive(Clone, Debug, Hash, PartialEq, Eq)]
pub struct Properties(String);

impl Properties {
    pub(crate) const EMPTY_REFERENCE: &'static Properties = &Properties(String::new());

    /// The number of property entries contained within.
    ///
    /// # Note
    /// This method calculates the length in `O(N)` time.
    ///
    /// # Examples
    /// - Retrieving the number of properties:
    /// ```
    /// # use rbook::Epub;
    /// # fn main() -> rbook::ebook::errors::EbookResult<()> {
    /// let epub = Epub::open("tests/ebooks/example_epub")?;
    /// let nav_xhtml = epub.manifest().by_property("nav").next().unwrap();
    /// let properties = nav_xhtml.properties();
    ///
    /// assert_eq!("scripted nav", properties.as_str());
    /// assert_eq!(2, properties.len());
    /// # Ok(())
    /// # }
    /// ```
    pub fn len(&self) -> usize {
        self.iter().count()
    }

    /// Returns `true` if there are no properties.
    ///
    /// # Examples
    /// - Checking if there are properties present:
    /// ```
    /// # use rbook::Epub;
    /// # fn main() -> rbook::ebook::errors::EbookResult<()> {
    /// let epub = Epub::open("tests/ebooks/example_epub")?;
    ///
    /// let nav_xhtml = epub.manifest().by_property("nav").next().unwrap();
    /// let nav_xhtml_properties = nav_xhtml.properties();
    ///
    /// assert_eq!("scripted nav", nav_xhtml_properties.as_str());
    /// assert!(!nav_xhtml_properties.is_empty());
    ///
    /// let chapter_1 = epub.manifest().by_id("c1").unwrap();
    /// let chapter_1_properties = chapter_1.properties();
    ///
    /// assert_eq!("", chapter_1_properties.as_str());
    /// assert!(chapter_1_properties.is_empty());
    /// # Ok(())
    /// # }
    /// ```
    pub fn is_empty(&self) -> bool {
        self.as_str().trim().is_empty()
    }

    /// Returns the associated property if the given `index` is less than
    /// [`Self::len`], otherwise [`None`].
    pub fn get(&self, index: usize) -> Option<&str> {
        self.iter().nth(index)
    }

    /// Returns an iterator over **all** properties.
    ///
    /// # Examples
    /// - Iterating over each property:
    /// ```
    /// # use rbook::Epub;
    /// # fn main() -> rbook::ebook::errors::EbookResult<()> {
    /// let epub = Epub::open("tests/ebooks/example_epub")?;
    /// let nav_xhtml = epub.manifest().by_property("nav").next().unwrap();
    /// let properties = nav_xhtml.properties();
    /// let mut iterator = properties.iter();
    ///
    /// assert_eq!("scripted nav", properties.as_str());
    /// assert_eq!(Some("scripted"), iterator.next());
    /// assert_eq!(Some("nav"), iterator.next());
    /// assert_eq!(None, iterator.next());
    /// # Ok(())
    /// # }
    /// ```
    pub fn iter(&self) -> PropertiesIter<'_> {
        PropertiesIter(self.0.split_whitespace())
    }

    /// Returns `true` if the given property is present.
    ///
    /// # Examples
    /// - Assessing if the provided properties are present:
    /// ```
    /// # use rbook::Epub;
    /// # fn main() -> rbook::ebook::errors::EbookResult<()> {
    /// let epub = Epub::open("tests/ebooks/example_epub")?;
    /// let nav_xhtml = epub.manifest().by_property("nav").next().unwrap();
    /// let properties = nav_xhtml.properties();
    ///
    /// assert_eq!("scripted nav", properties.as_str());
    /// assert_eq!(true, properties.has_property("scripted"));
    /// assert_eq!(true, properties.has_property("nav"));
    /// assert_eq!(false, properties.has_property("other"));
    /// assert_eq!(false, properties.has_property(" "));
    /// # Ok(())
    /// # }
    /// ```
    pub fn has_property(&self, property: &str) -> bool {
        self.iter().any(|value| value == property)
    }

    /// The underlying raw properties.
    ///
    /// - Retrieving the properties as a string:
    /// ```
    /// # use rbook::Epub;
    /// # fn main() -> rbook::ebook::errors::EbookResult<()> {
    /// let epub = Epub::open("tests/ebooks/example_epub")?;
    /// let nav_xhtml = epub.manifest().by_property("nav").next().unwrap();
    /// let properties = nav_xhtml.properties();
    ///
    /// assert_eq!("scripted nav", properties.as_str());
    /// # Ok(())
    /// # }
    /// ```
    pub fn as_str(&self) -> &str {
        self.0.trim()
    }
}

impl Display for Properties {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(self.as_str())
    }
}

impl AsRef<str> for Properties {
    fn as_ref(&self) -> &str {
        &self.0
    }
}

impl<'a> IntoIterator for &'a Properties {
    type Item = &'a str;
    type IntoIter = PropertiesIter<'a>;

    fn into_iter(self) -> Self::IntoIter {
        self.iter()
    }
}

/// An iterator over each property within [`Properties`].
///
/// # See Also
/// - [`Properties::iter`] to create an instance of this struct.
///
/// # Examples
/// - Iterating over each property:
/// ```
/// # use rbook::Epub;
/// # fn main() -> rbook::ebook::errors::EbookResult<()> {
/// let epub = Epub::open("tests/ebooks/example_epub")?;
/// let nav_xhtml = epub.manifest().by_property("nav").next().unwrap();
///
/// for property in nav_xhtml.properties() {
///     // process property //
/// }
/// # Ok(())
/// # }
/// ```
pub struct PropertiesIter<'a>(SplitWhitespace<'a>);

impl<'a> Iterator for PropertiesIter<'a> {
    type Item = &'a str;

    fn next(&mut self) -> Option<Self::Item> {
        self.0.next()
    }
}

/// A collection of [`Attribute`] entries associated with an element.
#[derive(Clone, Debug, PartialEq)]
pub struct Attributes(KeyedVec<Attribute>);

impl Attributes {
    /// The number of [`Attribute`] entries contained within.
    pub fn len(&self) -> usize {
        self.0.len()
    }

    /// Returns `true` if there are no attributes.
    pub fn is_empty(&self) -> bool {
        self.0.is_empty()
    }

    /// Returns the associated [`Attribute`] if the given `index` is less than
    /// [`Self::len`], otherwise [`None`].
    pub fn get(&self, index: usize) -> Option<&Attribute> {
        self.0.get(index)
    }

    /// Returns an iterator over **all** [`Attribute`] entries.
    pub fn iter(&self) -> AttributesIter<'_> {
        AttributesIter(self.0.0.iter())
    }

    /// Returns the [`Attribute`] with the given `name` if present, otherwise [`None`].
    ///
    /// # See Also
    /// - [`Self::get_value`] to retrieve the attribute [value](Attribute::value) directly.
    pub fn by_name(&self, name: &str) -> Option<&Attribute> {
        self.0.by_key(name)
    }

    /// Returns the [value](Attribute::value) of the [`Attribute`]
    /// with the given `name` if present, otherwise [`None`].
    pub fn get_value(&self, name: &str) -> Option<&str> {
        self.0.by_key(name).map(|attr| attr.value())
    }

    /// Returns `true` if an [`Attribute`] with the given `name` is present.
    pub fn has_name(&self, name: &str) -> bool {
        self.0.has_key(name)
    }
}

impl<'a> IntoIterator for &'a Attributes {
    type Item = &'a Attribute;
    type IntoIter = AttributesIter<'a>;

    fn into_iter(self) -> Self::IntoIter {
        self.iter()
    }
}

/// An iterator over all [`Attribute`] entries within [`Attributes`].
///
/// # See Also
/// - [`Attributes::iter`] to create an instance of this struct.
///
/// # Examples
/// - Iterating over all attributes:
/// ```
/// # use rbook::{Ebook, Epub};
/// # fn main() -> rbook::ebook::errors::EbookResult<()> {
/// let epub = Epub::open("tests/ebooks/example_epub")?;
/// let nav_xhtml = epub.manifest().by_property("nav").next().unwrap();
///
/// for attribute in nav_xhtml.attributes() {
///     // process attribute //
/// }
/// # Ok(())
/// # }
/// ```
pub struct AttributesIter<'a>(std::slice::Iter<'a, Attribute>);

impl<'a> Iterator for AttributesIter<'a> {
    // AttributeData is not returned directly here
    // to allow greater flexibility in the future.
    type Item = &'a Attribute;

    fn next(&mut self) -> Option<Self::Item> {
        self.0.next()
    }
}

/// An attribute, containing details about an element.
///
/// An attribute encompasses a [`Name`] as `key` and a string as `value`.
///
/// # Note
/// - The attribute [name](Self::name) and [value](Self::value)
///   are trimmed of leading and trailing whitespace on creation.
/// - When the `write` feature flag is enabled, only modification of the value is allowed.
///   **The name cannot be modified once an attribute is created.**
///   This prevents duplicate keys within [`Attributes`].
#[derive(Clone, Debug, PartialEq)]
pub struct Attribute {
    name: String,
    value: Properties,
}

impl Attribute {
    /// Internal constructor.
    ///
    /// The public constructor [`Self::new`] is available when the `write`
    /// feature is enabled.
    pub(crate) fn create(name: impl Into<String>, value: impl Into<String>) -> Self {
        let mut name = name.into();
        let mut value = value.into();

        name.trim_in_place();
        value.trim_in_place();

        Self {
            value: Properties(value),
            name,
        }
    }

    /// The attribute name/key.
    pub fn name(&self) -> Name<'_> {
        Name(&self.name)
    }

    /// The attribute value.
    ///
    /// # Note
    /// Attribute values have their leading and trailing whitespace trimmed.
    pub fn value(&self) -> &str {
        self.value.as_str()
    }

    /// The attribute [`value`](Self::value) in the form of [`Properties`].
    pub fn as_properties(&self) -> &Properties {
        &self.value
    }
}

impl Keyed for Attribute {
    type Key = str;

    fn key(&self) -> &Self::Key {
        self.name.as_str()
    }
}

/// The name of an element or [`Attribute`].
#[derive(Copy, Clone, Debug, Hash, PartialEq, Eq)]
pub struct Name<'a>(&'a str);

impl<'a> Name<'a> {
    pub(crate) fn new(name: &'a str) -> Self {
        Self(name)
    }

    fn index(&self) -> Option<usize> {
        self.0.find(':')
    }

    /// The prefix of a name.
    ///
    /// # Examples
    /// - The name `dcterms:modified` has a prefix of `dcterms`:
    /// ```
    /// # use rbook::Epub;
    /// # fn main() -> rbook::ebook::errors::EbookResult<()> {
    /// # let epub = Epub::open("tests/ebooks/example_epub")?;
    /// # let modified = epub.metadata().by_property("dcterms:modified").next().unwrap();
    /// # let name = modified.property();
    /// assert_eq!("dcterms:modified", name);
    /// assert_eq!(Some("dcterms"), name.prefix());
    /// # Ok(())
    /// # }
    /// ```
    pub fn prefix(&self) -> Option<&'a str> {
        self.index().map(|i| &self.0[..i])
    }

    /// The name with its [prefix](Self::prefix) stripped.
    ///
    /// # Examples
    /// - The name `dcterms:modified` with its prefix stripped is `modified`.
    /// ```
    /// # use rbook::Epub;
    /// # fn main() -> rbook::ebook::errors::EbookResult<()> {
    /// # let epub = Epub::open("tests/ebooks/example_epub")?;
    /// # let modified = epub.metadata().by_property("dcterms:modified").next().unwrap();
    /// # let name = modified.property();
    /// assert_eq!("dcterms:modified", name);
    /// assert_eq!("modified", name.local());
    /// # Ok(())
    /// # }
    /// ```
    pub fn local(&self) -> &'a str {
        self.index().map_or(self.0, |i| &self.0[i + 1..])
    }

    /// The raw name with its prefix intact (e.g., `dcterms:modified`).
    pub fn as_str(&self) -> &'a str {
        self.0
    }
}

impl PartialEq<&str> for Name<'_> {
    fn eq(&self, other: &&str) -> bool {
        self.0 == *other
    }
}

impl PartialEq<Name<'_>> for &str {
    fn eq(&self, other: &Name<'_>) -> bool {
        *self == other.0
    }
}

impl<'a> AsRef<str> for Name<'a> {
    fn as_ref(&self) -> &'a str {
        self.0
    }
}

impl Display for Name<'_> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
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
    const LEFT_TO_RIGHT: &'static str = "ltr";
    const RIGHT_TO_LEFT: &'static str = "rtl";
    const AUTO: &'static str = "auto";

    /// Returns `true` if the text direction is [`TextDirection::LeftToRight`].
    pub fn is_ltr(self) -> bool {
        matches!(self, Self::LeftToRight)
    }

    /// Returns `true` if the text direction is [`TextDirection::RightToLeft`].
    pub fn is_rtl(self) -> bool {
        matches!(self, Self::RightToLeft)
    }

    /// Returns `true` if the text direction is [`TextDirection::Auto`].
    pub fn is_auto(self) -> bool {
        matches!(self, Self::Auto)
    }

    /// The string representation of a [`TextDirection`] hint.
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

impl Display for TextDirection {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(self.as_str())
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

////////////////////////////////////////////////////////////////////////////////
// PRIVATE API - Implements Default
// Note: Public API for `Properties` and `Attributes`
//       does not offer explicit creation.
////////////////////////////////////////////////////////////////////////////////

#[derive(Clone, Debug, PartialEq)]
pub(crate) struct PropertiesData(Properties);

impl Default for PropertiesData {
    fn default() -> Self {
        Self(Properties(String::new()))
    }
}

impl Deref for PropertiesData {
    type Target = Properties;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl DerefMut for PropertiesData {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.0
    }
}

impl From<Option<String>> for PropertiesData {
    fn from(value: Option<String>) -> Self {
        Self(Properties(value.unwrap_or_default()))
    }
}

#[derive(Clone, Debug, PartialEq)]
pub(crate) struct AttributesData(Attributes);

impl Default for AttributesData {
    fn default() -> Self {
        Self(Attributes(KeyedVec(Vec::new())))
    }
}

impl Deref for AttributesData {
    type Target = Attributes;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl DerefMut for AttributesData {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.0
    }
}

impl From<Vec<Attribute>> for AttributesData {
    fn from(value: Vec<Attribute>) -> Self {
        Self(Attributes(KeyedVec(value)))
    }
}

#[cfg(test)]
mod tests {
    use crate::ebook::element::Href;

    #[test]
    fn test_href_parent() {
        let expected = [
            (None, "abc.html"),
            (Some("/EPUB"), "/EPUB/c1.xhtml"),
            (Some("a"), "a/b"),
            (None, ""),
            (None, " "),
            (None, "/"),
            (None, "xyz"),
        ];

        for (parent, href) in expected {
            assert_eq!(parent, Href(href).parent().map(|p| p.as_str()))
        }
    }

    #[test]
    fn test_href_extension() {
        let expected = [
            (Some("xhtml"), "s04.xhtml#pgepubid00588"),
            (Some("html"), "/EPUB/c1.html?q=1#start"),
            (Some("css"), "abc.css"),
            (Some("xyz"), ".xyz"),
            (None, "a/b"),
            (None, "abc"),
            (None, ""),
            (None, " "),
            (None, "/"),
        ];

        for (extension, href) in expected {
            assert_eq!(extension, Href(href).extension())
        }
    }

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
    fn test_href_name() {
        let expected = [
            ("s04.xhtml", "s04.xhtml#pgepubid00588"),
            ("c1.xhtml", "/EPUB/c1.xhtml?q=1#start"),
            ("c", "a/b/c"),
            ("", ""),
            ("", "/"),
            ("", "?q=1#start"),
        ];

        for (path, href) in expected {
            assert_eq!(Href(path), Href(href).name())
        }
    }

    #[test]
    fn test_href_fragment() {
        let expected = [
            (Some("page-epub-id00588"), "s04.xhtml#page-epub-id00588"),
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
