//! Format-agnostic [`Resource`] types for ebooks.

use crate::ebook::element::Href;
use crate::util::{StrExt, StringExt};
use std::borrow::Cow;
use std::fmt::{Debug, Display, Formatter};
use std::path::Path;

/// A resource within an [`Ebook`](crate::Ebook), pointing to where associated content is stored.
///
/// Each resource consists of:
/// 1. A [`ResourceKey`], which locates the data, such as a relative path (`"OEBPS/ch1.xhtml"`).
/// 2. A [`ResourceKind`], indicating the data’s kind, such as determining if it's a `PNG` file.
///
/// # Examples
/// - Creating a [`Resource`] using [`Into`] ([`From`] is usable as well):
/// ```
/// # use rbook::ebook::resource::{Resource, ResourceKey, ResourceKind};
/// # use std::borrow::Cow;
/// // Providing only the resource key value:
/// let ch1: Resource = "OEBPS/main/c1.xhtml".into();
/// // Providing the resource kind and key value:
/// let ncx: Resource = ("application/x-dtbncx+xml", "OEBPS/main/toc.ncx").into();
/// // Providing the resource kind and key position:
/// let xhtml: Resource = ("application/xhtml+xml", 6).into();
///
/// // Checking the key:
/// assert!(matches!(ch1.key(), ResourceKey::Value(Cow::Borrowed("OEBPS/main/c1.xhtml"))));
/// assert!(matches!(ncx.key(), ResourceKey::Value(Cow::Borrowed("OEBPS/main/toc.ncx"))));
/// assert!(matches!(xhtml.key(), ResourceKey::Position(6)));
///
/// // Checking The kind:
/// // The kind for `ch1` was never specified, so its kind is `UNSPECIFIED`.
/// assert!(ch1.kind().as_str().is_empty());
/// assert_eq!(&ResourceKind::UNSPECIFIED, ch1.kind());
/// assert_eq!("application/x-dtbncx+xml", ncx.kind().as_str());
/// assert_eq!("application/xhtml+xml", xhtml.kind().as_str());
/// ```
/// - Providing a [`Resource`] as an argument:
/// ```
/// # use rbook::ebook::errors::EbookResult;
/// # use rbook::ebook::resource::Resource;
/// # use rbook::{Ebook, Epub};
/// # fn main() -> EbookResult<()> {
/// let cover_path = "/EPUB/cover.xhtml";
/// let resource = Resource::from(cover_path);
///
/// let epub = Epub::open("tests/ebooks/example_epub")?;
///
/// // `read_resource_bytes` accepts `Into<Resource>` as an argument,
/// // so anything that implements Into for `Resource` may be passed.
/// let data1 = epub.read_resource_bytes(cover_path)?;
/// let data2 = epub.read_resource_bytes(resource)?;
///
/// assert_eq!(data1, data2);
/// # Ok(())
/// # }
/// ```
#[derive(Clone, Debug, Hash, PartialEq, Eq)]
pub struct Resource<'a> {
    key: ResourceKey<'a>,
    kind: ResourceKind<'a>,
}

impl<'a> Resource<'a> {
    pub(crate) fn new(kind: impl Into<ResourceKind<'a>>, key: impl Into<ResourceKey<'a>>) -> Self {
        Self {
            kind: kind.into(),
            key: key.into(),
        }
    }

    pub(crate) fn swap_value(mut self, value: String) -> Self {
        self.key = ResourceKey::Value(Cow::Owned(value));
        self
    }

    pub(crate) fn as_static(&self) -> Resource<'static> {
        Resource {
            key: self.key.as_static(),
            kind: self.kind.as_static(),
        }
    }

    /// The [`ResourceKey`], indicating where the associated content is held.
    pub fn key(&self) -> &ResourceKey<'a> {
        &self.key
    }

    /// The [`ResourceKind`], indicating if a resource is an `XHTML` file,
    /// `JPEG` image, `CSS` stylesheet, etc.
    pub fn kind(&self) -> &ResourceKind<'a> {
        &self.kind
    }
}

impl Display for Resource<'_> {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        write!(f, "{:?}@{:?}", self.kind, self.key)
    }
}

