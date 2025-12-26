//! EPUB-specific manifest content.

use crate::ebook::element::{AttributeData, Attributes, Href, Properties, PropertiesData};
use crate::ebook::epub::consts;
use crate::ebook::epub::metadata::{EpubRefinements, EpubRefinementsData};
use crate::ebook::errors::EbookResult;
use crate::ebook::manifest::{Manifest, ManifestEntry};
use crate::ebook::resource::{Resource, ResourceKind};
use crate::epub::EpubResourceProvider;
use std::collections::hash_map::Iter as HashMapIter;
use std::collections::{HashMap, HashSet};
use std::fmt::{Debug, Formatter};

////////////////////////////////////////////////////////////////////////////////
// PRIVATE API
////////////////////////////////////////////////////////////////////////////////

/// The kinds of readable content for an epub intended for end-user reading,
/// typically `application/xhtml+xml`.
/// `text/html` is possible as well, although not as common.
const READABLE_CONTENT_MIME: [&str; 2] = ["application/xhtml+xml", "text/html"];
const SCRIPTS_MIME: [&str; 3] = [
    "application/javascript",
    "application/ecmascript",
    "text/javascript",
];
const CSS_MIME: &str = "text/css";

#[derive(Debug, PartialEq)]
pub(super) struct EpubManifestData {
    entries: HashMap<String, EpubManifestEntryData>,
}

impl EpubManifestData {
    pub(super) fn new(entries: HashMap<String, EpubManifestEntryData>) -> Self {
        Self { entries }
    }

    pub(super) fn empty() -> Self {
        Self {
            entries: HashMap::new(),
        }
    }

    pub(super) fn by_id_mut(&mut self, id: &str) -> Option<&mut EpubManifestEntryData> {
        self.entries.get_mut(id)
    }

    pub(super) fn iter(&self) -> HashMapIter<'_, String, EpubManifestEntryData> {
        self.entries.iter()
    }
}

#[derive(Clone, Debug, Default, PartialEq)]
pub(super) struct EpubManifestEntryData {
    /// The resolved `absolute` href
    pub(super) href: String,
    /// The source `relative` href
    pub(super) href_raw: String,
    pub(super) media_type: String,
    pub(super) fallback: Option<String>,
    pub(super) media_overlay: Option<String>,
    pub(super) properties: PropertiesData,
    pub(super) attributes: Vec<AttributeData>,
    pub(super) refinements: EpubRefinementsData,
}

/// Provider to retrieve [`EpubManifestEntry`] instances.
#[derive(Copy, Clone)]
pub(super) struct EpubManifestEntryProvider<'ebook>(EpubManifestContext<'ebook>);

impl<'ebook> EpubManifestEntryProvider<'ebook> {
    pub(super) fn by_id(&self, id: &str) -> Option<EpubManifestEntry<'ebook>> {
        self.0.lookup_entry_by_id(id)
    }

    pub(super) fn by_href(&self, href: &str) -> Option<EpubManifestEntry<'ebook>> {
        self.0.lookup_entry_by_href(href)
    }
}

impl<'ebook> From<EpubManifest<'ebook>> for EpubManifestEntryProvider<'ebook> {
    fn from(manifest: EpubManifest<'ebook>) -> Self {
        Self(manifest.ctx)
    }
}

impl<'ebook> From<EpubManifestContext<'ebook>> for EpubManifestEntryProvider<'ebook> {
    fn from(ctx: EpubManifestContext<'ebook>) -> Self {
        Self(ctx)
    }
}

/// The context of an [`EpubManifestEntry`] for fallback lookup and raw resource retrieval.
#[derive(Copy, Clone)]
struct EpubManifestContext<'ebook> {
    resource: EpubResourceProvider<'ebook>,
    data: &'ebook EpubManifestData,
}

impl<'ebook> EpubManifestContext<'ebook> {
    fn new(resource: EpubResourceProvider<'ebook>, data: &'ebook EpubManifestData) -> Self {
        Self { resource, data }
    }

    fn create(self) -> EpubManifest<'ebook> {
        EpubManifest { ctx: self }
    }

    fn create_entry(
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

    fn lookup_entry_by_id(&self, id: &str) -> Option<EpubManifestEntry<'ebook>> {
        self.create().by_id(id)
    }

    fn lookup_entry_by_href(&self, href: &str) -> Option<EpubManifestEntry<'ebook>> {
        self.create().by_href(href)
    }
}

