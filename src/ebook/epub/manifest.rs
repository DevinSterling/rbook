//! EPUB-specific manifest content.
//!
//! # See Also
//! - [`ebook::manifest`](crate::ebook::manifest) for the general manifest module.

#[cfg(feature = "write")]
mod write;

use crate::ebook::archive::ResourceProvider;
use crate::ebook::archive::errors::ArchiveResult;
use crate::ebook::element::{Attributes, AttributesData, Href, Properties, PropertiesData};
use crate::ebook::epub::consts::opf;
use crate::ebook::epub::metadata::{EpubMetadataData, EpubRefinements, EpubRefinementsData};
use crate::ebook::epub::package::EpubPackageMetaContext;
use crate::ebook::manifest::{Manifest, ManifestEntry};
use crate::ebook::resource::consts::mime;
use crate::ebook::resource::{Resource, ResourceKind};
use crate::input::Many;
use crate::util::{self, Sealed};
use indexmap::IndexMap;
use indexmap::map::Iter as HashMapIter;
use std::collections::HashSet;
use std::fmt::Debug;
use std::io::Write;

#[cfg(feature = "write")]
pub use write::{
    DetachedEpubManifestEntry, EpubManifestEntryMut, EpubManifestMut, EpubManifestMutIter,
    HrefOptions, IdOptions,
};

////////////////////////////////////////////////////////////////////////////////
// PRIVATE API
////////////////////////////////////////////////////////////////////////////////

/// The kinds of readable content for an epub intended for end-user reading,
/// typically `application/xhtml+xml`.
/// `text/html` is possible as well, although not as common.
const READABLE_CONTENT_MIME: [&str; 2] = [mime::XHTML, mime::HTML];
const SCRIPTS_MIME: [&str; 3] = [mime::JAVASCRIPT, mime::ECMASCRIPT, mime::JAVASCRIPT_TEXT];

#[derive(Debug, PartialEq)]
pub(super) struct EpubManifestData {
    pub(super) entries: IndexMap<String, EpubManifestEntryData>,
}

impl EpubManifestData {
    pub(super) fn new(entries: IndexMap<String, EpubManifestEntryData>) -> Self {
        Self { entries }
    }

    pub(super) fn empty() -> Self {
        Self {
            entries: IndexMap::new(),
        }
    }

    pub(super) fn by_id(&self, id: &str) -> Option<(&String, &EpubManifestEntryData)> {
        self.entries.get_key_value(id)
    }

    pub(super) fn by_href(&self, href: &str) -> Option<(&String, &EpubManifestEntryData)> {
        self.entries
            .iter()
            .find(|(_, entry)| entry.href == href || entry.href_raw == href)
    }

    pub(super) fn iter(&self) -> HashMapIter<'_, String, EpubManifestEntryData> {
        self.entries.iter()
    }
}

#[derive(Clone, Debug, Default, PartialEq)]
pub(super) struct EpubManifestEntryData {
    /// The resolved ***absolute*** href
    pub(super) href: String,
    /// The source ***relative*** href
    pub(super) href_raw: String,
    pub(super) media_type: String,
    pub(super) fallback: Option<String>,
    pub(super) media_overlay: Option<String>,
    pub(super) properties: PropertiesData,
    pub(super) attributes: AttributesData,
    pub(super) refinements: EpubRefinementsData,
}

/// The context of an [`EpubManifestEntry`] for fallback lookup and raw resource retrieval.
#[derive(Copy, Clone)]
pub(super) struct EpubManifestContext<'ebook> {
    resource: ResourceProvider<'ebook>,
    meta_ctx: EpubPackageMetaContext<'ebook>,
    manifest: Option<&'ebook EpubManifestData>,
}

impl<'ebook> EpubManifestContext<'ebook> {
    #[cfg(feature = "write")]
    pub(super) const EMPTY: EpubManifestContext<'static> =
        EpubManifestContext::new(ResourceProvider::Empty, EpubPackageMetaContext::EMPTY, None);

    pub(super) const fn new(
        resource: ResourceProvider<'ebook>,
        meta_ctx: EpubPackageMetaContext<'ebook>,
        manifest: Option<&'ebook EpubManifestData>,
    ) -> Self {
        Self {
            resource,
            meta_ctx,
            manifest,
        }
    }