impl<'a, Kind: Into<ResourceKind<'a>>, Key: Into<ResourceKey<'a>>> From<(Kind, Key)>
    for Resource<'a>
{
    fn from((kind, key): (Kind, Key)) -> Self {
        Self::new(kind, key)
    }
}

impl<'a, K: Into<ResourceKey<'a>>> From<K> for Resource<'a> {
    fn from(value: K) -> Self {
        Self::new(ResourceKind::UNSPECIFIED, value)
    }
}

impl<'a> From<&'a Self> for Resource<'a> {
    fn from(value: &'a Self) -> Self {
        Self::new(&value.kind, &value.key)
    }
}

/// A key identifying where the content is held for a [`Resource`], either by
/// [`string`](ResourceKey::Value) or numeric [`position`](ResourceKey::Position).
///
/// # Examples
/// - Creating a key using [`From`] ([`Into`] is usable as well):
/// ```
/// # use rbook::ebook::resource::{ResourceKey, ResourceKind};
/// # use std::borrow::Cow;
/// # use std::path::Path;
/// let position_key = ResourceKey::from(5);
/// let str_key = ResourceKey::from("OEBPS/nav/toc.xhtml");
/// // A path may be passed as well, although it must contain valid UTF-8.
/// // Otherwise, the requested file won't be found and an error will
/// // be propagated from methods that process a `ResourceKey`:
/// let str_key2 = ResourceKey::from(Path::new("EPUB/toc.ncx"));
///
/// assert!(matches!(position_key, ResourceKey::Position(5)));
/// assert!(matches!(str_key, ResourceKey::Value(Cow::Borrowed("OEBPS/nav/toc.xhtml"))));
/// assert!(matches!(str_key2, ResourceKey::Value(Cow::Borrowed("EPUB/toc.ncx"))));
/// ```
#[derive(Clone, Debug, Hash, PartialEq, Eq)]
pub enum ResourceKey<'a> {
    /// A string-based [`key`](ResourceKey) (e.g., `file path`, `URI`)
    /// that references where a [`resource`](Resource) is stored.
    ///
    /// When using the [`From<&Path>`](ResourceKey::from) impl, the input must be
    /// valid UTF-8; any non-UTF-8 sequences will be replaced with `�`.
    Value(Cow<'a, str>),

    /// A numeric index-based [`key`](ResourceKey) for locations that
    /// cannot be represented with a string value.
    ///
    /// This is primarily useful for [`resources`](Resource) that do not have a string value
    /// available, but rather a "pointer" to access an element within an indexable structure,
    /// such as the `MOBI` format.
    Position(usize),
}

impl ResourceKey<'_> {
    fn as_static(&self) -> ResourceKey<'static> {
        match self {
            Self::Value(value) => ResourceKey::Value(Cow::Owned(value.to_string())),
            Self::Position(position) => ResourceKey::Position(*position),
        }
    }

    /// Returns the contained value if `self` is [`ResourceKey::Value`].
    ///
    /// # Examples
    /// - Retrieving the value:
    /// ```
    /// # use rbook::ebook::resource::ResourceKey;
    /// let key = ResourceKey::from("value");
    /// assert_eq!(Some("value"), key.value());
    /// assert_eq!(None, key.position());
    /// ```
    pub fn value(&self) -> Option<&str> {
        match self {
            Self::Value(value) => Some(value.as_ref()),
            Self::Position(_) => None,
        }
    }

    /// Returns the contained position if `self` is [`ResourceKey::Position`].
    ///
    /// # Examples
    /// - Retrieving the position:
    /// ```
    /// # use rbook::ebook::resource::ResourceKey;
    /// let key = ResourceKey::from(5);
    /// assert_eq!(Some(5), key.position());
    /// assert_eq!(None, key.value());
    /// ```
    pub fn position(&self) -> Option<usize> {
        match self {
            Self::Position(position) => Some(*position),
            Self::Value(_) => None,
        }
    }
}

impl<'a> From<Href<'a>> for ResourceKey<'a> {
    fn from(value: Href<'a>) -> Self {
        value.path().as_str().into()
    }
}

impl<'a> From<&'a Path> for ResourceKey<'a> {
    fn from(value: &'a Path) -> Self {
        // It is EXPECTED that the path given is UTF-8 compliant
        Self::Value(value.to_string_lossy())
    }
}

