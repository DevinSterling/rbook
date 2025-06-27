//! Format-agnostic [`Manifest`]-related content.

use crate::ebook::resource::{Resource, ResourceKind};

/// The manifest of an [`Ebook`](super::Ebook)
/// encompassing all internal [`resources`](Resource) (e.g., images, files, etc.).
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
/// let resource = chapter_2.resource();
///
/// assert_eq!("application/xhtml+xml", resource.kind().as_str());
///
/// let str_1 = epub.read_resource_str(resource)?;
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
    fn entries(&self) -> impl Iterator<Item = impl ManifestEntry<'ebook>> + 'ebook;

    /// The [`ManifestEntry`] of an ebook’s cover image if present.
    ///
    /// To inspect the kind of image format, see [ManifestEntry::resource_kind].
    fn cover_image(&self) -> Option<impl ManifestEntry<'ebook>>;

    /// Returns an iterator over all image [`entries`](ManifestEntry) in the manifest.
    ///
    /// The iterated entries may correspond to different image kinds,
    /// such as `PNG`, `JPEG`, etc. To inspect the exact kind, see
    /// [ManifestEntry::resource_kind].
    ///
    /// This method provides the same functionality as invoking
    /// [`Self::by_resource_kind`] with the argument as
    /// [`ResourceKind::IMAGE`].
    fn images(&self) -> impl Iterator<Item = impl ManifestEntry<'ebook>> + 'ebook;

    /// Returns an iterator over all "readable content"
    /// [`entries`](ManifestEntry) in the manifest.
    ///
    /// "Readable content" refers to textual resources intended for end-user reading,
    /// such as `XHTML` and `HTML` files.
    ///
    /// # Note
    /// To traverse readable content in canonical reading order, the
    /// [`Spine`](crate::ebook::spine::Spine) is preferred over this method.
    fn readable_content(&self) -> impl Iterator<Item = impl ManifestEntry<'ebook>> + 'ebook;

    /// Returns an iterator over all [`entries`](ManifestEntry) in the
    /// manifest whose resource kind matches the specified [`ResourceKind`].
    fn by_resource_kind(
        &self,
        kind: impl Into<ResourceKind<'ebook>>,
    ) -> impl Iterator<Item = impl ManifestEntry<'ebook>> + 'ebook;

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
    ) -> impl Iterator<Item = impl ManifestEntry<'ebook>> + 'ebook;

    /// Returns `true` if there are no [`entries`](ManifestEntry), otherwise `false`.
    fn is_empty(&self) -> bool {
        self.len() == 0
    }
}

/// An entry contained within a [`Manifest`], encompassing associated metadata.
///
/// An entry corresponds to a single [`Resource`] (e.g., an `XHTML` file,
/// a `JPEG` image, a `CSS` stylesheet), providing access to that resource.
pub trait ManifestEntry<'ebook> {
    /// The unique key of an entry within the [`Manifest`].
    fn key(&self) -> Option<&'ebook str>;

    /// The underlying [`Resource`] a manifest entry points to.
    fn resource(&self) -> Resource<'ebook>;

    /// The kind of [`ResourceKind`] a manifest entry represents,
    /// such as `XHTML`, `PNG`, `CSS`, etc.
    fn resource_kind(&self) -> ResourceKind<'ebook>;
}