////////////////////////////////////////////////////////////////////////////////
// PUBLIC API
////////////////////////////////////////////////////////////////////////////////

/// An EPUB manifest, see [`Manifest`] for more details.
///
/// Retrieving entries from the manifest using methods such as [`EpubManifest::by_href`]
/// is generally a linear (`O(N)`) operation except for [`EpubManifest::by_id`],
/// which is constant (`O(1)`).
///
/// # Ordering
/// Methods that return iterators yield entries in unspecified order.
/// This is because manifest entries are stored in a hash map with `id` as the key.
#[derive(Copy, Clone)]
pub struct EpubManifest<'ebook> {
    ctx: EpubManifestContext<'ebook>,
}

impl<'ebook> EpubManifest<'ebook> {
    pub(super) fn new(
        provider: EpubResourceProvider<'ebook>,
        data: &'ebook EpubManifestData,
    ) -> Self {
        Self {
            ctx: EpubManifestContext::new(provider, data),
        }
    }

    fn by_predicate(
        &self,
        predicate: impl Fn(&EpubManifestEntryData) -> bool,
    ) -> Option<EpubManifestEntry<'ebook>> {
        self.ctx
            .data
            .iter()
            .find(|(_, data)| predicate(data))
            .map(|(id, data)| self.ctx.create_entry(id, data))
    }

    /// Returns the [`EpubManifestEntry`] that matches the given `id` if present,
    /// otherwise [`None`].
    ///
    /// This is a constant (`O(1)`) operation.
    pub fn by_id(&self, id: &str) -> Option<EpubManifestEntry<'ebook>> {
        self.ctx
            .data
            .entries
            .get_key_value(id)
            .map(|(id, data)| self.ctx.create_entry(id, data))
    }

    /// Returns the [`EpubManifestEntry`] that matches the given `href` if present,
    /// otherwise [`None`].
    ///
    /// # Note
    /// The given `href` is ***not*** normalized or percent-decoded.
    /// It is compared **case-sensitively** against both [`EpubManifestEntry::href()`] and
    /// [`EpubManifestEntry::href_raw()`].
    ///
    /// [`Self::by_id`] is recommended over this method,
    /// as this method performs a linear `O(N)` search.
    pub fn by_href(&self, href: &str) -> Option<EpubManifestEntry<'ebook>> {
        self.by_predicate(|data| data.href == href || data.href_raw == href)
    }

    /// Returns an iterator over all [`entries`](EpubManifestEntry) in the
    /// [`manifest`](EpubManifest) whose [`properties`](EpubManifestEntry::properties)
    /// contains the specified `property`.
    pub fn by_property(
        &self,
        property: &'ebook str,
    ) -> impl Iterator<Item = EpubManifestEntry<'ebook>> + 'ebook {
        let ctx = self.ctx;

        self.ctx
            .data
            .iter()
            .filter(|(_, data)| data.properties.has_property(property))
            .map(move |(id, data)| ctx.create_entry(id, data))
    }

    /// Returns an iterator over JavaScript [`entries`](ManifestEntry) in the manifest.
    ///
    /// The iterated entries may correspond to different script kinds, specifically
    /// the EPUB-spec's core media types for scripts:
    /// - `application/javascript`
    /// - `application/ecmascript`
    /// - `text/javascript`
    ///
    /// # See Also
    /// - [`ManifestEntry::resource_kind`] to inspect the exact script kind.
    pub fn scripts(&self) -> impl Iterator<Item = EpubManifestEntry<'ebook>> {
        self.by_resource_kinds(SCRIPTS_MIME)
    }

    /// Returns an iterator over CSS stylesheet [`entries`](ManifestEntry) in the manifest.
    ///
    /// All iterated entries will have a media type of `text/css`.
    ///
    /// # See Also
    /// - [`ManifestEntry::resource_kind`] to inspect the exact resource kind.
    pub fn styles(&self) -> impl Iterator<Item = EpubManifestEntry<'ebook>> + 'ebook {
        self.by_resource_kind(CSS_MIME)
    }

    /// Returns an iterator over all font [`entries`](ManifestEntry) in the manifest,
    /// including EPUB-compatible legacy font MIMEs (e.g., `application/font-woff`).
    ///
    /// The iterated entries may correspond to different font kinds,
    /// such as `TTF`, `WOFF`, etc.
    ///
    /// # Note
    /// This method behaves differently compared to invoking
    /// [`Self::by_resource_kind`] with the argument as
    /// [`ResourceKind::FONT`], which checks if MIMEs match the pattern `font/*`.
    ///
    /// # See Also
    /// - [`ManifestEntry::resource_kind`] to inspect the exact font kind.
    pub fn fonts(&self) -> impl Iterator<Item = EpubManifestEntry<'ebook>> + 'ebook {
        self.entries()
            // Filtering is preferred over `by_resource_kind(ResourceKind::FONT)`
            // as that method retrieves all entries whose MIME match `font/*`.
            // > older EPUB-compatible font types start with `application/*`
            .filter(|entry| entry.resource_kind().is_font())
    }

    /// Returns an iterator over all audio [`entries`](ManifestEntry) in the manifest.
    ///
    /// The iterated entries may correspond to different audio kinds,
    /// such as `MP3`, `AAC`, etc.
    ///
    /// This method provides the same functionality as invoking
    /// [`Self::by_resource_kind`] with the argument as
    /// [`ResourceKind::AUDIO`].
    ///
    /// # See Also
    /// - [`ManifestEntry::resource_kind`] to inspect the exact audio kind.
    pub fn audio(&self) -> impl Iterator<Item = EpubManifestEntry<'ebook>> + 'ebook {
        self.by_resource_kind(ResourceKind::AUDIO)
    }

    /// Returns an iterator over all video [`entries`](ManifestEntry) in the manifest.
    ///
    /// The iterated entries may correspond to different video kinds,
    /// such as `MP4`, `WEBM`, etc.
    ///
    /// This method provides the same functionality as invoking
    /// [`Self::by_resource_kind`] with the argument as
    /// [`ResourceKind::VIDEO`].
    ///
    /// # See Also
    /// - [`ManifestEntry::resource_kind`] to inspect the exact video kind.
    pub fn video(&self) -> impl Iterator<Item = EpubManifestEntry<'ebook>> + 'ebook {
        self.by_resource_kind(ResourceKind::VIDEO)
    }
}