impl<'a> From<&'a str> for ResourceKey<'a> {
    fn from(value: &'a str) -> Self {
        Self::Value(value.into())
    }
}

impl<'a> From<String> for ResourceKey<'a> {
    fn from(value: String) -> Self {
        Self::Value(value.into())
    }
}

impl<'a> From<Cow<'a, str>> for ResourceKey<'a> {
    fn from(value: Cow<'a, str>) -> Self {
        Self::Value(value)
    }
}

impl From<usize> for ResourceKey<'_> {
    fn from(value: usize) -> Self {
        Self::Position(value)
    }
}

impl<'a> From<&'a Self> for ResourceKey<'a> {
    fn from(value: &'a Self) -> Self {
        match value {
            Self::Value(value) => Self::Value(Cow::Borrowed(value)),
            Self::Position(position) => Self::Position(*position),
        }
    }
}

/// The kind of [`resource`](Resource) contained within an ebook based on
/// [`MIME`](https://developer.mozilla.org/en-US/docs/Web/HTTP/Guides/MIME_types),
/// useful for inferring if a resource is an `XHTML` file, `PNG` image, etc.
///
/// To view all standard MIME types, see:
/// <https://www.iana.org/assignments/media-types/media-types.xhtml>
///
/// MIME structure: `maintype/subtype[+suffix][;params]`
///
/// # Equality
///  Resource kinds are compared by their components, so the following are treated as equivalent:
/// ```
/// # use rbook::ebook::resource::ResourceKind;
/// let a = ResourceKind::from("application/example;param=XYZ;param2=ABC");
/// let b = ResourceKind::from("  application/EXAMPLE; PARAM2 = ABC;param = XYZ;; ;   ");
/// assert_eq!(a, b);
/// ```
/// Equality is based on the following:
/// - Type components ([`maintype`](ResourceKind::maintype), [`subtype`](ResourceKind::subtype),
///   and [`suffix`](ResourceKind::suffix)) are case-insensitive.
/// - Parameter order does not matter,
///   although parameter keys are case-insensitive and values are case-sensitive.
/// - Duplicate parameter keys are considered malformations and equality (`==`)
///   behavior is undefined.
/// - Within [`parameters`](ResourceKind::params),
///   extra semicolons (`;`) and surrounding whitespace are ignored.
///
/// # Examples
/// - Minimal `PNG` resource kind:
/// ```
/// # use rbook::ebook::resource::ResourceKind;
/// let png_kind = ResourceKind::from("image/png");
/// // Also equivalent to:
/// let png_kind: ResourceKind = "image/png".into();
///
/// assert!(png_kind.is_image());
/// assert_eq!("image", png_kind.maintype());
/// assert_eq!("png", png_kind.subtype());
/// assert_eq!(None, png_kind.suffix());
/// assert_eq!(None, png_kind.params());
/// ```
/// - Thorough `NCX` resource kind:
/// ```
/// # use rbook::ebook::resource::ResourceKind;
/// let ncx_kind = ResourceKind::from("application/x-dtbncx+xml; charset=UTF-8");
///
/// assert!(ncx_kind.is_application());
/// assert_eq!("application", ncx_kind.maintype());
/// assert_eq!("x-dtbncx", ncx_kind.subtype());
/// assert_eq!(Some("xml"), ncx_kind.suffix());
///
/// // Getting the parameters
/// assert_eq!(Some("charset=UTF-8"), ncx_kind.params());
/// assert_eq!(Some("UTF-8"), ncx_kind.get_param("charset"));
/// assert_eq!([("charset", "UTF-8")], *ncx_kind.params_iter().collect::<Vec<_>>());
/// ```
#[derive(Clone, Debug, Hash, Eq)]
pub struct ResourceKind<'a>(Cow<'a, str>);

