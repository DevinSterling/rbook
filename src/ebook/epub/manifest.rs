//! EPUB manifest-related content.

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
const READABLE_CONTENT: [&str; 2] = ["application/xhtml+xml", "text/html"];

#[derive(Debug, PartialEq)]
pub(super) struct EpubManifestData {
    entries: HashMap<String, EpubManifestEntryData>,
}

impl EpubManifestData {
    pub(super) fn new(entries: HashMap<String, EpubManifestEntryData>) -> Self {
        Self { entries }
    }

    pub(super) fn by_id_mut(&mut self, id: &str) -> Option<&mut EpubManifestEntryData> {
        self.entries.get_mut(id)
    }

    pub(super) fn iter(&self) -> HashMapIter<String, EpubManifestEntryData> {
        self.entries.iter()
    }
}

#[derive(Debug, PartialEq)]
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
pub(super) struct EpubManifestEntryProvider<'ebook>(EpubManifest<'ebook>);

impl<'ebook> EpubManifestEntryProvider<'ebook> {
    pub(super) fn by_id(&self, id: &str) -> Option<EpubManifestEntry<'ebook>> {
        self.0.by_id(id)
    }

    pub(super) fn by_href(&self, href: &str) -> Option<EpubManifestEntry<'ebook>> {
        self.0.by_href(href)
    }
}

impl<'ebook> From<EpubManifest<'ebook>> for EpubManifestEntryProvider<'ebook> {
    fn from(manifest: EpubManifest<'ebook>) -> Self {
        Self(manifest)
    }
}

/// The context of an [`EpubManifestEntry`] for fallback lookup and raw resource retrieval.
#[derive(Copy, Clone)]
struct EpubManifestEntryContext<'ebook> {
    entry: EpubManifestEntryProvider<'ebook>,
    resource: EpubResourceProvider<'ebook>,
}

impl<'ebook> From<&EpubManifest<'ebook>> for EpubManifestEntryContext<'ebook> {
    fn from(manifest: &EpubManifest<'ebook>) -> Self {
        Self {
            entry: (*manifest).into(),
            resource: manifest.resource_provider,
        }
    }
}

////////////////////////////////////////////////////////////////////////////////
// PUBLIC API
////////////////////////////////////////////////////////////////////////////////

/// An EPUB manifest, see [`Manifest`] for more details.
///
/// Retrieving entries from the manifest from such as [`EpubManifest::by_href`]
/// is generally a linear (`O(N)`) operation except for [`EpubManifest::by_id`],
/// which is constant (`O(1)`).
#[derive(Copy, Clone)]
pub struct EpubManifest<'ebook> {
    data: &'ebook EpubManifestData,
    resource_provider: EpubResourceProvider<'ebook>,
}

impl<'ebook> EpubManifest<'ebook> {
    pub(super) fn new(
        data: &'ebook EpubManifestData,
        resource_provider: EpubResourceProvider<'ebook>,
    ) -> Self {
        Self {
            data,
            resource_provider,
        }
    }

    fn by_predicate(
        &self,
        predicate: impl Fn(&EpubManifestEntryData) -> bool,
    ) -> Option<EpubManifestEntry<'ebook>> {
        self.data
            .iter()
            .find(|(_, data)| predicate(data))
            .map(|(id, data)| EpubManifestEntry::new(id, data, self.into()))
    }

    /// Returns the [`EpubManifestEntry`] that matches the given `id` if present,
    /// otherwise [`None`].
    ///
    /// This is a constant (`O(1)`) operation.
    pub fn by_id(&self, id: &str) -> Option<EpubManifestEntry<'ebook>> {
        self.data
            .entries
            .get_key_value(id)
            .map(|(id, data)| EpubManifestEntry::new(id, data, self.into()))
    }

    /// Returns the [`EpubManifestEntry`] that matches the given `href` if present,
    /// otherwise [`None`].
    ///
    /// # Note
    /// The given `href` is ***not*** normalized or percent-decoded.
    /// It is compared literally against both [`EpubManifestEntry::href()`] and
    /// [`EpubManifestEntry::href_raw()`].
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
        let context = self.into();

        self.data
            .iter()
            .filter(move |(_, data)| data.properties.has_property(property))
            .map(move |(id, data)| EpubManifestEntry::new(id, data, context))
    }
}