    pub(super) fn create_entry(
        self,
        id: &'ebook str,
        data: &'ebook EpubManifestEntryData,
    ) -> EpubManifestEntry<'ebook> {
        EpubManifestEntry {
            ctx: self,
            id,
            data,
        }
    }

    pub(super) fn by_id(&self, id: &str) -> Option<EpubManifestEntry<'ebook>> {
        self.manifest?
            .by_id(id)
            .map(|(id, data)| self.create_entry(id, data))
    }

    pub(super) fn by_href(&self, href: &str) -> Option<EpubManifestEntry<'ebook>> {
        self.manifest?
            .by_href(href)
            .map(|(id, data)| self.create_entry(id, data))
    }
}

impl<'ebook> From<EpubManifest<'ebook>> for EpubManifestContext<'ebook> {
    fn from(manifest: EpubManifest<'ebook>) -> Self {
        manifest.ctx
    }
}

////////////////////////////////////////////////////////////////////////////////
// PUBLIC API
////////////////////////////////////////////////////////////////////////////////

/// The EPUB manifest accessible via [`Epub::manifest`](super::Epub::manifest).
/// See [`Manifest`] for more details.
///
/// Retrieving entries from the manifest using methods such as [`EpubManifest::by_href`]
/// is generally a linear (`O(N)`) operation except for [`EpubManifest::by_id`],
/// which is constant (`O(1)`).
///
/// # Ordering
/// Methods that return iterators yield entries in their original order.
///
/// # See Also
/// - [`EpubManifestMut`] for a mutable view.
#[derive(Copy, Clone)]
pub struct EpubManifest<'ebook> {
    ctx: EpubManifestContext<'ebook>,
    manifest: &'ebook EpubManifestData,

    // Metadata may include references that assist manifest item lookup.
    // For example, looking up an EPUB 2 cover image.
    metadata: &'ebook EpubMetadataData,
}

impl<'ebook> EpubManifest<'ebook> {
    pub(super) fn new(
        manifest_provider: ResourceProvider<'ebook>,
        package_ctx: EpubPackageMetaContext<'ebook>,
        manifest: &'ebook EpubManifestData,
        metadata: &'ebook EpubMetadataData,
    ) -> Self {
        Self {
            ctx: EpubManifestContext::new(manifest_provider, package_ctx, Some(manifest)),
            manifest,
            metadata,
        }
    }

