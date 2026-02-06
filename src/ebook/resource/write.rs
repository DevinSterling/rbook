use crate::util::uri;
use std::borrow::Cow;
use std::path::{Path, PathBuf};

/// The source of a resource's content, either stored
/// [in-memory](Self::Memory) or as a pointer to a [file](Self::File) on disk.
///
/// # Conversions
/// For convenience, this enum implements [`From`] for the following types:
/// - **Bytes** (`&[u8]`, `Vec<u8>`): Treated as raw binary data (stored in memory).
/// - **Strings** (`&str`, `String`): Treated as literal UTF-8 content (stored in memory).
/// - **Paths** (`&Path`, `PathBuf`):
///   Treated as a pointer to a file on disk.
///
///   When providing a file path, the contained content is managed
///   by the internal archive of an [`Ebook`](crate::Ebook) and retrieved on
///   demand when requested.
///
/// # Examples
/// - Providing resource content to an [`Epub`](crate::Epub):
/// ```
/// # use rbook::ebook::resource::ResourceContent;
/// # use rbook::Epub;
/// # use std::path::PathBuf;
///
/// let bytes_content = ResourceContent::memory(vec![/*...*/]);
/// let file_content = ResourceContent::file("local/images/art4.png");
///
/// let epub = Epub::builder()
///     // In-memory
///     .resource(("art0.png", b"..."))
///     .resource(("art1.png", vec![/*...*/]))
///     .resource(("art2.png", bytes_content))
///     // Referencing a local file stored on disk
///     .resource(("art3.png", PathBuf::from("local/images/art3.png")))
///     .resource(("art4.png", file_content))
///     .build();
/// ```
#[derive(Clone, Debug, PartialEq)]
pub enum ResourceContent {
    /// Content stored in-memory.
    ///
    /// # See Also
    /// - [`Self::memory`] to conveniently create an instance of this variant.
    Memory(Vec<u8>),
    /// A path to a file on disk (referencing content in the OS file system).
    ///
    /// The content at this path is managed by the internal archive of an
    /// [`Ebook`](crate::Ebook) and retrieved on demand when requested.
    ///
    /// This is preferred over [`Self::Memory`] when the space in RAM is a constraint.
    ///
    /// # See Also
    /// - [`Self::file`] to conveniently create an instance of this variant.
    File(PathBuf),
}

impl ResourceContent {
    /// Creates a [`Self::Memory`] instance with the given `buffer`.
    pub fn memory(buffer: impl Into<Vec<u8>>) -> Self {
        Self::Memory(buffer.into())
    }

    /// Creates a [`Self::File`] instance with the given `path`.
    pub fn file(path: impl Into<PathBuf>) -> Self {
        Self::File(path.into())
    }

    /// Returns `true` if the content is [`ResourceContent::Memory`].
    ///
    /// # Examples
    /// - Assessing in-memory content:
    /// ```
    /// # use rbook::ebook::resource::ResourceContent;
    /// let content = ResourceContent::memory(b"data");
    ///
    /// assert!(content.is_memory());
    /// assert!(!content.is_file());
    /// ```
    pub fn is_memory(&self) -> bool {
        matches!(self, ResourceContent::Memory(_))
    }

    /// Returns `true` if the content is [`ResourceContent::File`].
    ///
    /// # Examples
    /// - Assessing a file reference:
    /// ```
    /// # use rbook::ebook::resource::ResourceContent;
    /// let content = ResourceContent::file("path/to/file");
    ///
    /// assert!(content.is_file());
    /// assert!(!content.is_memory());
    /// ```
    pub fn is_file(&self) -> bool {
        matches!(self, ResourceContent::File(_))
    }
}

impl From<Vec<u8>> for ResourceContent {
    fn from(value: Vec<u8>) -> Self {
        Self::Memory(value)
    }
}

impl From<&[u8]> for ResourceContent {
    fn from(value: &[u8]) -> Self {
        Self::Memory(value.to_vec())
    }
}

impl<const N: usize> From<&[u8; N]> for ResourceContent {
    fn from(value: &[u8; N]) -> Self {
        Self::Memory(value.to_vec())
    }
}

impl From<Cow<'_, [u8]>> for ResourceContent {
    fn from(value: Cow<'_, [u8]>) -> Self {
        Self::Memory(value.into_owned())
    }
}

impl From<String> for ResourceContent {
    fn from(value: String) -> Self {
        Self::Memory(value.into_bytes())
    }
}

impl From<&str> for ResourceContent {
    fn from(value: &str) -> Self {
        Self::Memory(value.as_bytes().to_vec())
    }
}

impl From<Cow<'_, str>> for ResourceContent {
    fn from(value: Cow<'_, str>) -> Self {
        Self::Memory(value.into_owned().into_bytes())
    }
}

impl From<PathBuf> for ResourceContent {
    fn from(path: PathBuf) -> Self {
        Self::File(path)
    }
}

impl From<&Path> for ResourceContent {
    fn from(path: &Path) -> Self {
        Self::File(path.to_path_buf())
    }
}

// In the future, this will be optimized to not allocate
pub(crate) fn infer_media_type(href: &str) -> String {
    let extension = uri::file_extension(href).unwrap_or_default();

    match extension {
        // Images
        "jpg" | "jpeg" => "image/jpeg",
        "png" => "image/png",
        "svg" => "image/svg+xml",
        "gif" => "image/gif",
        "webp" => "image/webp",

        // Text
        "xhtml" => "application/xhtml+xml",
        "html" | "htm" => "text/html",
        "css" => "text/css",
        "js" => "text/javascript",
        "smil" => "application/smil+xml",
        "ncx" => "application/x-dtbncx+xml",
        "xml" => "application/xml",

        // Fonts
        "ttf" => "font/ttf",
        "otf" => "font/otf",
        "woff" => "font/woff",
        "woff2" => "font/woff2",

        // Audio
        "mp3" => "audio/mpeg",
        "m4a" => "audio/mp4",
        "aac" => "audio/aac",

        // Video
        "mp4" | "m4v" => "video/mp4",
        "webm" => "video/webm",

        _ => "application/octet-stream",
    }
    .to_owned()
}