#[allow(refining_impl_trait)]
impl<'ebook> Manifest<'ebook> for EpubManifest<'ebook> {
    fn len(&self) -> usize {
        self.ctx.data.entries.len()
    }

    /// Returns an iterator over **all** [`entries`](ManifestEntry) in the manifest.
    ///
    /// # Ordering
    /// Iteration order is unspecified; see [`EpubManifest`] for ordering details.
    fn entries(&self) -> EpubManifestIter<'ebook> {
        EpubManifestIter {
            ctx: self.ctx,
            iter: self.ctx.data.iter(),
        }
    }

    fn cover_image(&self) -> Option<EpubManifestEntry<'ebook>> {
        self.by_property(consts::COVER_IMAGE).next()
    }

    fn images(&self) -> impl Iterator<Item = EpubManifestEntry<'ebook>> + 'ebook {
        self.by_resource_kind(ResourceKind::IMAGE)
    }

    fn readable_content(&self) -> impl Iterator<Item = EpubManifestEntry<'ebook>> + 'ebook {
        self.by_resource_kinds(READABLE_CONTENT_MIME)
    }

    fn by_resource_kind(
        &self,
        kind: impl Into<ResourceKind<'ebook>>,
    ) -> impl Iterator<Item = EpubManifestEntry<'ebook>> + 'ebook {
        self.by_resource_kinds(std::iter::once(kind))
    }

    fn by_resource_kinds(
        &self,
        into_kinds: impl IntoIterator<Item = impl Into<ResourceKind<'ebook>>>,
    ) -> impl Iterator<Item = EpubManifestEntry<'ebook>> + 'ebook {
        let targets = into_kinds.into_iter().map(Into::into).collect::<Vec<_>>();
        let ctx = self.ctx;

        self.ctx
            .data
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

impl Debug for EpubManifest<'_> {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("EpubManifest")
            .field("data", self.ctx.data)
            .finish_non_exhaustive()
    }
}

impl PartialEq for EpubManifest<'_> {
    fn eq(&self, other: &Self) -> bool {
        self.ctx.data == other.ctx.data
    }
}