    /// Returns the [`EpubManifestEntry`] matching the given `id`, or [`None`] if not found.
    ///
    /// This is a constant (`O(1)`) operation.
    pub fn by_id(&self, id: &str) -> Option<EpubManifestEntry<'ebook>> {
        self.manifest
            .by_id(id)
            .map(|(id, data)| self.ctx.create_entry(id, data))
    }

    /// Returns the [`EpubManifestEntry`] matching the given `href`, or [`None`] if not found.
    ///
    /// # Note
    /// The given `href` is ***not*** normalized or percent-decoded.
    /// It is compared **case-sensitively** against both [`EpubManifestEntry::href()`] and
    /// [`EpubManifestEntry::href_raw()`].
    ///
    /// [`Self::by_id`] is recommended over this method,
    /// as this method performs a linear `O(N)` search.
    pub fn by_href(&self, href: &str) -> Option<EpubManifestEntry<'ebook>> {
        self.manifest
            .by_href(href)
            .map(|(id, data)| self.ctx.create_entry(id, data))
    }

    /// Returns an iterator over all [entries](EpubManifestEntry) in the
    /// [`manifest`](EpubManifest) whose [`properties`](EpubManifestEntry::properties)
    /// contains the specified `property`.
    pub fn by_property(
        &self,
        property: &'ebook str,
    ) -> impl Iterator<Item = EpubManifestEntry<'ebook>> + 'ebook {
        let ctx = self.ctx;

        self.manifest
            .iter()
            .filter(|(_, data)| data.properties.has_property(property))
            .map(move |(id, data)| ctx.create_entry(id, data))
    }

    /// The total number of [entries](EpubManifestEntry) that makes up the manifest.
    #[doc = util::inherent_doc!(Manifest, len)]
    pub fn len(&self) -> usize {
        self.manifest.entries.len()
    }

    /// Returns `true` if there are no [entries](EpubManifestEntry).
    #[doc = util::inherent_doc!(Manifest, is_empty)]
    pub fn is_empty(&self) -> bool {
        Manifest::is_empty(self)
    }

    /// Returns an iterator over all [entries](EpubManifestEntry) in the manifest.
    #[doc = util::inherent_doc!(Manifest, iter)]
    pub fn iter(&self) -> EpubManifestIter<'ebook> {
        EpubManifestIter {
            ctx: self.ctx,
            iter: self.manifest.iter(),
        }
    }

    /// The [`EpubManifestEntry`] of an ebook’s cover image, if present.
    ///
    /// This method returns the entry with the `cover-image` property,
    /// falling back to EPUB 2 cover metadata for lookup.
    #[doc = util::inherent_doc!(Manifest, cover_image)]
    pub fn cover_image(&self) -> Option<EpubManifestEntry<'ebook>> {
        self.by_property(opf::COVER_IMAGE).next().or_else(|| {
            // Fallback to EPUB 2 cover image, if present
            self.metadata
                .epub2_cover_image_id()
                .and_then(|id| self.by_id(id))
        })
    }

    /// Returns an iterator over all image [entries](EpubManifestEntry) in the manifest.
    #[doc = util::inherent_doc!(Manifest, images)]
    pub fn images(&self) -> impl Iterator<Item = EpubManifestEntry<'ebook>> + 'ebook {
        self.by_kind(ResourceKind::IMAGE)
    }

    /// Returns an iterator over JavaScript [entries](EpubManifestEntry) in the manifest.
    #[doc = util::inherent_doc!(Manifest, scripts)]
    pub fn scripts(&self) -> impl Iterator<Item = EpubManifestEntry<'ebook>> + 'ebook {
        self.by_kind(SCRIPTS_MIME)
    }

    /// Returns an iterator over CSS stylesheet [entries](EpubManifestEntry) in the manifest.
    #[doc = util::inherent_doc!(Manifest, styles)]
    pub fn styles(&self) -> impl Iterator<Item = EpubManifestEntry<'ebook>> + 'ebook {
        self.by_kind(mime::CSS)
    }

    /// Returns an iterator over all font [entries](EpubManifestEntry) in the manifest,
    /// including legacy font MIMEs (e.g., `application/font-woff`).
    #[doc = util::inherent_doc!(Manifest, fonts)]
    pub fn fonts(&self) -> impl Iterator<Item = EpubManifestEntry<'ebook>> + 'ebook {
        self.iter()
            // Filtering is preferred over `by_kind(ResourceKind::FONT)`
            // as that method retrieves all entries whose MIME match `font/*`.
            // > older EPUB-compatible font types start with `application/*`
            .filter(|entry| entry.kind().is_font())
    }

    /// Returns an iterator over all audio [entries](EpubManifestEntry) in the manifest.
    #[doc = util::inherent_doc!(Manifest, audio)]
    pub fn audio(&self) -> impl Iterator<Item = EpubManifestEntry<'ebook>> + 'ebook {
        self.by_kind(ResourceKind::AUDIO)
    }

    /// Returns an iterator over all video [entries](EpubManifestEntry) in the manifest.
    #[doc = util::inherent_doc!(Manifest, video)]
    pub fn video(&self) -> impl Iterator<Item = EpubManifestEntry<'ebook>> + 'ebook {
        self.by_kind(ResourceKind::VIDEO)
    }

    /// Returns an iterator over all "readable content" [entries](EpubManifestEntry)
    /// in the manifest.
    #[doc = util::inherent_doc!(Manifest, readable_content)]
    pub fn readable_content(&self) -> impl Iterator<Item = EpubManifestEntry<'ebook>> + 'ebook {
        self.by_kind(READABLE_CONTENT_MIME)
    }

    /// Returns an iterator over all [entries](ManifestEntry) in the
    /// manifest whose resource kind matches the specified [`ResourceKind`] via the [`Many`] trait.
    #[doc = util::inherent_doc!(Manifest, by_kind)]
    pub fn by_kind(
        &self,
        kind: impl Many<ResourceKind<'ebook>>,
    ) -> impl Iterator<Item = EpubManifestEntry<'ebook>> + 'ebook {
        // If allocation ever becomes a bottleneck here, this logic can be
        // further optimized despite the cost of greater complexity.
        let targets: Vec<_> = kind.iter_many().collect();
        let ctx = self.ctx;

        self.manifest
            .iter()
            .filter(move |(_, data)| {
                let kind = ResourceKind::from(data.media_type.as_str());

                targets.iter().any(|target| {
                    if target.is_unspecified() {
                        // If the kind is unspecified, get the maintype as a "catch-all"
                        target.maintype().eq_ignore_ascii_case(kind.maintype())
                    } else {
                        target.as_str().eq_ignore_ascii_case(kind.as_str())
                    }
                })
            })
            .map(move |(id, data)| ctx.create_entry(id, data))
    }
}