impl ResourceKind<'_> {
    const _UNSPECIFIED: &'static str = "";
    const _APPLICATION: &'static str = "application";
    const _AUDIO: &'static str = "audio";
    const _FONT: &'static str = "font";
    const _IMAGE: &'static str = "image";
    const _TEXT: &'static str = "text";
    const _VIDEO: &'static str = "video";

    /// An unspecified or unknown resource.
    ///
    /// This constant has nothing (e.g., no maintype, subtype, etc.), primarily for
    /// use as a wildcard to catch all resources for methods that accept a [`ResourceKind`].
    pub const UNSPECIFIED: ResourceKind<'static> = Self::borrowed(Self::_UNSPECIFIED);

    /// Resources that require an application to use, such as `XHTML` files.
    ///
    /// This constant only has a maintype, `application`, primarily for use as a maintype
    /// wildcard matching any `application/*` for methods that accept a [`ResourceKind`].
    pub const APPLICATION: ResourceKind<'static> = Self::borrowed(Self::_APPLICATION);

    /// Audio or music resources, such as `AAC` and `MP3`.
    ///
    /// This constant only has a maintype, `audio`, primarily for use as a maintype
    /// wildcard matching any `audio/*` for methods that accept a [`ResourceKind`].
    pub const AUDIO: ResourceKind<'static> = Self::borrowed(Self::_AUDIO);

    /// Font-related resources, such as `OTF` and `WOFF`.
    ///
    /// This constant only has a maintype, `font`, primarily for use as a maintype
    /// wildcard matching any `font/*` for methods that accept a [`ResourceKind`].
    pub const FONT: ResourceKind<'static> = Self::borrowed(Self::_FONT);

    /// Image-related resources, such as `JPEG` and `PNG`.
    ///
    /// This constant only has a maintype, `image`, primarily for use as a maintype
    /// wildcard matching any `image/*` for methods that accept a [`ResourceKind`].
    pub const IMAGE: ResourceKind<'static> = Self::borrowed(Self::_IMAGE);

    /// Text-only human-readable resources, such as `TXT` and `CSS`.
    ///
    /// This constant only has a maintype, `text`, primarily for use as a maintype
    /// wildcard matching any `text/*` for methods that accept a [`ResourceKind`].
    pub const TEXT: ResourceKind<'static> = Self::borrowed(Self::_TEXT);

    /// Video-related resources, such as `OPUS` and `AV1`.
    ///
    /// This constant only has a maintype, `video`, primarily for use as a maintype
    /// wildcard matching any `video/*` for methods that accept a [`ResourceKind`].
    pub const VIDEO: ResourceKind<'static> = Self::borrowed(Self::_VIDEO);

    const fn borrowed(static_str: &str) -> ResourceKind<'_> {
        ResourceKind(Cow::Borrowed(static_str))
    }

    fn as_static(&self) -> ResourceKind<'static> {
        ResourceKind(Cow::Owned(self.0.to_string()))
    }

    /// The raw underlying string of a resource kind.
    ///
    /// # Example:
    /// ```
    /// # use rbook::ebook::resource::ResourceKind;
    /// // Input is implicitly trimmed
    /// let kind = ResourceKind::from("   image/png  ");
    /// let other_kind = ResourceKind::from("audio/webm");
    ///
    /// assert_eq!("image/png", kind.as_str());
    /// assert_eq!("audio/webm", other_kind.as_str());
    /// ```
    pub fn as_str(&self) -> &str {
        self.0.as_ref()
    }

    /// The maintype of a resource kind.
    /// For an ebook, the maintype is generally one of the following:
    /// - `application`
    /// - `audio`
    /// - `font`
    /// - `image`
    /// - `text`
    /// - `video`
    ///
    /// # Structure
    /// - _**`application`**_`/xhtml+xml;charset=UTF-8`
    /// - The maintype: `application`
    ///
    /// # Examples
    /// - Retrieving the maintype of a resource kind:
    /// ```
    /// # use rbook::ebook::resource::ResourceKind;
    /// let xhtml_kind = ResourceKind::from("application/xhtml+xml");
    /// let audio_kind = ResourceKind::from("audio/mpeg");
    ///
    /// assert_eq!("application", xhtml_kind.maintype());
    /// assert_eq!("audio", audio_kind.maintype());
    /// ```
    pub fn maintype(&self) -> &str {
        self.0.split('/').next().unwrap_or_default()
    }

    /// The subtype, immediately after the [`maintype`](Self::maintype)
    /// separated by a forward slash (`/`).
    ///
    /// # Structure
    /// - `application/`_**`xhtml`**_`+xml;charset=UTF-8`
    /// - The subtype: `xhtml`
    ///
    /// # Examples
    /// - Retrieving the subtype of a resource kind:
    /// ```
    /// # use rbook::ebook::resource::ResourceKind;
    /// let xhtml_kind = ResourceKind::from("application/xhtml+xml");
    /// let audio_kind = ResourceKind::from("audio/mpeg");
    ///
    /// assert_eq!("xhtml", xhtml_kind.subtype());
    /// assert_eq!("mpeg", audio_kind.subtype());
    /// ```
    pub fn subtype(&self) -> &str {
        self.0.split(['/', '+', ';']).nth(1).unwrap_or_default()
    }

    /// The suffix, after the [`subtype`](Self::subtype) separated by a plus symbol (`+`).
    /// Depending on the type, a suffix may not be applicable:
    /// - Present: `application/xhtml+xml`
    /// - Not Present: `application/xml`
    ///
    /// # Structure
    /// - `application/xhtml+`_**`xml`**_`;charset=UTF-8`
    /// - The suffix: `xml`
    ///
    /// # Examples
    /// - Retrieving the suffix of a resource kind (with and without):
    /// ```
    /// # use rbook::ebook::resource::ResourceKind;
    /// // Has a suffix
    /// let xhtml_kind = ResourceKind::from("application/xhtml+xml");
    /// // Has no suffix
    /// let audio_kind = ResourceKind::from("audio/ogg; codecs=opus");
    ///
    /// assert_eq!(Some("xml"), xhtml_kind.suffix());
    /// assert_eq!(None, audio_kind.suffix());
    /// ```
    pub fn suffix(&self) -> Option<&str> {
        // Remove parameters as it can conflict with finding the suffix
        let base_type = self.0.split(';').next()?;
        // With the parameters removed, find the suffix
        base_type.rfind('+').map(|index| &base_type[index + 1..])
    }

    /// The raw parameters string of a resource kind.
    ///
    /// # Structure
    /// - `application/xhtml+xml;`_**`charset=UTF-8`**_
    /// - The parameters: `charset=UTF-8`
    ///
    /// # See Also
    /// - [`Self::params_iter`] to iterate over all parameters.
    ///
    /// # Examples
    /// - Retrieving the suffix of a resource kind (with and without):
    /// ```
    /// # use rbook::ebook::resource::ResourceKind;
    /// // Has parameters
    /// let with_params = ResourceKind::from("audio/ogg; codecs=opus; other_param=value");
    /// // Has no parameters
    /// let no_params = ResourceKind::from("audio/ogg");
    ///
    /// assert_eq!(Some("codecs=opus; other_param=value"), with_params.params());
    /// assert_eq!(None, no_params.params());
    /// ```
    pub fn params(&self) -> Option<&str> {
        self.0.find(';').map(|index| self.0[index + 1..].trim())
    }

    /// Returns an iterator over all the parameters contained within a resource kind.
    /// Each `Item` within the iterator is a tuple containing the `key` and `value`
    /// of a parameter.
    ///
    /// Tuple structure: (`param key`, `param value`)
    ///
    /// # Examples
    /// - Iterating over all the parameters:
    /// ```
    /// # use rbook::ebook::resource::ResourceKind;
    /// let kind = ResourceKind::from("audio/ogg; codecs=opus; other_param=value");
    /// let mut iterator = kind.params_iter();
    ///
    /// assert_eq!(Some(("codecs", "opus")), iterator.next());
    /// assert_eq!(Some(("other_param", "value")), iterator.next());
    /// assert_eq!(None, iterator.next());
    /// ```
    pub fn params_iter(&self) -> impl Iterator<Item = (&str, &str)> {
        self.params()
            .unwrap_or_default()
            .split(';')
            .filter_map(|param| param.split_once('='))
            .map(|(key, value)| (key.trim(), value.trim()))
    }

    /// Returns the parameter value associated with the given key if present, otherwise [None].
    ///
    /// # Examples
    /// - Retrieving the parameter of an audio resource kind:
    /// ```
    /// # use rbook::ebook::resource::ResourceKind;
    /// let kind = ResourceKind::from("audio/ogg; codecs=opus; other_param=value");
    ///
    /// assert_eq!(Some("value"), kind.get_param("other_param"));
    /// assert_eq!(Some("opus"), kind.get_param("codecs"));
    /// assert_eq!(None, kind.get_param("codec"));
    /// ```
    pub fn get_param(&self, param_key: &str) -> Option<&str> {
        self.params_iter()
            .find_map(|(key, value)| (param_key == key).then_some(value))
    }

    /// Returns `true` if the maintype or subtype is **not** present.
    ///
    /// Constants which evaluate to `true` for `is_unspecified` (acting as wildcards):
    /// - [`ResourceKind::APPLICATION`]
    /// - [`ResourceKind::AUDIO`]
    /// - [`ResourceKind::FONT`]
    /// - [`ResourceKind::IMAGE`]
    /// - [`ResourceKind::VIDEO`]
    ///
    /// # Examples
    /// - Assessing if resource kinds are unspecified:
    /// ```
    /// # use rbook::ebook::resource::ResourceKind;
    /// let audio_ogg_kind: ResourceKind = "audio/ogg".into();
    /// let audio_kind: ResourceKind = "audio/".into();
    /// let none_kind: ResourceKind = "".into();
    ///
    /// // Not unspecified
    /// assert!(!audio_ogg_kind.is_unspecified());
    ///
    /// // Is unspecified
    /// assert!(audio_kind.is_unspecified());
    /// assert!(none_kind.is_unspecified());
    /// assert!(ResourceKind::IMAGE.is_unspecified());
    /// ```
    pub fn is_unspecified(&self) -> bool {
        self.subtype().is_empty() || self.maintype().is_empty()
    }

    /// Returns `true` if the kind of resource is an application-level resource.
    ///
    /// Specifically, `true` is returned if the [`maintype`](Self::maintype)
    /// equals case-insensitive `application`.
    ///
    /// # Examples
    /// - Assessing if `x-dtbncx+xml` is an application type:
    /// ```
    /// # use rbook::ebook::resource::ResourceKind;
    /// let kind: ResourceKind = "application/x-dtbncx+xml".into();
    ///
    /// assert!(kind.is_application());
    /// ```
    pub fn is_application(&self) -> bool {
        self.maintype().eq_ignore_ascii_case(Self::_APPLICATION)
    }

    /// Returns `true` if the kind of resource is audio.
    ///
    /// Specifically, `true` is returned if the [`maintype`](Self::maintype)
    /// equals case-insensitive `audio`.
    ///
    /// # Examples
    /// - Assessing if `ogg` is an audio type:
    /// ```
    /// # use rbook::ebook::resource::ResourceKind;
    /// let kind: ResourceKind = "audio/ogg; codecs=opus".into();
    ///
    /// assert!(kind.is_audio());
    /// ```
    pub fn is_audio(&self) -> bool {
        self.maintype().eq_ignore_ascii_case(Self::_AUDIO)
    }

    /// Returns `true` if the kind of resource is a font.
    ///
    /// Specifically, `true` is returned if the [`maintype`](Self::maintype) equals `font`
    /// or the [`subtype`](Self::subtype) matches one of the following:
    /// - `font-` (starts with)
    /// - `x-font` (starts with)
    /// - `vnd.ms-fontobject` (equals)
    /// - `vnd.ms-opentype` (equals)
    ///
    /// # Note
    /// Operations are case-insensitive; capitalization has no effect.
    ///
    /// # Examples
    /// - Assessing if the specified kinds are fonts:
    /// ```
    /// # use rbook::ebook::resource::ResourceKind;
    /// assert!(ResourceKind::from("font/woff").is_font());
    /// // Legacy/obsolete MIME variants:
    /// assert!(ResourceKind::from("application/FONT-WOFF").is_font());
    /// assert!(ResourceKind::from("application/x-FONT-WOFF").is_font());
    /// assert!(ResourceKind::from("application/vnd.ms-fontobject").is_font());
    /// assert!(ResourceKind::from("application/vnd.ms-OpEntYpE").is_font());
    /// ```
    pub fn is_font(&self) -> bool {
        if self.maintype().eq_ignore_ascii_case(Self::_FONT) {
            return true;
        }

        let subtype = self.subtype();
        // Legacy/obsolete MIME handling
        subtype.starts_with_ignore_case("font-")
            || subtype.starts_with_ignore_case("x-font")
            // Special cases regarding EPUB core media
            || subtype.eq_ignore_ascii_case("vnd.ms-fontobject")
            || subtype.eq_ignore_ascii_case("vnd.ms-opentype")
    }

    /// Returns `true` if the kind of resource is an image.
    ///
    /// Specifically, `true` is returned if the [`maintype`](Self::maintype)
    /// equals case-insensitive `image`.
    ///
    /// # Examples
    /// - Assessing if `svg+xml` is an image type:
    /// ```
    /// # use rbook::ebook::resource::ResourceKind;
    /// let kind: ResourceKind = "image/svg+xml".into();
    ///
    /// assert!(kind.is_image());
    /// ```
    pub fn is_image(&self) -> bool {
        self.maintype().eq_ignore_ascii_case(Self::_IMAGE)
    }

    /// Returns `true` if the kind of resource is text-related.
    ///
    /// Specifically, `true` is returned if the [`maintype`](Self::maintype)
    /// equals case-insensitive `text`.
    ///
    /// # Examples
    /// - Assessing if `css` is a text-related type:
    /// ```
    /// # use rbook::ebook::resource::ResourceKind;
    /// let kind: ResourceKind = "text/css".into();
    ///
    /// assert!(kind.is_text());
    /// ```
    pub fn is_text(&self) -> bool {
        self.maintype().eq_ignore_ascii_case(Self::_TEXT)
    }

    /// Returns `true` if the kind of resource is a video.
    ///
    /// Specifically, `true` is returned if the [`maintype`](Self::maintype)
    /// equals case-insensitive `video`.
    ///
    /// # Examples
    /// - Assessing if `mpeg` is a video type:
    /// ```
    /// # use rbook::ebook::resource::ResourceKind;
    /// let kind: ResourceKind = "video/mpeg".into();
    ///
    /// assert!(kind.is_video());
    /// ```
    pub fn is_video(&self) -> bool {
        self.maintype().eq_ignore_ascii_case(Self::_VIDEO)
    }
}

