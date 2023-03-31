use std::borrow::Cow;
use std::fmt::{Display, Formatter};
use std::ops::Deref;

/// Used to retrieve specific information about retrieved
/// [Content] from a [Reader](super::Reader).
#[derive(Debug)]
pub enum ContentType {
    /// The path where the content originates from, i.e.,
    /// `ebook/content/cover.xhtml`.
    Path,
    /// The manifest element id associated with the content.
    Id,
    /// The media type of the content, i.e.,
    /// `application/xhtml+xml`.
    Type,
}

impl ContentType {
    pub(crate) fn as_str(&self) -> &str {
        match self {
            ContentType::Path => "path",
            ContentType::Id => "id",
            ContentType::Type => "type",
        }
    }
}

/// Retrieved content from a [Reader](super::Reader)
/// to access string data, bytes, media type, etc.
///
/// # Example
/// Displaying content:
/// ```
/// # use rbook::Ebook;
/// # use rbook::read::ContentType;
/// # let epub = rbook::Epub::new("tests/ebooks/moby-dick.epub").unwrap();
/// let mut reader = epub.reader();
///
/// let content = reader.next_page().unwrap();
///
/// // content implements display
/// println!("{content}");
/// ```
/// Accessing content data:
/// ```
/// # use rbook::Ebook;
/// # let epub = rbook::Epub::new("tests/ebooks/moby-dick.epub").unwrap();
/// # let mut reader = epub.reader();
/// # let content = reader.next_page().unwrap();
/// use rbook::read::ContentType;
///
/// // Data in byte form
/// assert!(content.starts_with(b"<?xml"));
/// assert!(content.starts_with(b"\x3C\x3F\x78\x6D\x6C"));
///
/// // Data in string form
/// assert!(content.as_lossy_str().starts_with("<?xml"));
///
/// // Retrieve the media type
/// assert_eq!("application/xhtml+xml", content.get_content(ContentType::Type).unwrap());
///
/// // Retrieve the path
/// assert_eq!("OPS/titlepage.xhtml", content.get_content(ContentType::Path).unwrap());
/// ```
#[derive(Debug, PartialEq, Eq)]
pub struct Content<'a> {
    bytes: Vec<u8>,
    // Used instead of hashmap as the size is always small. ~3 items
    fields: Vec<(&'static str, Cow<'a, str>)>,
}

impl<'a> Content<'a> {
    pub(crate) fn new(bytes: Vec<u8>, fields: Vec<(&'static str, Cow<'a, str>)>) -> Self {
        Self { bytes, fields }
    }

    /// Retrieve the content data in the form of a string.
    pub fn as_lossy_str(&self) -> Cow<str> {
        String::from_utf8_lossy(&self.bytes)
    }

    /// Retrieve specific information about the content.
    ///
    /// See [ContentType] for available options.
    pub fn get_content(&self, content_type: ContentType) -> Option<&str> {
        self.fields
            .iter()
            .find(|(key, _)| *key == content_type.as_str())
            .map(|(_, value)| value.as_ref())
    }
}

impl<'a> Deref for Content<'a> {
    type Target = [u8];

    fn deref(&self) -> &Self::Target {
        &self.bytes
    }
}

impl Display for Content<'_> {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.as_lossy_str())
    }
}