impl<'ebook> IntoIterator for &EpubManifest<'ebook> {
    type Item = EpubManifestEntry<'ebook>;
    type IntoIter = EpubManifestIter<'ebook>;

    fn into_iter(self) -> EpubManifestIter<'ebook> {
        self.entries()
    }
}

impl<'ebook> IntoIterator for EpubManifest<'ebook> {
    type Item = EpubManifestEntry<'ebook>;
    type IntoIter = EpubManifestIter<'ebook>;

    fn into_iter(self) -> EpubManifestIter<'ebook> {
        self.entries()
    }
}

/// An iterator over all the [`entries`](EpubManifestEntry) of an [`EpubManifest`].
///
/// # See Also
/// - [`EpubManifest::entries`]
///
/// # Examples
/// - Iterating over all manifest entries:
/// ```
/// # use rbook::ebook::errors::EbookResult;
/// # use rbook::{Ebook, Epub};
/// # fn main() -> EbookResult<()> {
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
    /// relative to [`Epub::package_directory`](super::Epub::package_directory).
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
        self.data.href.as_str().into()
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
    /// - [`Epub`](super::Epub) documentation of `read_resource_bytes` for normalization details.
    pub fn href_raw(&self) -> Href<'ebook> {
        self.data.href_raw.as_str().into()
    }

    /// The **non-capitalized** `MIME` identifying the media type of
    /// the resource referenced by [`Self::href`].
    ///
    /// This method is a lower-level call than [`Self::resource_kind`].
    pub fn media_type(&self) -> &'ebook str {
        &self.data.media_type
    }

    /// The media overlay of an entry providing pre-recorded narration
    /// for the associated content.
    /// Returns [`None`] if there is no media overlay available.
    pub fn media_overlay(&self) -> Option<Self> {
        self.data
            .media_overlay
            .as_deref()
            .and_then(|media_overlay| self.ctx.lookup_entry_by_id(media_overlay))
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
    /// # use rbook::ebook::errors::EbookResult;
    /// # use rbook::ebook::manifest::Manifest;
    /// # use rbook::{Ebook, Epub};
    /// # fn main() -> EbookResult<()> {
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
            .and_then(|fallback| self.ctx.lookup_entry_by_id(fallback))
            // Disallow self-references
            .filter(|entry| !std::ptr::eq(self.data, entry.data))
    }

    /// Returns an iterator over **all** fallback manifest entries, stopping if there’s a cycle.
    ///
    /// Fallback entries are useful for applications that do not support or cannot render
    /// the content of a manifest entry, allowing to "fallback" to an entry that is
    /// eventually compatible.
    ///
    /// # Examples
    /// - Fallback on potentially incompatible image formats:
    /// ```
    /// # use rbook::ebook::errors::EbookResult;
    /// # use rbook::ebook::manifest::Manifest;
    /// # use rbook::{Ebook, Epub};
    /// # fn main() -> EbookResult<()> {
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
    pub fn properties(&self) -> Properties<'ebook> {
        (&self.data.properties).into()
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
    pub fn attributes(&self) -> Attributes<'ebook> {
        (&self.data.attributes).into()
    }

    /// Complementary refinement metadata entries.
    pub fn refinements(&self) -> EpubRefinements<'ebook> {
        (&self.data.refinements).into()
    }
}

impl<'ebook> ManifestEntry<'ebook> for EpubManifestEntry<'ebook> {
    fn key(&self) -> Option<&'ebook str> {
        Some(self.id())
    }

    fn resource(&self) -> Resource<'ebook> {
        Resource::new(self.media_type(), self.data.href.as_str())
    }

    fn resource_kind(&self) -> ResourceKind<'ebook> {
        self.media_type().into()
    }

    fn read_str(&self) -> EbookResult<String> {
        self.ctx.resource.read_str(self.href().decode().into())
    }

    fn read_bytes(&self) -> EbookResult<Vec<u8>> {
        self.ctx.resource.read_bytes(self.href().decode().into())
    }
}

impl<'ebook> From<EpubManifestEntry<'ebook>> for Resource<'ebook> {
    fn from(entry: EpubManifestEntry<'ebook>) -> Self {
        entry.resource()
    }
}

impl Debug for EpubManifestEntry<'_> {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
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
