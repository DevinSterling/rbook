//! Format-agnostic [`Manifest`]-related content.
//!
//! # See Also
//! - [`epub::manifest`][crate::epub::manifest] for the epub-specific manifest module.

use crate::ebook::archive;
use crate::ebook::archive::errors::ArchiveResult;
use crate::ebook::resource::{Resource, ResourceKind};
use crate::input::Many;
use crate::util::Sealed;
use std::io::Write;

/// The manifest of an [`Ebook`](super::Ebook)
/// encompassing internal [`resources`](Resource) (e.g., images, files, etc.).
///
/// # See Also
/// - [`EpubManifest`](crate::epub::manifest::EpubManifest) for epub-specific manifest information.
///
/// # Examples
/// - Retrieving the cover image from the manifest:
/// ```
/// # use rbook::Epub;
/// # fn main() -> rbook::ebook::errors::EbookResult<()> {
/// let epub = Epub::open("tests/ebooks/example_epub")?;
///
/// let cover_image = epub.manifest().cover_image().unwrap();
/// let resource_kind = cover_image.kind();
///
/// assert!(resource_kind.is_image());
/// assert_eq!("webm", resource_kind.subtype());
/// assert_eq!("/EPUB/img/cover.webm", cover_image.href().as_ref());
/// # Ok(())
/// # }
/// ```
/// - Reading a resource from the manifest:
/// ```
/// # use rbook::Epub;
/// # fn main() -> rbook::ebook::errors::EbookResult<()> {
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
pub trait Manifest<'ebook>: Sealed {
    /// The total number of [entries](ManifestEntry) that makes up the manifest.
    fn len(&self) -> usize;

    /// Returns an iterator over all [entries](ManifestEntry) in the manifest.
    ///
    /// # Note
    /// Entries may be of different [`ResourceKinds`](ResourceKind)
    /// (e.g., `PNG`, `JPEG`, `CSS`).
    fn iter(&self) -> impl Iterator<Item = impl ManifestEntry<'ebook> + 'ebook> + 'ebook;

    /// The [`ManifestEntry`] of an ebook’s cover image, if present.
    ///
    /// # See Also
    /// - [`ManifestEntry::kind`] to inspect the kind of image format.
    ///
    /// # Examples
    /// - Retrieving cover image hrefs from the manifest:
    /// ```
    /// # use rbook::Epub;
    /// # use rbook::epub::metadata::EpubVersion;
    /// # fn main() -> rbook::ebook::errors::EbookResult<()> {
    /// let epub2 = Epub::open("tests/ebooks/epub2")?;
    /// let epub3 = Epub::open("tests/ebooks/example_epub")?;
    ///
    /// let epub2_cover = epub2.manifest().cover_image().unwrap();
    /// assert_eq!(epub2_cover.href().as_ref(), "/cover.jpg" );
    /// assert_eq!(epub2_cover.kind().subtype(), "jpg");
    ///
    /// let epub3_cover = epub3.manifest().cover_image().unwrap();
    /// assert_eq!(epub3_cover.href().as_ref(), "/EPUB/img/cover.webm");
    /// assert_eq!(epub3_cover.kind().subtype(), "webm");
    /// # Ok(())
    /// # }
    /// ```
    fn cover_image(&self) -> Option<impl ManifestEntry<'ebook> + 'ebook>;

    /// Returns an iterator over all image [entries](ManifestEntry) in the manifest.
    ///
    /// The iterated entries may correspond to different image kinds,
    /// such as `PNG`, `JPEG`, etc.
    ///
    /// This method provides the same functionality as invoking
    /// [`Self::by_kind`] with the argument as
    /// [`ResourceKind::IMAGE`].
    ///
    /// # See Also
    /// - [`ManifestEntry::kind`] to inspect the exact image kind.
    fn images(&self) -> impl Iterator<Item = impl ManifestEntry<'ebook> + 'ebook> + 'ebook;

    /// Returns an iterator over JavaScript [entries](ManifestEntry) in the manifest.
    ///
    /// The iterated entries may correspond to different script kinds, specifically
    /// the EPUB-spec's core media types for scripts:
    /// - `application/javascript`
    /// - `application/ecmascript`
    /// - `text/javascript`
    ///
    /// # See Also
    /// - [`ManifestEntry::kind`] to inspect the exact script kind.
    fn scripts(&self) -> impl Iterator<Item = impl ManifestEntry<'ebook> + 'ebook> + 'ebook;

    /// Returns an iterator over CSS stylesheet [entries](ManifestEntry) in the manifest.
    ///
    /// All iterated entries will have a media type of `text/css`.
    ///
    /// # See Also
    /// - [`ManifestEntry::kind`] to inspect the exact resource kind.
    fn styles(&self) -> impl Iterator<Item = impl ManifestEntry<'ebook> + 'ebook> + 'ebook;

    /// Returns an iterator over all font [entries](ManifestEntry) in the manifest,
    /// including legacy font MIMEs (e.g., `application/font-woff`).
    ///
    /// The iterated entries may correspond to different font kinds,
    /// such as `TTF`, `WOFF`, etc.
    ///
    /// # Note
    /// This method behaves differently compared to invoking
    /// [`Self::by_kind`] with the argument as
    /// [`ResourceKind::FONT`], which checks if MIMEs match the pattern `font/*`.
    ///
    /// # See Also
    /// - [`ManifestEntry::kind`] to inspect the exact font kind.
    fn fonts(&self) -> impl Iterator<Item = impl ManifestEntry<'ebook> + 'ebook> + 'ebook;

    /// Returns an iterator over all audio [entries](ManifestEntry) in the manifest.
    ///
    /// The iterated entries may correspond to different audio kinds,
    /// such as `MP3`, `AAC`, etc.
    ///
    /// This method provides the same functionality as invoking
    /// [`Self::by_kind`] with the argument as
    /// [`ResourceKind::AUDIO`].
    ///
    /// # See Also
    /// - [`ManifestEntry::kind`] to inspect the exact audio kind.
    fn audio(&self) -> impl Iterator<Item = impl ManifestEntry<'ebook> + 'ebook> + 'ebook;

    /// Returns an iterator over all video [entries](ManifestEntry) in the manifest.
    ///
    /// The iterated entries may correspond to different video kinds,
    /// such as `MP4`, `WEBM`, etc.
    ///
    /// This method provides the same functionality as invoking
    /// [`Self::by_kind`] with the argument as
    /// [`ResourceKind::VIDEO`].
    ///
    /// # See Also
    /// - [`ManifestEntry::kind`] to inspect the exact video kind.
    fn video(&self) -> impl Iterator<Item = impl ManifestEntry<'ebook> + 'ebook> + 'ebook;

    /// Returns an iterator over all "readable content"
    /// [entries](ManifestEntry) in the manifest.
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

    /// Returns an iterator over all [entries](ManifestEntry) in the manifest whose
    /// resource kind matches the specified [`ResourceKind`] via the [`Many`] trait.
    ///
    /// # Examples
    /// - Retrieving entries by multiple kinds from the manifest:
    /// ```
    /// # use rbook::Epub;
    /// # use rbook::ebook::resource::ResourceKind;
    /// # fn main() -> rbook::ebook::errors::EbookResult<()> {
    /// let epub = Epub::open("tests/ebooks/example_epub")?;
    /// let manifest = epub.manifest();
    ///
    /// let css = manifest.by_kind("text/css");
    /// assert_eq!(1, css.count());
    ///
    /// // Retrieving readable content:
    /// let text = manifest.by_kind([
    ///     "application/xhtml+xml",
    ///     "text/html",
    /// ]);
    /// assert_eq!(5, text.count());
    ///
    /// // Retrieving media:
    /// let media = manifest.by_kind([
    ///     ResourceKind::IMAGE,
    ///     ResourceKind::AUDIO,
    ///     ResourceKind::VIDEO,
    /// ]);
    /// assert_eq!(3, media.count());
    /// # Ok(())
    /// # }
    /// ```
    fn by_kind(
        &self,
        kind: impl Many<ResourceKind<'ebook>>,
    ) -> impl Iterator<Item = impl ManifestEntry<'ebook> + 'ebook> + 'ebook;

    /// Returns `true` if there are no [entries](ManifestEntry).
    ///
    /// Generally, manifests are not empty as ebooks *should* have content.
    /// However, this is possible if a feature such as
    /// [`EpubOpenOptions::strict`](crate::epub::EpubOpenOptions::strict) is set to `false`.
    fn is_empty(&self) -> bool {
        self.len() == 0
    }
}

