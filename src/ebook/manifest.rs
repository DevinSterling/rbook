//! Format-agnostic [`Manifest`]-related content.

use crate::ebook::errors::{ArchiveError, EbookResult};
use crate::ebook::resource::{Resource, ResourceKind};

/// The manifest of an [`Ebook`](super::Ebook)
/// encompassing internal [`resources`](Resource) (e.g., images, files, etc.).
///
/// # Examples
/// - Retrieving the cover image from the manifest:
/// ```
/// # use rbook::ebook::errors::EbookResult;
/// # use rbook::ebook::manifest::{Manifest, ManifestEntry};
/// # use rbook::{Ebook, Epub};
/// # fn main() -> EbookResult<()> {
/// let epub = Epub::open("tests/ebooks/example_epub")?;
///
/// let cover_image = epub.manifest().cover_image().unwrap();
/// let resource_kind = cover_image.resource_kind();
///
/// assert!(resource_kind.is_image());
/// assert_eq!("image/webm", resource_kind.as_str());
/// assert_eq!("/EPUB/img/cover.webm", cover_image.href().as_ref());
/// # Ok(())
/// # }
/// ```
/// - Reading a resource provided by the manifest:
/// ```
/// # use rbook::ebook::errors::EbookResult;
/// # use rbook::ebook::manifest::{Manifest, ManifestEntry};
/// # use rbook::{Ebook, Epub};
/// # fn main() -> EbookResult<()> {
/// let epub = Epub::open("tests/ebooks/example_epub")?;
///
/// let chapter_2 = epub.manifest().by_id("c2").unwrap();
///
/// assert_eq!("application/xhtml+xml", chapter_2.media_type());
///
/// let str_1 = chapter_2.read_str()?;
/// // Content can also be retrieved by string:
/// let str_2 = epub.read_resource_str("c2.xhtml")?;
///
/// assert_eq!(str_1, str_2);
/// # Ok(())
/// # }
/// ```
pub trait Manifest<'ebook> {
    /// The total number of [`entries`](ManifestEntry) that make up the manifest.
    fn len(&self) -> usize;

    /// Returns an iterator over all [`entries`](ManifestEntry) in the manifest.
    ///
    /// # Note
    /// Entries may be of different [`ResourceKinds`](ResourceKind)
    /// (e.g., `PNG`, `JPEG`, `CSS`).
    fn entries(&self) -> impl Iterator<Item = impl ManifestEntry<'ebook> + 'ebook> + 'ebook;

    /// The [`ManifestEntry`] of an ebook’s cover image if present.
    ///
    /// # See Also
    /// - [`ManifestEntry::resource_kind`] to inspect the kind of image format.
    fn cover_image(&self) -> Option<impl ManifestEntry<'ebook> + 'ebook>;

    /// Returns an iterator over all image [`entries`](ManifestEntry) in the manifest.
    ///
    /// The iterated entries may correspond to different image kinds,
    /// such as `PNG`, `JPEG`, etc.
    ///
    /// This method provides the same functionality as invoking
    /// [`Self::by_resource_kind`] with the argument as
    /// [`ResourceKind::IMAGE`].
    ///
    /// # See Also
    /// - [`ManifestEntry::resource_kind`] to inspect the exact image kind.
    fn images(&self) -> impl Iterator<Item = impl ManifestEntry<'ebook> + 'ebook> + 'ebook;

    /// Returns an iterator over all "readable content"
    /// [`entries`](ManifestEntry) in the manifest.
    ///
    /// "Readable content" refers to textual resources intended for end-user reading,
    /// such as `XHTML` and `HTML` files.
    ///
    /// # Note
    /// To traverse readable content in canonical reading order, the
    /// [`Spine`](crate::ebook::spine::Spine) is preferred over this method.
    fn readable_content(
        &self,
    ) -> impl Iterator<Item = impl ManifestEntry<'ebook> + 'ebook> + 'ebook;

    /// Returns an iterator over all [`entries`](ManifestEntry) in the
    /// manifest whose resource kind matches the specified [`ResourceKind`].
    fn by_resource_kind(
        &self,
        kind: impl Into<ResourceKind<'ebook>>,
    ) -> impl Iterator<Item = impl ManifestEntry<'ebook> + 'ebook> + 'ebook;

    /// Returns an iterator over all [`entries`](ManifestEntry) in the
    /// manifest whose resource kind matches the specified [`ResourceKinds`](ResourceKind).
    ///
    /// # Examples
    /// - Retrieving entries by multiple kinds from the manifest:
    /// ```
    /// # use rbook::ebook::errors::EbookResult;
    /// # use rbook::ebook::manifest::Manifest;
    /// # use rbook::ebook::resource::ResourceKind;
    /// # use rbook::{Ebook, Epub};
    /// # fn main() -> EbookResult<()> {
    /// let epub = Epub::open("tests/ebooks/example_epub")?;
    /// let manifest = epub.manifest();
    ///
    /// // Retrieving readable content:
    /// let text = manifest.by_resource_kinds([
    ///     "application/xhtml+xml",
    ///     "text/html",
    /// ]);
    ///
    /// assert_eq!(5, text.count());
    ///
    /// // Retrieving media:
    /// let media = manifest.by_resource_kinds([
    ///     ResourceKind::IMAGE,
    ///     ResourceKind::AUDIO,
    ///     ResourceKind::VIDEO,
    /// ]);
    ///
    /// assert_eq!(3, media.count());
    /// # Ok(())
    /// # }
    /// ```
    fn by_resource_kinds(
        &self,
        kinds: impl IntoIterator<Item = impl Into<ResourceKind<'ebook>>>,
    ) -> impl Iterator<Item = impl ManifestEntry<'ebook> + 'ebook> + 'ebook;

    /// Returns `true` if there are no [`entries`](ManifestEntry).
    ///
    /// Generally, manifests are not empty as ebooks *should* have content.
    /// However, this is possible if a feature such as
    /// [`EpubSettings::strict`](crate::epub::EpubSettings::strict) is set to `false`.
    fn is_empty(&self) -> bool {
        self.len() == 0
    }
}