impl PartialEq for ResourceKind<'_> {
    fn eq(&self, other: &Self) -> bool {
        fn extract_type<'a>(kind: &'a ResourceKind) -> (&'a str, bool) {
            let mut split = kind.0.split(';');
            let full_type = split.next().unwrap().trim(); // Split guarantees at least one entry
            let has_params = split.next().is_some();
            (full_type, has_params)
        }

        let (self_type, self_has_params) = extract_type(self);
        let (other_type, other_has_params) = extract_type(other);

        // - Params must match
        // - Types must match (main, sub, and suffix)
        if self_has_params != other_has_params || !self_type.eq_ignore_ascii_case(other_type) {
            return false;
        }
        // If neither has parameters as this point, they're identical.
        if !self_has_params && !other_has_params {
            return true;
        }

        // Compare parameters
        let mut self_params = self.params_iter().collect::<Vec<_>>();
        let mut other_params = other.params_iter().collect::<Vec<_>>();

        if self_params.len() != other_params.len() {
            return false;
        }

        // Sort from [A-z] to ensure proper order (KEY_A == key_a)
        self_params.sort_unstable_by_key(|(k, _)| k.to_ascii_lowercase());
        other_params.sort_unstable_by_key(|(k, _)| k.to_ascii_lowercase());

        // Compare key (case-insensitive) and value (case-sensitive)
        self_params
            .iter()
            .zip(other_params)
            .all(|(&(k1, v1), (k2, v2))| k1.eq_ignore_ascii_case(k2) && v1.eq(v2))
    }
}

impl Display for ResourceKind<'_> {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        f.write_str(self.as_str())
    }
}

impl AsRef<str> for ResourceKind<'_> {
    fn as_ref(&self) -> &str {
        self.as_str()
    }
}

impl<'a> From<&'a str> for ResourceKind<'a> {
    fn from(value: &'a str) -> Self {
        Self(value.trim().into())
    }
}

impl From<String> for ResourceKind<'_> {
    fn from(mut value: String) -> Self {
        value.trim_in_place();
        Self(value.into())
    }
}

impl<'a> From<Cow<'a, str>> for ResourceKind<'a> {
    fn from(value: Cow<'a, str>) -> Self {
        match value {
            Cow::Borrowed(borrowed) => Self::from(borrowed),
            Cow::Owned(owned) => Self::from(owned),
        }
    }
}

impl<'a> From<&'a Self> for ResourceKind<'a> {
    fn from(value: &'a Self) -> Self {
        Self(Cow::Borrowed(value.0.as_ref()))
    }
}