/// An entry contained within a [`Manifest`], encompassing resource-related metadata.
///
/// An entry corresponds to a single [`Resource`] (e.g., an `XHTML` file,
/// a `JPEG` image, a `CSS` stylesheet), providing access to that resource.
///
/// # See Also
/// - [`EpubManifestEntry`](crate::epub::manifest::EpubManifestEntry)
///   for epub-specific entry information.
pub trait ManifestEntry<'ebook>: Sealed {
    /// The underlying [`Resource`] a manifest entry points to.
    ///
    /// # See Also
    /// - [`Self::read_str`] to retrieve the string content of a manifest entry.
    /// - [`Self::read_bytes`] to retrieve the raw byte content of a manifest entry.
    ///
    /// # Examples
    /// - Reading a resource from the manifest:
    /// ```
    /// # use rbook::Epub;
    /// # fn main() -> rbook::ebook::errors::EbookResult<()> {
    /// let epub = Epub::open("tests/ebooks/example_epub")?;
    ///
    /// let chapter_1 = epub.manifest().by_id("c1").unwrap();
    /// let content = epub.read_resource_str(chapter_1)?;
    ///
    /// // process content //
    /// # Ok(())
    /// # }
    /// ```
    fn resource(&self) -> Resource<'ebook>;

    /// The [`ResourceKind`] a manifest entry represents,
    /// such as `XHTML`, `PNG`, `CSS`, etc.
    ///
    /// # Examples
    /// - Observing a manifest entry's resource kind:
    /// ```
    /// # use rbook::Epub;
    /// # fn main() -> rbook::ebook::errors::EbookResult<()> {
    /// let epub = Epub::open("tests/ebooks/example_epub")?;
    ///
    /// let chapter_1 = epub.manifest().by_id("c1").unwrap();
    /// let kind = chapter_1.kind();
    ///
    /// assert_eq!("application/xhtml+xml", kind.as_str());
    /// assert_eq!("application", kind.maintype());
    /// assert_eq!("xhtml", kind.subtype());
    /// assert_eq!(Some("xml"), kind.suffix());
    /// # Ok(())
    /// # }
    /// ```
    fn kind(&self) -> ResourceKind<'ebook>;

    /// Copies the associated content into the given `writer`,
    /// returning the total number of bytes written on success.
    ///
    /// This method is similar to calling
    /// [`Ebook::copy_resource`](super::Ebook::copy_resource) and passing
    /// [`Self::resource`] and the given `writer` as arguments.
    ///
    /// # Errors
    /// [`ArchiveError`](archive::errors::ArchiveError):
    /// When copying the requested content fails.
    ///
    /// # Examples
    /// - Copying the byte contents of a [cover image](Manifest::cover_image):
    /// ```
    /// # use rbook::Epub;
    /// # fn main() -> rbook::ebook::errors::EbookResult<()> {
    /// let epub = Epub::open("tests/ebooks/example_epub")?;
    /// let cover_image = epub.manifest().cover_image().unwrap();
    ///
    /// // While a `Vec` is used here, any implementation of `Write` is supported.
    /// let mut vec_a = Vec::new();
    /// let mut vec_b = Vec::new();
    ///
    /// let bytes_written_a = cover_image.copy_bytes(&mut vec_a)?;
    /// let bytes_written_b = epub.copy_resource(cover_image, &mut vec_b)?;
    ///
    /// assert_eq!(vec_a, vec_b);
    /// assert_eq!(bytes_written_a, bytes_written_b);
    /// # Ok(())
    /// # }
    /// ```
    fn copy_bytes(&self, writer: &mut impl Write) -> ArchiveResult<u64>;

    /// Returns the associated content as a [`String`].
    ///
    /// This method is similar to calling
    /// [`Ebook::read_resource_str`](super::Ebook::read_resource_str) and passing
    /// [`Self::resource`] as the argument.
    ///
    /// # Errors
    /// [`ArchiveError`](archive::errors::ArchiveError):
    /// When retrieval of the requested content fails.
    ///
    /// # Examples
    /// - Retrieving the string content associated with a manifest entry:
    /// ```
    /// # use rbook::Epub;
    /// # fn main() -> rbook::ebook::errors::EbookResult<()> {
    /// let epub = Epub::open("tests/ebooks/example_epub")?;
    /// let chapter_1 = epub.manifest().by_id("c1").unwrap();
    ///
    /// let xhtml_a = chapter_1.read_str()?;
    /// let xhtml_b = epub.read_resource_str(chapter_1)?;
    ///
    /// assert_eq!(xhtml_a, xhtml_b);
    /// # Ok(())
    /// # }
    /// ```
    fn read_str(&self) -> ArchiveResult<String> {
        archive::into_utf8_string(&self.resource(), self.read_bytes()?)
    }

    /// Returns the associated content as bytes.
    ///
    /// This method is similar to calling
    /// [`Ebook::read_resource_bytes`](super::Ebook::read_resource_bytes) and passing
    /// [`Self::resource`] as the argument.
    ///
    /// # Errors
    /// [`ArchiveError`](archive::errors::ArchiveError):
    /// When retrieval of the requested content fails.
    ///
    /// # Examples
    /// - Retrieving the byte contents of a [cover image](Manifest::cover_image):
    /// ```
    /// # use rbook::Epub;
    /// # fn main() -> rbook::ebook::errors::EbookResult<()> {
    /// let epub = Epub::open("tests/ebooks/example_epub")?;
    /// let cover_image = epub.manifest().cover_image().unwrap();
    ///
    /// let bytes_a = cover_image.read_bytes()?;
    /// let bytes_b = epub.read_resource_bytes(cover_image)?;
    ///
    /// assert_eq!(bytes_a, bytes_b);
    /// # Ok(())
    /// # }
    /// ```
    fn read_bytes(&self) -> ArchiveResult<Vec<u8>> {
        let mut vec = Vec::new();
        self.copy_bytes(&mut vec)?;
        Ok(vec)
    }
}