impl Sealed for EpubManifest<'_> {}

#[allow(refining_impl_trait)]
impl<'ebook> Manifest<'ebook> for EpubManifest<'ebook> {
    fn len(&self) -> usize {
        self.len()
    }

    fn iter(&self) -> EpubManifestIter<'ebook> {
        self.iter()
    }

    fn cover_image(&self) -> Option<EpubManifestEntry<'ebook>> {
        self.cover_image()
    }

    fn images(&self) -> impl Iterator<Item = EpubManifestEntry<'ebook>> + 'ebook {
        self.images()
    }

    fn scripts(&self) -> impl Iterator<Item = EpubManifestEntry<'ebook>> + 'ebook {
        self.scripts()
    }

    fn styles(&self) -> impl Iterator<Item = EpubManifestEntry<'ebook>> + 'ebook {
        self.styles()
    }

    fn fonts(&self) -> impl Iterator<Item = EpubManifestEntry<'ebook>> + 'ebook {
        self.fonts()
    }

    fn audio(&self) -> impl Iterator<Item = EpubManifestEntry<'ebook>> + 'ebook {
        self.audio()
    }

    fn video(&self) -> impl Iterator<Item = EpubManifestEntry<'ebook>> + 'ebook {
        self.video()
    }

    fn readable_content(&self) -> impl Iterator<Item = EpubManifestEntry<'ebook>> + 'ebook {
        self.readable_content()
    }

    fn by_kind(
        &self,
        kind: impl Many<ResourceKind<'ebook>>,
    ) -> impl Iterator<Item = EpubManifestEntry<'ebook>> + 'ebook {
        self.by_kind(kind)
    }
}

impl Debug for EpubManifest<'_> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("EpubManifest")
            .field("data", self.manifest)
            .finish_non_exhaustive()
    }
}

impl PartialEq for EpubManifest<'_> {
    fn eq(&self, other: &Self) -> bool {
        self.manifest == other.manifest
    }
}

impl<'ebook> IntoIterator for &EpubManifest<'ebook> {
    type Item = EpubManifestEntry<'ebook>;
    type IntoIter = EpubManifestIter<'ebook>;

    fn into_iter(self) -> EpubManifestIter<'ebook> {
        self.iter()
    }
}

impl<'ebook> IntoIterator for EpubManifest<'ebook> {
    type Item = EpubManifestEntry<'ebook>;
    type IntoIter = EpubManifestIter<'ebook>;

    fn into_iter(self) -> EpubManifestIter<'ebook> {
        self.iter()
    }
}

/// An iterator over all the [entries](EpubManifestEntry) contained within an [`EpubManifest`].
///
/// # See Also
/// - [`EpubManifest::iter`] to create an instance of this struct.
///
/// # Examples
/// - Iterating over all manifest entries:
/// ```
/// # use rbook::Epub;
/// # fn main() -> rbook::ebook::errors::EbookResult<()> {
/// let epub = Epub::open("tests/ebooks/example_epub")?;
///
/// for entry in epub.manifest() {
///     // process entry //
/// }
/// # Ok(())
/// # }
/// ```
pub struct EpubManifestIter<'ebook> {
    ctx: EpubManifestContext<'ebook>,
    iter: HashMapIter<'ebook, String, EpubManifestEntryData>,
}

impl<'ebook> Iterator for EpubManifestIter<'ebook> {
    type Item = EpubManifestEntry<'ebook>;

    fn next(&mut self) -> Option<Self::Item> {
        self.iter
            .next()
            .map(|(id, data)| self.ctx.create_entry(id, data))
    }
}

/// A [`ManifestEntry`] contained within an [`EpubManifest`], encompassing
/// resource-related metadata.
///
/// # See Also
/// - [`EpubManifestEntryMut`] for a mutable view.
#[derive(Copy, Clone)]
pub struct EpubManifestEntry<'ebook> {
    ctx: EpubManifestContext<'ebook>,
    id: &'ebook str,
    data: &'ebook EpubManifestEntryData,
}