/// An entry contained within a [`Manifest`], encompassing resource-related metadata.
///
/// An entry corresponds to a single [`Resource`] (e.g., an `XHTML` file,
/// a `JPEG` image, a `CSS` stylesheet), providing access to that resource.
pub trait ManifestEntry<'ebook> {
    /// The unique key of an entry within the [`Manifest`].
    fn key(&self) -> Option<&'ebook str>;

    /// The underlying [`Resource`] a manifest entry points to.
    ///
    /// # See Also
    /// - [`Self::read_str`]
    /// - [`Self::read_bytes`]
    ///
    /// # Examples
    /// - Reading a resource provided by the manifest:
    /// ```
    /// # use rbook::ebook::errors::EbookResult;
    /// # use rbook::ebook::manifest::{Manifest, ManifestEntry};
    /// # use rbook::{Ebook, Epub};
    /// # fn main() -> EbookResult<()> {
    /// let epub = Epub::open("tests/ebooks/example_epub")?;
    ///
    /// let chapter_1 = epub.manifest().by_id("c1").unwrap();
    /// let content = epub.read_resource_str(chapter_1.resource())?;
    ///
    /// // process content //
    /// # Ok(())
    /// # }
    /// ```
    fn resource(&self) -> Resource<'ebook>;

    /// The kind of [`ResourceKind`] a manifest entry represents,
    /// such as `XHTML`, `PNG`, `CSS`, etc.
    ///
    /// # Examples
    /// - Observing a manifest entry's resource kind:
    /// ```
    /// # use rbook::ebook::errors::EbookResult;
    /// # use rbook::ebook::manifest::{Manifest, ManifestEntry};
    /// # use rbook::{Ebook, Epub};
    /// # fn main() -> EbookResult<()> {
    /// let epub = Epub::open("tests/ebooks/example_epub")?;
    ///
    /// let chapter_1 = epub.manifest().by_id("c1").unwrap();
    /// let kind = chapter_1.resource_kind();
    ///
    /// assert_eq!("application/xhtml+xml", kind.as_str());
    /// assert_eq!("application", kind.maintype());
    /// assert_eq!("xhtml", kind.subtype());
    /// assert_eq!(Some("xml"), kind.suffix());
    /// # Ok(())
    /// # }
    /// ```
    fn resource_kind(&self) -> ResourceKind<'ebook>;

    /// Returns the associated content in the form of a string.
    ///
    /// This method is equivalent to calling
    /// [`Ebook::read_resource_str`](super::Ebook::read_resource_str) and passing
    /// [`Self::resource`] as the argument.
    ///
    /// # Errors
    /// [`ArchiveError`]: When retrieval of the requested content fails.
    ///
    /// # Examples
    /// - Retrieving the string content associated with a manifest entry:
    /// ```
    /// # use rbook::ebook::errors::EbookResult;
    /// # use rbook::ebook::manifest::{Manifest, ManifestEntry};
    /// # use rbook::{Ebook, Epub};
    /// # fn main() -> EbookResult<()> {
    /// let epub = Epub::open("tests/ebooks/example_epub")?;
    /// let chapter_1 = epub.manifest().by_id("c1").unwrap();
    ///
    /// let xhtml_a = chapter_1.read_str()?;
    /// let xhtml_b = epub.read_resource_str(chapter_1.resource())?;
    ///
    /// assert_eq!(xhtml_a, xhtml_b);
    /// # Ok(())
    /// # }
    /// ```
    fn read_str(&self) -> EbookResult<String> {
        default_placeholder(self.resource())
    }

    /// Returns the associated content in the form of bytes.
    ///
    /// This method is equivalent to calling
    /// [`Ebook::read_resource_bytes`](super::Ebook::read_resource_bytes) and passing
    /// [`Self::resource`] as the argument.
    ///
    /// # Errors
    /// [`ArchiveError`]: When retrieval of the requested content fails.
    ///
    /// # Examples
    /// - Retrieving the byte contents of a [cover image](Manifest::cover_image):
    /// ```
    /// # use rbook::ebook::errors::EbookResult;
    /// # use rbook::ebook::manifest::{Manifest, ManifestEntry};
    /// # use rbook::{Ebook, Epub};
    /// # fn main() -> EbookResult<()> {
    /// let epub = Epub::open("tests/ebooks/example_epub")?;
    /// let cover_image = epub.manifest().cover_image().unwrap();
    ///
    /// let bytes_a = cover_image.read_bytes()?;
    /// let bytes_b = epub.read_resource_bytes(cover_image.resource())?;
    ///
    /// assert_eq!(bytes_a, bytes_b);
    /// # Ok(())
    /// # }
    /// ```
    fn read_bytes(&self) -> EbookResult<Vec<u8>> {
        default_placeholder(self.resource())
    }
}

/// Placeholder introduced in rbook v0.6.3 for backwards-compatibility.
/// This will be removed in v0.7.0 where
/// [`ManifestEntry::read_str`] and [`ManifestEntry::read_bytes`] will not be default methods.
///
/// This method is only invoked if [`ManifestEntry`] is implemented
/// outside `rbook`.
fn default_placeholder<T>(resource: Resource) -> EbookResult<T> {
    Err(ArchiveError::InvalidResource {
        source: std::io::Error::new(
            std::io::ErrorKind::NotFound,
            "The default implementation of `read_str/bytes` will always return this error.",
        ),
        resource: resource.as_static(),
    }
    .into())
}