#[allow(refining_impl_trait)]
impl<'ebook> Manifest<'ebook> for EpubManifest<'ebook> {
    fn len(&self) -> usize {
        self.data.entries.len()
    }

    /// Returns an iterator over **all** [`entries`](ManifestEntry) in the manifest.
    ///
    /// # Order
    /// As manifest entries are stored in a hash map with `id` as the key,
    /// iteration order is arbitrary; non-deterministic.
    fn entries(&self) -> EpubManifestIter<'ebook> {
        self.into_iter()
    }

    fn cover_image(&self) -> Option<EpubManifestEntry<'ebook>> {
        self.by_property(consts::COVER_IMAGE).next()
    }

    fn images(&self) -> impl Iterator<Item = EpubManifestEntry<'ebook>> + 'ebook {
        self.by_resource_kind(ResourceKind::IMAGE)
    }

    fn readable_content(&self) -> impl Iterator<Item = EpubManifestEntry<'ebook>> + 'ebook {
        self.by_resource_kinds(READABLE_CONTENT)
    }

    fn by_resource_kind(
        &self,
        kind: impl Into<ResourceKind<'ebook>>,
    ) -> impl Iterator<Item = EpubManifestEntry<'ebook>> + 'ebook {
        self.by_resource_kinds([kind])
    }

    fn by_resource_kinds(
        &self,
        into_kinds: impl IntoIterator<Item = impl Into<ResourceKind<'ebook>>>,
    ) -> impl Iterator<Item = EpubManifestEntry<'ebook>> + 'ebook {
        let kinds = into_kinds.into_iter().map(Into::into).collect::<Vec<_>>();
        let context = self.into();

        self.data
            .iter()
            .filter(move |(_, data)| {
                kinds.iter().any(|kind| {
                    if kind.is_unspecified() {
                        // If the kind is unspecified, get the maintype as a "catch-all"
                        data.media_type.starts_with(kind.maintype())
                    } else {
                        data.media_type == kind.as_str()
                    }
                })
            })
            .map(move |(id, data)| EpubManifestEntry::new(id, data, context))
    }
}

impl Debug for EpubManifest<'_> {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("EpubManifest")
            .field("data", self.data)
            .finish_non_exhaustive()
    }
}

impl PartialEq for EpubManifest<'_> {
    fn eq(&self, other: &Self) -> bool {
        self.data == other.data
    }
}

impl<'ebook> IntoIterator for &EpubManifest<'ebook> {
    type Item = EpubManifestEntry<'ebook>;
    type IntoIter = EpubManifestIter<'ebook>;

    fn into_iter(self) -> EpubManifestIter<'ebook> {
        EpubManifestIter {
            context: self.into(),
            iter: self.data.iter(),
        }
    }
}

impl<'ebook> IntoIterator for EpubManifest<'ebook> {
    type Item = EpubManifestEntry<'ebook>;
    type IntoIter = EpubManifestIter<'ebook>;

    fn into_iter(self) -> EpubManifestIter<'ebook> {
        (&self).into_iter()
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
    context: EpubManifestEntryContext<'ebook>,
    iter: HashMapIter<'ebook, String, EpubManifestEntryData>,
}

impl<'ebook> Iterator for EpubManifestIter<'ebook> {
    type Item = EpubManifestEntry<'ebook>;

    fn next(&mut self) -> Option<Self::Item> {
        self.iter
            .next()
            .map(|(id, data)| EpubManifestEntry::new(id, data, self.context))
    }
}

/// A [`ManifestEntry`] contained within an [`EpubManifest`], encompassing
/// resource-related metadata.
#[derive(Copy, Clone)]
pub struct EpubManifestEntry<'ebook> {
    id: &'ebook str,
    data: &'ebook EpubManifestEntryData,
    context: EpubManifestEntryContext<'ebook>,
}

impl<'ebook> EpubManifestEntry<'ebook> {
    fn new(
        id: &'ebook str,
        data: &'ebook EpubManifestEntryData,
        context: EpubManifestEntryContext<'ebook>,
    ) -> Self {
        Self { id, data, context }
    }

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
    /// # See Also
    /// - [`Self::resource`] as the primary means for retrieving ebook content.
    pub fn href(&self) -> Href<'ebook> {
        self.data.href.as_str().into()
    }

    /// The raw (relative) percent-encoded `href`,
    /// pointing to the location of the associated resource.
    ///
    /// Example of a raw (relative) href:
    /// ```text
    /// ../../../c1.xhtml
    /// ```
    ///
    /// # Note
    /// [`Self::href`] is recommended over this method unless access to the original
    /// raw `href` is required for analysis.
    /// Providing the raw value to a method such as
    /// [`Ebook::read_resource_bytes`](crate::Ebook::read_resource_bytes) can fail.
    ///
    /// # See Also
    /// - [`Epub`](super::Epub) documentation of `read_resource_bytes` for normalization details.
    pub fn href_raw(&self) -> Href<'ebook> {
        self.data.href_raw.as_str().into()
    }

    /// The **non-capitalized** `MIME` of this [`EpubManifestEntry`].
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
            .and_then(|media_overlay| self.context.entry.by_id(media_overlay))
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
            .and_then(|fallback| self.context.entry.by_id(fallback))
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
    /// // If the app does not support `webm`; fallback
    /// let mut fallbacks = webm_cover.fallbacks();
    /// let avif_cover = fallbacks.next().unwrap();
    /// assert_eq!("image/avif", avif_cover.media_type());
    ///
    /// // If the app does not support `avif`; fallback
    /// let png_cover = fallbacks.next().unwrap();
    /// assert_eq!("image/png", png_cover.media_type());
    ///
    /// // No more fallbacks
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

    /// All additional `XML` [`Attributes`].
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
        self.context.resource.read_str(self.href().decode().into())
    }

    fn read_bytes(&self) -> EbookResult<Vec<u8>> {
        self.context
            .resource
            .read_bytes(self.href().decode().into())
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