impl<'ebook> EpubManifestEntry<'ebook> {
    /// The unique `id` of an entry within the [`EpubManifest`].
    pub fn id(&self) -> &'ebook str {
        self.id
    }

    /// The resolved absolute percent-encoded `href`,
    /// pointing to the location of the associated resource.
    ///
    /// Example of a resolved href:
    /// ```text
    /// /EPUB/OEBPS/chapters/c1.xhtml
    /// ```
    ///
    /// The href is resolved by calculating the location of [`Self::href_raw`]
    /// relative to [`EpubPackage::directory`](super::package::EpubPackage::directory).
    ///
    /// # Note
    /// - The resolved href is pre-calculated during parsing.
    /// - The href is corrected if [`EpubOpenOptions::strict`](super::EpubOpenOptions::strict)
    ///   is disabled.
    ///   For example, if the source EPUB contained unencoded characters (e.g., spaces),
    ///   they are automatically encoded.
    ///
    /// # See Also
    /// - [`Self::resource`] as the primary means for retrieving ebook content.
    pub fn href(&self) -> Href<'ebook> {
        Href::new(&self.data.href)
    }

    /// The raw (relative) `href`,
    /// pointing to the location of the associated resource.
    ///
    /// Example of a raw (relative) href:
    /// ```text
    /// ../../../c1.xhtml
    /// ```
    ///
    /// # Percent-Encoding
    /// If [`EpubOpenOptions::strict`](super::EpubOpenOptions::strict) is disabled
    /// and the EPUB is malformed (e.g., unencoded hrefs),
    /// the returned [`Href`] will reflect that raw state.
    ///
    /// # Note
    /// [`Self::href`] is recommended over this method.
    /// Providing the raw href to a method such as
    /// [`Ebook::read_resource_bytes`](crate::Ebook::read_resource_bytes) **may fail**.
    ///
    /// # See Also
    /// - [`Epub`](super::Epub) documentation of `copy_resource` for normalization details.
    pub fn href_raw(&self) -> Href<'ebook> {
        Href::new(&self.data.href_raw)
    }

    /// The **non-capitalized** `MIME` identifying the media type of
    /// the resource referenced by [`Self::href`].
    ///
    /// This method is a lower-level call than [`Self::kind`].
    pub fn media_type(&self) -> &'ebook str {
        &self.data.media_type
    }

    /// The SMIL media overlay of an entry providing pre-recorded narration
    /// for the associated content.
    /// Returns [`None`] if there is no media overlay available.
    pub fn media_overlay(&self) -> Option<Self> {
        self.data
            .media_overlay
            .as_deref()
            .and_then(|media_overlay| self.ctx.by_id(media_overlay))
    }

    /// The fallback of an entry when an application does not support or cannot render
    /// the associated content.
    /// Returns [`None`] if there is no fallback available.
    ///
    /// # Note
    /// **This method does *not* protect against cycles in malformed EPUBs,
    /// [`Self::fallbacks`] provides protection and is recommended when chaining.**
    ///
    /// # Examples
    /// - Falling back on a potentially incompatible image format:
    /// ```
    /// # use rbook::Epub;
    /// # fn main() -> rbook::ebook::errors::EbookResult<()> {
    /// let epub = Epub::open("tests/ebooks/example_epub")?;
    ///
    /// let webm_cover = epub.manifest().cover_image().unwrap();
    /// assert_eq!("image/webm", webm_cover.media_type());
    ///
    /// // If the app does not support `webm`; fallback
    /// let avif_cover = webm_cover.fallback().unwrap();
    /// assert_eq!("image/avif", avif_cover.media_type());
    /// # Ok(())
    /// # }
    /// ```
    pub fn fallback(&self) -> Option<Self> {
        self.data
            .fallback
            .as_deref()
            .and_then(|fallback| self.ctx.by_id(fallback))
            // Disallow self-references
            .filter(|entry| !std::ptr::eq(self.data, entry.data))
    }

    /// Returns an iterator over **all** fallback manifest entries,
    /// stopping when there are no available fallbacks or if there’s a cycle.
    ///
    /// Fallback entries are useful for applications that do not support or cannot render
    /// the content of a manifest entry, allowing to "fallback" to an entry that is
    /// eventually compatible.
    ///
    /// # Examples
    /// - Fallback on potentially incompatible image formats:
    /// ```
    /// # use rbook::Epub;
    /// # fn main() -> rbook::ebook::errors::EbookResult<()> {
    /// let epub = Epub::open("tests/ebooks/example_epub")?;
    ///
    /// let webm_cover = epub.manifest().cover_image().unwrap();
    /// assert_eq!("image/webm", webm_cover.media_type());
    ///
    /// // If `webm` is unsupported, fallback
    /// let mut fallbacks = webm_cover.fallbacks();
    /// let avif_cover = fallbacks.next().unwrap();
    /// assert_eq!("image/avif", avif_cover.media_type());
    ///
    /// // If `avif` is unsupported, fallback
    /// let png_cover = fallbacks.next().unwrap();
    /// assert_eq!("image/png", png_cover.media_type());
    ///
    /// // No fallbacks remaining
    /// assert_eq!(None, fallbacks.next());
    /// # Ok(())
    /// # }
    /// ```
    pub fn fallbacks(&self) -> impl Iterator<Item = Self> + 'ebook {
        let mut cycle = HashSet::new();
        cycle.insert(std::ptr::from_ref(self.data));

        std::iter::successors(self.fallback(), move |entry| {
            entry.fallback().filter(|entry| cycle.insert(entry.data))
        })
    }

    /// The [`Properties`] associated with a manifest entry.
    ///
    /// While not limited to, potential contained property values are:
    /// - `cover-image`
    /// - `mathml`
    /// - `nav`
    /// - `remote-resources`
    /// - `scripted`
    /// - `svg`
    /// - `switch`
    ///
    /// See the specification for more details regarding properties:
    /// <https://www.w3.org/TR/epub/#app-item-properties-vocab>
    pub fn properties(&self) -> &'ebook Properties {
        &self.data.properties
    }

    /// All additional XML [`Attributes`].
    ///
    /// # Omitted Attributes
    /// The following attributes will **not** be found within the returned collection:
    /// - [`id`](Self::id)
    /// - [`href`](Self::href)
    /// - [`media-type`](Self::media_type)
    /// - [`media-overlay`](Self::media_overlay)
    /// - [`fallback`](Self::fallbacks)
    /// - [`properties`](Self::properties)
    pub fn attributes(&self) -> &'ebook Attributes {
        &self.data.attributes
    }

    /// Complementary refinement metadata entries.
    pub fn refinements(&self) -> EpubRefinements<'ebook> {
        self.ctx
            .meta_ctx
            .create_refinements(Some(self.id), &self.data.refinements)
    }

    /// The underlying [`Resource`] a manifest entry points to.
    #[doc = util::inherent_doc!(ManifestEntry, resource)]
    pub fn resource(&self) -> Resource<'ebook> {
        Resource::new(self.media_type(), self.data.href.as_str())
    }

    /// The [`ResourceKind`] a manifest entry represents,
    /// such as `XHTML`, `PNG`, `CSS`, etc.
    #[doc = util::inherent_doc!(ManifestEntry, kind)]
    pub fn kind(&self) -> ResourceKind<'ebook> {
        self.media_type().into()
    }

    /// Copies the associated content into the given `writer`,
    /// returning the total number of bytes written on success.
    #[doc = util::inherent_doc!(ManifestEntry, copy_bytes)]
    pub fn copy_bytes(&self, writer: &mut impl Write) -> ArchiveResult<u64> {
        self.ctx.resource.copy_bytes(&self.resource(), writer)
    }

    /// Returns the associated content as a [`String`].
    #[doc = util::inherent_doc!(ManifestEntry, read_str)]
    pub fn read_str(&self) -> ArchiveResult<String> {
        ManifestEntry::read_str(self)
    }

    /// Returns the associated content as bytes.
    #[doc = util::inherent_doc!(ManifestEntry, read_bytes)]
    pub fn read_bytes(&self) -> ArchiveResult<Vec<u8>> {
        ManifestEntry::read_bytes(self)
    }
}

impl Sealed for EpubManifestEntry<'_> {}

impl<'ebook> ManifestEntry<'ebook> for EpubManifestEntry<'ebook> {
    fn resource(&self) -> Resource<'ebook> {
        self.resource()
    }

    fn kind(&self) -> ResourceKind<'ebook> {
        self.kind()
    }

    fn copy_bytes(&self, writer: &mut impl Write) -> ArchiveResult<u64> {
        self.copy_bytes(writer)
    }
}

impl<'ebook> From<EpubManifestEntry<'ebook>> for Resource<'ebook> {
    fn from(entry: EpubManifestEntry<'ebook>) -> Self {
        entry.resource()
    }
}

impl Debug for EpubManifestEntry<'_> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("EpubManifestEntry")
            .field("id", &self.id)
            .field("data", self.data)
            .finish_non_exhaustive()
    }
}

impl PartialEq for EpubManifestEntry<'_> {
    fn eq(&self, other: &Self) -> bool {
        self.id == other.id && self.data == other.data
    }
}
