use crate::ebook::archive::ResourceProvider;
use crate::ebook::element::{Attribute, Attributes, Properties};
use crate::ebook::epub::archive::EpubArchive;
use crate::ebook::epub::consts::opf;
use crate::ebook::epub::errors::EpubError;
use crate::ebook::epub::manifest::{
    EpubManifest, EpubManifestContext, EpubManifestData, EpubManifestEntry, EpubManifestEntryData,
};
use crate::ebook::epub::metadata::{DetachedEpubMetaEntry, EpubMetadataData, EpubRefinementsMut};
use crate::ebook::epub::package::EpubPackageMetaContext;
use crate::ebook::epub::spine::EpubSpineData;
use crate::ebook::epub::toc::EpubTocData;
use crate::ebook::errors::ArchiveResult;
use crate::ebook::resource::{self, ResourceContent};
use crate::input::{IntoOption, Many};
use crate::util;
use crate::util::uri::{self, UriResolver};
use std::fmt::{Debug, Write};

////////////////////////////////////////////////////////////////////////////////
// PRIVATE API
////////////////////////////////////////////////////////////////////////////////

impl<'ebook> EpubManifestContext<'ebook> {
    fn attached(archive: &'ebook EpubArchive, package: EpubPackageMetaContext<'ebook>) -> Self {
        Self::new(ResourceProvider::Archive(archive), package, None)
    }

    fn detached(content: Option<&'ebook ResourceContent>) -> Self {
        Self::new(
            match content {
                Some(c) => ResourceProvider::Single(c),
                None => ResourceProvider::Empty,
            },
            EpubPackageMetaContext::EMPTY,
            None,
        )
    }
}

impl EpubManifestData {
    /// This method is **best-effort** and assumes no other element
    /// within `<metadata>`, `<spine>`, etc. has the same `id`.
    ///
    /// Realistically, any other element within the package *shouldn't*
    /// conflict with the specialized IDs passed to this method
    /// (e.g., `toc_xhtml`, `toc_ncx`, `chapter_1_xhtml`, etc.).
    pub(crate) fn generate_unique_id(&self, mut id: String) -> String {
        let mut count = 1;
        let original_len = id.len();

        // Avoid collisions
        while self.entries.contains_key(&id) {
            id.truncate(original_len);
            write!(&mut id, "-{count}").ok();
            count += 1;
        }
        id
    }

    pub(crate) fn generate_unique_href(&self, mut href: String) -> String {
        let mut count = 1;
        let mut count_len = 0;
        let ext = href.rfind('.').unwrap_or(href.len());

        // Avoid identical hrefs
        // NOTE: Practically, most hrefs given to this method are already unique
        while self.entries.iter().any(|(_, entry)| entry.href == href) {
            let count_str = count.to_string();

            href.replace_range(ext..ext + count_len, &count_str);
            count_len = count_str.len();
            count += 1;
        }
        href
    }

    pub(crate) fn remove_non_existent_references(&mut self) {
        let entries = &mut self.entries;

        for i in 0..entries.len() {
            let data = &entries[i];
            let invalid_fallback = data
                .fallback
                .as_deref()
                .is_some_and(|idref| !entries.contains_key(idref));
            let invalid_media_overlay = data
                .media_overlay
                .as_deref()
                .is_some_and(|idref| !entries.contains_key(idref));

            let data = &mut entries[i];
            if invalid_fallback {
                data.fallback = None;
            }
            if invalid_media_overlay {
                data.media_overlay = None;
            }
        }
    }
}

impl EpubManifestEntryData {
    fn set_href_raw(&mut self, resolver: UriResolver<'_>, href_raw: String) -> (String, String) {
        let href = std::mem::replace(&mut self.href, resolver.resolve(&href_raw));
        let href_raw = std::mem::replace(&mut self.href_raw, href_raw);

        (href, href_raw)
    }

    fn resolve_href(&mut self, resolver: UriResolver<'_>) {
        self.href = resolver.resolve(&self.href_raw);
    }
}

struct AttachedEntryContext<'ebook> {
    /// The index always references an existing entry within [`EpubManifestData`].
    index: usize,
    href_resolver: UriResolver<'ebook>,
    archive: &'ebook mut EpubArchive,
    manifest: &'ebook mut EpubManifestData,

    // References to other content (to synchronize id/href changes)
    metadata: &'ebook mut EpubMetadataData,
    spine: &'ebook mut EpubSpineData,
    toc: &'ebook mut EpubTocData,
}

impl AttachedEntryContext<'_> {
    fn update_entry_id(&mut self, options: IdOptions) -> Result<String, EpubError> {
        let new_id = options.id;

        // Silently replacing an entry is not allowed; return an error
        if self.manifest.entries.contains_key(&new_id) {
            return Err(EpubError::DuplicateItemId(new_id));
        }

        let (old_id, data) = self
            .manifest
            .entries
            .shift_remove_index(self.index)
            .expect(ManifestEntryDataHandle::ENTRY_EXPECTED);

        if options.cascade {
            self.cascade_new_id(&old_id, &new_id);
        }

        // Reinsert using the new id
        self.manifest.entries.shift_insert(self.index, new_id, data);
        Ok(old_id)
    }

    fn update_entry_href(&mut self, options: HrefOptions) -> String {
        let data = &mut self.manifest.entries[self.index];
        let (old, old_raw) = data.set_href_raw(self.href_resolver, options.href);
        // Temporarily take possession of the new href
        let new_href = std::mem::take(&mut data.href);

        if options.cascade {
            self.cascade_new_href(&old, &new_href);
        }
        self.archive.relocate(old, &new_href);

        // Return the temporarily taken href
        self.manifest.entries[self.index].href = new_href;
        old_raw
    }

    fn cascade_new_id(&mut self, old_id: &str, new_id: &str) {
        fn update_idref(reference: Option<&mut String>, old_id: &str, new_id: &str) {
            if let Some(reference) = reference
                && reference == old_id
            {
                *reference = new_id.to_owned();
            }
        }

        // Update fallback/media overlay
        for (_, entry) in &mut self.manifest.entries {
            update_idref(entry.fallback.as_mut(), old_id, new_id);
            update_idref(entry.media_overlay.as_mut(), old_id, new_id);
        }

        // Multiple `itemref` elements can reference a manifest `item`
        for entry in self.spine.entries.iter_mut() {
            update_idref(Some(&mut entry.idref), old_id, new_id);
        }

        // Metadata cover
        if let Some(cover) = self.metadata.entries.get_mut(opf::COVER)
            // There should only be a single entry
            && let Some(entry) = cover.first_mut()
            && entry.id.as_deref().is_some_and(|id| id == old_id)
        {
            entry.value = new_id.to_owned();
        }
    }

    fn cascade_new_href(&mut self, old_href: &str, new_href: &str) {
        for (_, root) in self.toc.entries.iter_mut() {
            root.cascade_toc_href(uri::path(old_href), new_href);
        }
    }
}

enum ManifestEntryDataHandle<'ebook> {
    Attached(AttachedEntryContext<'ebook>),
    Detached {
        id: &'ebook mut String,
        data: &'ebook mut EpubManifestEntryData,
        content: &'ebook mut Option<ResourceContent>,
    },
}

impl ManifestEntryDataHandle<'_> {
    /// For [`Self::Attached`], the referenced entry by `id` is **guaranteed** to exist.
    /// 1. Before any [`Self::Attached`] is created,
    ///    the referenced entry by `id` is checked if it exists.
    /// 2. [`Self`] holds an exclusive borrow (`&mut`) of [`EpubManifestData`].
    /// 3. No other code can remove the ID while this object exists,
    ///    except [`EpubManifestEntryMut::set_id`], which immediately re-inserts it.
    const ENTRY_EXPECTED: &'static str = "[rbook] Manifest entry ID missing from map. This indicates a bug in `try_set_id` or `insert` logic.";

    fn get_mut(&mut self) -> &mut EpubManifestEntryData {
        self.get_mut_with_id().1
    }

    fn get_with_id(&self) -> (&str, &EpubManifestEntryData) {
        match self {
            Self::Detached { id, data, .. } => (id, data),
            Self::Attached(ctx) => {
                let (key, data) = ctx
                    .manifest
                    .entries
                    .get_index(ctx.index)
                    .expect(Self::ENTRY_EXPECTED);

                (key, data)
            }
        }
    }

    fn get_mut_with_id(&mut self) -> (&str, &mut EpubManifestEntryData) {
        match self {
            Self::Detached { id, data, .. } => (id, data),
            Self::Attached(ctx) => {
                let (key, data) = ctx
                    .manifest
                    .entries
                    .get_index_mut(ctx.index)
                    .expect(Self::ENTRY_EXPECTED);

                (key, data)
            }
        }
    }
}

impl Debug for ManifestEntryDataHandle<'_> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let mut debug = f.debug_struct("ManifestEntryDataHandle");

        match self {
            ManifestEntryDataHandle::Attached(ctx) => debug
                .field("href_resolver", &ctx.href_resolver)
                .field("index", &ctx.index),
            ManifestEntryDataHandle::Detached { id, data, content } => debug
                .field("id", id)
                .field("data", data)
                .field("content", content),
        }
        .finish_non_exhaustive()
    }
}

////////////////////////////////////////////////////////////////////////////////
// PUBLIC API
////////////////////////////////////////////////////////////////////////////////

impl EpubManifestEntry<'_> {
    /// Creates an owned detached manifest entry by cloning entry metadata.
    ///
    /// This method only clones entry metadata
    /// (e.g., [`id`](EpubManifestEntry::id), [`href`](EpubManifestEntry::href)).
    /// To retrieve the associated binary content as well, see [`Self::to_detached_with_content`].
    ///
    /// # Note
    /// If the source manifest entry has an `id`, the detached entry will retain it.
    /// To avoid ID collisions if re-inserting into the same [`Epub`](crate::epub::Epub),
    /// consider changing the ID using
    /// [`DetachedEpubManifestEntry::id`] or [`EpubManifestEntryMut::set_id`].
    ///
    /// # See Also
    /// - [`EpubManifestMut`] or [`EpubEditor::resource`](crate::epub::EpubEditor::resource)
    ///   to insert detached entries into or remove entries without cloning.
    ///
    /// # Examples
    /// - Cloning all manifest entries:
    /// ```
    /// # use rbook::Epub;
    /// # fn main() -> rbook::ebook::errors::EbookResult<()> {
    /// let mut epub = Epub::open("tests/ebooks/example_epub")?;
    ///
    /// let manifest = epub.manifest();
    /// assert_eq!(12, manifest.len());
    ///
    /// // Cloning all entries
    /// let detached: Vec<_> = manifest
    ///     .iter()
    ///     .map(|entry| entry.to_detached())
    ///     .collect();
    ///
    /// drop(epub);
    ///
    /// // Detached manifest entries are accessible even after `epub` is dropped:
    /// assert_eq!(12, detached.len());
    /// # Ok(())
    /// # }
    /// ```
    pub fn to_detached(&self) -> DetachedEpubManifestEntry {
        DetachedEpubManifestEntry {
            id: self.id.to_owned(),
            data: self.data.clone(),
            content: None,
        }
    }

    /// Creates an owned detached manifest entry by cloning,
    /// alongside cloning the binary content in-memory.
    ///
    /// The binary content is directly retrievable via:
    /// - [`DetachedEpubManifestEntry::content_ref`]
    /// - [`DetachedEpubManifestEntry::content_mut`]
    ///
    /// Indirectly retrievable through [`DetachedEpubMetaEntry::as_view`]:
    /// - [`Self::copy_bytes`]
    /// - [`Self::read_str`]
    /// - [`Self::read_bytes`]
    ///
    /// # Errors
    /// [`ArchiveError`](crate::ebook::errors::ArchiveError):
    /// If the resource content cannot be read from the archive.
    ///
    /// # Examples
    /// - Cloning a manifest entry and its binary content in-memory:
    /// ```
    /// # use rbook::Epub;
    /// # use rbook::ebook::resource::ResourceContent;
    /// # fn main() -> rbook::ebook::errors::EbookResult<()> {
    /// let mut epub = Epub::open("tests/ebooks/example_epub")?;
    ///
    /// let chapter_1 = epub.manifest().by_id("c1").unwrap();
    ///
    /// // Cloning `chapter_1` and its binary content
    /// let mut detached = chapter_1.to_detached_with_content()?;
    ///
    /// drop(epub);
    ///
    /// // Checking the in-memory buffer
    /// let mut content = detached.content_ref().unwrap();
    /// assert!(content.is_memory());
    ///
    /// if let ResourceContent::Memory(bytes) = content {
    ///     assert!(bytes.starts_with(br#"<?xml version="1.0" encoding="UTF-8"?>"#));
    /// }
    /// assert_eq!("c1", detached.as_view().id());
    /// # Ok(())
    /// # }
    /// ```
    pub fn to_detached_with_content(&self) -> ArchiveResult<DetachedEpubManifestEntry> {
        self.read_bytes()
            .map(|content| self.to_detached().content(content))
    }
}

/// An owned [`EpubManifestEntry`] detached from an [`Epub`](crate::epub::Epub).
///
/// This struct acts as a builder for creating new manifest entries
/// before insertion into [`EpubManifestMut`].
///
/// # Examples
/// - Inserting an image into a manifest:
/// ```
/// # use rbook::Epub;
/// # use rbook::ebook::epub::manifest::DetachedEpubManifestEntry;
/// # const IMAGE_BYTES: &[u8] = &[];
/// let mut epub = Epub::new();
///
/// // Insertion
/// epub.manifest_mut().push(
///     DetachedEpubManifestEntry::new("art1")
///         .href("art_1.jpg")
///         .content(IMAGE_BYTES),
/// );
///
/// // Retrieval
/// let art_1 = epub.manifest().by_id("art1").unwrap();
/// assert_eq!("art1", art_1.id());
/// assert_eq!("art_1.jpg", art_1.href_raw());
/// assert_eq!("image/jpeg", art_1.media_type());
/// # assert_eq!("/OEBPS/art_1.jpg", art_1.href());
/// ```
#[derive(Clone, Debug, PartialEq)]
pub struct DetachedEpubManifestEntry {
    id: String,
    data: EpubManifestEntryData,
    content: Option<ResourceContent>,
}

impl DetachedEpubManifestEntry {
    /// Creates a new manifest entry (`<item>`) with the given [`id`](Self::id).
    pub fn new(id: impl Into<String>) -> Self {
        Self {
            id: id.into(),
            data: EpubManifestEntryData::default(),
            content: None,
        }
    }

    /// Returns a mutable view to modify an entry's data,
    /// useful for modifications without builder-esque methods.
    pub fn as_mut(&mut self) -> EpubManifestEntryMut<'_> {
        EpubManifestEntryMut::new(
            EpubPackageMetaContext::EMPTY,
            ManifestEntryDataHandle::Detached {
                id: &mut self.id,
                data: &mut self.data,
                content: &mut self.content,
            },
        )
    }

    /// Returns a read-only view, useful for inspecting state before applying modifications.
    ///
    /// Attempting to retrieve any [fallbacks](EpubManifestEntry::fallback)
    /// or [media overlays](EpubManifestEntry::media_overlay) on a detached instance will not work,
    /// returning [`None`].
    pub fn as_view(&self) -> EpubManifestEntry<'_> {
        EpubManifestContext::detached(self.content.as_ref()).create_entry(&self.id, &self.data)
    }

    /// Returns a reference to the associated [resource content](ResourceContent), if present.
    pub fn content_ref(&self) -> Option<&ResourceContent> {
        self.content.as_ref()
    }

    /// Returns a mutable reference to the associated [resource content](ResourceContent),
    /// if present.
    ///
    /// This method is useful for in-place modification of memory buffers without re-allocation.
    ///
    /// # Examples
    /// - Modifying the resource content of a detached manifest entry:
    /// ```
    /// # use rbook::ebook::resource::ResourceContent;
    /// # use rbook::epub::manifest::DetachedEpubManifestEntry;
    /// let mut detached = DetachedEpubManifestEntry::new("txt")
    ///     .content("Hello World!");
    ///
    /// // Modify the in-memory buffer in-place
    /// if let Some(ResourceContent::Memory(bytes)) = detached.content_mut() {
    ///     bytes.extend_from_slice(b" Goodbye!");
    ///     assert_eq!(b"Hello World! Goodbye!", bytes.as_slice());
    /// }
    /// ```
    pub fn content_mut(&mut self) -> Option<&mut ResourceContent> {
        self.content.as_mut()
    }

    /// Sets the raw byte content of a resource (e.g., XHTML, images, fonts).
    ///
    /// # See Also
    /// - [`Self::content_ref`] and [`Self::content_mut`]
    ///   for direct access to the given [`ResourceContent`].
    /// - [`Self::media_type`] to set the type of resource, if needed.
    /// - [`ResourceContent`] for details on providing data from memory (bytes/strings)
    ///   or the OS file system (paths).
    ///
    /// # Examples
    /// - Referencing image data from a file stored on disk:
    /// ```no_run
    /// # use std::path::PathBuf;
    /// # use rbook::ebook::epub::manifest::DetachedEpubManifestEntry;
    /// let detached = DetachedEpubManifestEntry::new("art1")
    ///     // The location where the resource will be stored within the EPUB.
    ///     .href("art_1.jpg")
    ///     // The location of the source file on the OS file system.
    ///     .content(PathBuf::from("path/to/image/1.jpg"));
    /// ```
    pub fn content(mut self, content: impl Into<ResourceContent>) -> Self {
        self.as_mut().set_content(content);
        self
    }

    /// Sets the unique `id`.
    ///
    /// # Uniqueness
    /// IDs must be unique within the entire package document (`.opf`).
    /// Duplicate IDs will result in invalid XML and behavior is undefined
    /// for reading systems.
    ///
    /// Ensure that `ids` are unique across:
    /// - Manifest entries
    /// - [Spine entries](crate::epub::spine::DetachedEpubSpineEntry::id)
    /// - [Metadata/Refinement entries](DetachedEpubMetaEntry::id)
    ///
    /// Other than the EPUB 2 guide,
    /// ToC entries ([`EpubTocEntry`](crate::epub::toc::EpubTocEntry)) are exempt
    /// from this restriction, as they reside in a separate file (`toc.ncx/xhtml`).
    ///
    /// # Refinements
    /// If the entry has refinements (children), their `refines` field
    /// are linked implicitly.
    pub fn id(mut self, id: impl Into<String>) -> Self {
        self.as_mut().set_id(id);
        self
    }

    /// Sets the href (resource location).
    ///
    /// **This method sets the value of the [`EpubManifestEntry::href_raw`] field.**
    ///
    /// # Uniqueness
    /// Hrefs **must** be unique within the entire package document (`.opf`).
    /// Duplicate hrefs will result in a malformed package and discarded content.
    /// Reading system behavior is unpredictable.
    ///
    /// # Percent Encoding
    /// The given `raw_href` is expected to already be percent encoded.
    ///
    /// For maximum compatibility with reading systems,
    /// it is recommended to only use alphanumeric characters,
    /// dashes (`-`), and underscores (`_`) in directory and file names.
    ///
    /// - **Malformed**: `My+chapter & #1.xhtml` (Invalid; Not percent-encoded)
    /// - Not recommended: `my%20chapter%20no1.xhtml` (Valid; percent-encoded)
    /// - Recommended: `my_chapter_no1.xhtml` (Valid)
    ///
    /// # Note
    /// - This method does not check for href uniqueness or validity.
    /// - Calling this method will set [`EpubManifestEntry::href`] to an empty location (`""`),
    ///   as there is no [package directory](crate::epub::package::EpubPackage::directory)
    ///   to resolve against for a detached entry.
    ///   The resolved href is computed once the entry is inserted into an [`Epub`](crate::epub::Epub).
    ///
    /// # See Also
    /// - [`EpubEditor::resource`](crate::epub::EpubEditor::resource):
    ///   The same path resolution rules apply to this method
    ///   when a detached manifest entry is inserted into a manifest.
    ///
    /// # Examples
    /// - Setting the href and inserting into a manifest:
    /// ```
    /// # use rbook::Epub;
    /// # use rbook::ebook::epub::manifest::DetachedEpubManifestEntry;
    /// # const IMAGE_BYTES: &[u8] = &[];
    /// let mut epub = Epub::new();
    ///
    /// // Insertion
    /// epub.manifest_mut().push(
    ///     DetachedEpubManifestEntry::new("image1")
    ///         // Providing an href
    ///         .href("image/john_doe_1.jpg")
    ///         .content(IMAGE_BYTES),
    /// );
    ///
    /// // Retrieval
    /// let image1 = epub.manifest().by_id("image1").unwrap();
    /// // Resolved percent-encoded href
    /// assert_eq!("/OEBPS/image/john_doe_1.jpg", image1.href());
    /// // Source href (original input)
    /// assert_eq!("image/john_doe_1.jpg", image1.href_raw());
    /// ```
    pub fn href(mut self, raw_href: impl Into<String>) -> Self {
        self.as_mut().set_href(raw_href);
        self
    }

    /// Sets the media type, indicating the specific type of content an entry contains.
    ///
    /// The given `media_type` is not validated and ***should*** be a valid
    /// [MIME](https://www.iana.org/assignments/media-types/media-types.xhtml).
    ///
    /// If no explicit media type is provided, it is inferred upon insertion into a manifest.
    /// See [`EpubEditor::resource`](crate::epub::EpubEditor::resource) for inference details.
    ///
    /// # Examples
    /// - Inferring the media type:
    /// ```
    /// # use rbook::Epub;
    /// # use rbook::epub::manifest::DetachedEpubManifestEntry;
    /// # const CSS_BYTES: &[u8] = &[];
    /// # const XHTML_BYTES: &[u8] = &[];
    /// # const PNG_BYTES: &[u8] = &[];
    /// let mut epub = Epub::new();
    ///
    /// epub.manifest_mut().push([
    ///     DetachedEpubManifestEntry::new("a").href("a.css").content(CSS_BYTES),
    ///     DetachedEpubManifestEntry::new("b").href("b.xhtml").content(XHTML_BYTES),
    ///     DetachedEpubManifestEntry::new("c").href("c.png").content(PNG_BYTES),
    /// ]);
    ///
    /// let manifest = epub.manifest();
    /// assert_eq!("text/css", manifest.by_id("a").unwrap().media_type());
    /// assert_eq!("application/xhtml+xml", manifest.by_id("b").unwrap().media_type());
    /// assert_eq!("image/png", manifest.by_id("c").unwrap().media_type());
    /// ```
    pub fn media_type(mut self, media_type: impl Into<String>) -> Self {
        self.as_mut().set_media_type(media_type);
        self
    }

    /// Sets the fallback resource to use when an entry cannot be rendered
    /// by a reading system.
    ///
    /// The given `idref` ***should*** match the [`id`](Self::id) of an
    /// entry in the [`EpubManifest`] to fall back to.
    pub fn fallback(mut self, idref: impl IntoOption<String>) -> Self {
        self.as_mut().set_fallback(idref);
        self
    }

    /// Sets the SMIL media overlay resource, providing pre-recorded
    /// narration for the associated content.
    ///
    /// The given `idref` ***should*** match the [`id`](Self::id) of an
    /// entry in the [`EpubManifest`] to narrate over.
    ///
    /// # Note
    /// This is an EPUB 3 feature.
    /// When [writing](crate::Epub::write) an EPUB 2 ebook, this field is ignored.
    pub fn media_overlay(mut self, idref: impl IntoOption<String>) -> Self {
        self.as_mut().set_media_overlay(idref);
        self
    }

    /// Appends one or more properties (e.g., `cover-image`, `nav`) via [`Properties::insert`].
    ///
    /// # Note
    /// This is an EPUB 3 feature.
    /// When [writing](crate::Epub::write) an EPUB 2 ebook, properties are ignored.
    ///
    /// # See Also
    /// - [`EpubManifestEntryMut::properties_mut`] for a modifiable collection of attributes
    ///   through [`Self::as_mut`].
    pub fn property(mut self, property: &str) -> Self {
        self.as_mut().properties_mut().insert(property);
        self
    }

    /// Inserts one or more XML attributes via the [`Many`] trait.
    ///
    /// # Omitted Attributes
    /// The following attributes **should not** be set via this method
    /// as they have dedicated setters.
    /// If set here, they are ignored during [writing](crate::epub::Epub::write):
    /// - [`id`](Self::id)
    /// - [`href`](Self::href)
    /// - [`media_type`](Self::media_type)
    /// - [`fallback`](Self::fallback)
    /// - [`media_overlay`](Self::media_overlay)
    /// - [`properties`](Self::property)
    ///
    /// # See Also
    /// - [`EpubManifestEntryMut::attributes_mut`] for a modifiable collection of attributes
    ///   through [`Self::as_mut`].
    pub fn attribute(mut self, attribute: impl Many<Attribute>) -> Self {
        self.as_mut().attributes_mut().extend(attribute.iter_many());
        self
    }

    /// Appends one or more refinements to this entry via the [`Many`] trait.
    ///
    /// A refinement is a metadata entry that provides extra information about its parent.
    ///
    /// # Note
    /// This is an EPUB 3 feature.
    /// When [writing](crate::Epub::write) an EPUB 2 ebook, refinements are ignored.
    ///
    /// # See Also
    /// - [`EpubManifestEntryMut::refinements_mut`] for a modifiable collection of refinements
    ///   through [`Self::as_mut`].
    pub fn refinement(mut self, detached: impl Many<DetachedEpubMetaEntry>) -> Self {
        self.as_mut().refinements_mut().push(detached);
        self
    }
}

/// Creates a [`DetachedEpubManifestEntry`],
/// where `H` is the href and `C` is the resource content.
///
/// The [`id`](DetachedEpubManifestEntry::id) is generated from the given `href` via slugging:
/// - ASCII alphanumeric characters are retained and decapitalized.
/// - All other characters are replaced with `-`.
/// - For example: `images/art1.png` (href) -> `images_art1_png` (generated `id`)
///
/// # See Also
/// - [`Self::href`] for important href details.
/// - [`ResourceContent`] for details on providing data from memory (bytes/strings)
///   or the OS file system (paths).
impl<H: Into<String>, C: Into<ResourceContent>> From<(H, C)> for DetachedEpubManifestEntry {
    fn from((href, content): (H, C)) -> Self {
        let href = href.into();

        // Generate id from href
        let mut id = util::str::slugify(&href);

        // Ensure the id doesn't start with a number
        if id.chars().next().is_some_and(char::is_numeric) {
            id.insert(0, '_');
        }

        Self::new(id).href(href).content(content)
    }
}

/// Mutable view of [`EpubManifest`] accessible via
/// [`Epub::manifest_mut`](crate::epub::Epub::manifest_mut).
///
/// Allows the management of resources, including
/// adding, removing, and modifying manifest entries.
///
/// # See Also
/// - [`EpubEditor`](crate::epub::EpubEditor) for simple modification tasks.
pub struct EpubManifestMut<'ebook> {
    href_resolver: UriResolver<'ebook>,
    meta_ctx: EpubPackageMetaContext<'ebook>,
    archive: &'ebook mut EpubArchive,
    manifest: &'ebook mut EpubManifestData,

    // References to other content (to synchronize id/href changes)
    // - Metadata may include references that assist manifest item lookup.
    //   For example, looking up the cover image via the EPUB 2 cover metadata entry.
    metadata: &'ebook mut EpubMetadataData,
    spine: &'ebook mut EpubSpineData,
    toc: &'ebook mut EpubTocData,
}

impl<'ebook> EpubManifestMut<'ebook> {
    pub(in crate::ebook::epub) fn new(
        href_resolver: UriResolver<'ebook>,
        meta_ctx: EpubPackageMetaContext<'ebook>,
        archive: &'ebook mut EpubArchive,
        manifest: &'ebook mut EpubManifestData,
        metadata: &'ebook mut EpubMetadataData,
        spine: &'ebook mut EpubSpineData,
        toc: &'ebook mut EpubTocData,
    ) -> Self {
        Self {
            href_resolver,
            meta_ctx,
            archive,
            manifest,
            metadata,
            spine,
            toc,
        }
    }

    fn get(&mut self, index: usize) -> EpubManifestEntryMut<'_> {
        EpubManifestEntryMut::new(
            self.meta_ctx,
            ManifestEntryDataHandle::Attached(AttachedEntryContext {
                href_resolver: self.href_resolver,
                archive: self.archive,
                manifest: self.manifest,
                metadata: self.metadata,
                spine: self.spine,
                toc: self.toc,
                index,
            }),
        )
    }

    fn insert_detached(&mut self, mut entry: DetachedEpubManifestEntry) {
        // Resolve href
        entry.data.resolve_href(self.href_resolver);

        // Update Archive:
        // If present, store the associated binary resource in the archive
        if let Some(binary) = entry.content {
            self.archive.insert(entry.data.href.clone(), binary);
        }

        // Infer media type if not provided
        if entry.data.media_type.is_empty() {
            entry.data.media_type = resource::write::infer_media_type(&entry.data.href);
        }

        let (i, replaced) = self.manifest.entries.insert_full(entry.id, entry.data);

        // Remove the replaced entry
        if let Some(old) = replaced {
            let new = &self.manifest.entries[i];

            // Check if archive must be replaced!
            if old.href != new.href {
                // Remove orphaned resource
                self.archive.remove(&old.href);
            }
        }
    }

    //////////////////////////////////
    // PUBLIC API
    //////////////////////////////////

    /// Inserts one or more entries via the [`Many`] trait.
    ///
    /// # Replacements
    /// Duplicate IDs are overridden.
    /// For example, if an entry with the same `id` exists within the manifest, it is replaced.
    ///
    /// ToC entries that reference the replaced manifest entry’s href are orphaned if
    /// the new entry has a different [`href`](EpubManifestEntry::href).
    /// See [`Epub::cleanup`](crate::epub::Epub::cleanup) to remove orphaned entries.
    ///
    /// # See Also
    /// - [`EpubEditor::container_resource`](crate::epub::EpubEditor::container_resource)
    ///   to insert a resource without adding it to the manifest.
    pub fn push(&mut self, detached: impl Many<DetachedEpubManifestEntry>) {
        for entry in detached.iter_many() {
            self.insert_detached(entry);
        }
    }

    /// Returns a mutable view of the entry with the given `id`, if present.
    pub fn by_id_mut(&mut self, id: &str) -> Option<EpubManifestEntryMut<'_>> {
        self.manifest
            .entries
            .get_index_of(id)
            .map(|index| self.get(index))
    }

    /// The mutable cover image entry in the manifest, if present.
    ///
    /// This method returns the entry with the `cover-image` property,
    /// falling back to EPUB 2 cover metadata for lookup.
    ///
    /// # See Also
    /// - [`EpubEditor::cover_image`](crate::epub::EpubEditor::cover_image)
    ///   to conveniently create or set a new cover image resource.
    pub fn cover_image_mut(&mut self) -> Option<EpubManifestEntryMut<'_>> {
        for (i, data) in self.manifest.entries.values().enumerate() {
            if data.properties.has_property(opf::COVER_IMAGE) {
                return Some(self.get(i));
            }
        }
        // Fallback to EPUB 2
        if let Some(cover_id) = self.metadata.epub2_cover_image_id() {
            return self
                .manifest
                .entries
                .get_index_of(cover_id)
                .map(|i| self.get(i));
        }
        None
    }

    /// Calls a closure to mutate each entry within the manifest.
    ///
    /// This method is equivalent to calling [`Self::iter_mut`]
    /// and iterating over each entry.
    /// However, note that control flow such as
    /// `break` and `continue` are not possible from a closure.
    ///
    /// This primarily exists as [`EpubManifestMut`] cannot implement [`IntoIterator`]
    /// as entries must borrow from an iterator itself (**lending iterator**).
    /// As such, [`EpubManifestMut`] does not support standard `for` loops.
    ///
    /// # See Also
    /// - [`Self::iter_mut`] to iterate with control flow (`while let`).
    /// - [`Self::into_iter`] to consume the manifest by value and iterate with control flow.
    ///
    /// # Examples
    /// - Adding a [property](EpubManifestEntryMut::properties_mut) based on a predicate:
    /// ```
    /// # use rbook::Epub;
    /// # let mut epub = Epub::new();
    /// epub.manifest_mut().for_each_mut(|entry| {
    ///     if entry.as_view().href().name().as_str().ends_with("math") {
    ///         entry.properties_mut().insert("mathml");
    ///     }
    ///     // `break` not possible; all entries must be iterated over
    /// });
    /// ```
    pub fn for_each_mut(&mut self, mut f: impl FnMut(&mut EpubManifestEntryMut<'_>)) {
        let mut entries = self.iter_mut();

        while let Some(mut entry) = entries.next() {
            f(&mut entry);
        }
    }

    /// Returns a **lending** iterator over **all** manifest entries.
    ///
    /// The returned lending iterator provides **only** a single method:
    /// [`EpubManifestMutIter::next`]
    pub fn iter_mut(&mut self) -> EpubManifestMutIter<'_> {
        EpubManifestMutIter {
            href_resolver: self.href_resolver,
            meta_ctx: self.meta_ctx,
            archive: self.archive,
            metadata: self.metadata,
            manifest: self.manifest,
            spine: self.spine,
            toc: self.toc,
            index: 0,
        }
    }

    /// Returns a **lending** iterator over **all** manifest entries.
    ///
    /// The returned lending iterator provides **only** a single method:
    /// [`EpubManifestMutIter::next`]
    #[allow(clippy::should_implement_trait)]
    pub fn into_iter(self) -> EpubManifestMutIter<'ebook> {
        EpubManifestMutIter {
            href_resolver: self.href_resolver,
            meta_ctx: self.meta_ctx,
            archive: self.archive,
            manifest: self.manifest,
            metadata: self.metadata,
            spine: self.spine,
            toc: self.toc,
            index: 0,
        }
    }

    /// Removes and returns the entry matching the given `id`, if present.
    ///
    /// # See Also
    /// - [`Epub::cleanup`](crate::epub::Epub::cleanup) to remove orphaned entries.
    pub fn remove_by_id(&mut self, id: &str) -> Option<DetachedEpubManifestEntry> {
        self.manifest
            .entries
            .shift_remove_entry(id)
            .map(|(id, data)| DetachedEpubManifestEntry {
                content: self.archive.remove(&data.href),
                id,
                data,
            })
    }

    /// Retains only the entries specified by the predicate.
    ///
    /// If the closure returns `false`, the entry is retained.
    /// Otherwise, the entry is removed.
    ///
    /// This method operates in place and visits every entry exactly once.
    ///
    /// # See Also
    /// - [`Epub::cleanup`](crate::epub::Epub::cleanup) to remove orphaned entries.
    /// - [`Self::extract_if`] to retrieve an iterator of the removed entries.
    pub fn retain(&mut self, mut f: impl FnMut(EpubManifestEntry<'_>) -> bool) {
        self.manifest.entries.retain(|id, entry| {
            let ctx = EpubManifestContext::attached(self.archive, self.meta_ctx);
            let retain = f(ctx.create_entry(id, entry));

            if !retain {
                self.archive.remove(&entry.href);
            }
            retain
        });
    }

    /// Removes and returns only the entries specified by the predicate.
    ///
    /// If the closure returns `true`, the entry is removed and yielded.
    /// Otherwise, the entry is retained.
    ///
    /// # Drop
    /// If the returned iterator is not exhausted,
    /// (e.g. dropped without iterating or iteration short-circuits),
    /// then the remaining entries are retained.
    ///
    /// Prefer [`Self::retain`] with a negated predicate if the returned iterator is not needed.
    ///
    /// # See Also
    /// - [`Epub::cleanup`](crate::epub::Epub::cleanup) to remove orphaned entries.
    ///
    /// # Examples
    /// - Extracting all image entries:
    /// ```
    /// # use rbook::Epub;
    /// # fn main() -> rbook::ebook::errors::EbookResult<()> {
    /// let mut epub = Epub::open("tests/ebooks/example_epub")?;
    ///
    /// let non_linear: Vec<_> = epub.manifest_mut()
    ///     .extract_if(|entry| entry.kind().is_image())
    ///     .collect();
    /// # Ok(())
    /// # }
    /// ```
    pub fn extract_if(
        &mut self,
        mut f: impl FnMut(EpubManifestEntry<'_>) -> bool,
    ) -> impl Iterator<Item = DetachedEpubManifestEntry> {
        self.manifest
            .entries
            .extract_if(.., move |id, entry| {
                f(EpubManifestContext::EMPTY.create_entry(id, entry))
            })
            .map(|(id, data)| DetachedEpubManifestEntry {
                content: self.archive.remove(&data.href),
                id,
                data,
            })
    }

    /// Removes and returns all manifest entries.
    ///
    /// # See Also
    /// - [`Epub::cleanup`](crate::epub::Epub::cleanup) to remove orphaned entries.
    pub fn drain(&mut self) -> impl Iterator<Item = DetachedEpubManifestEntry> {
        self.manifest
            .entries
            .drain(..)
            .map(|(id, data)| DetachedEpubManifestEntry {
                content: self.archive.remove(&data.href),
                id,
                data,
            })
    }

    /// Removes all manifest entries.
    ///
    /// # See Also
    /// - [`Epub::cleanup`](crate::epub::Epub::cleanup) to remove orphaned entries.
    /// - [`Self::drain`] to retrieve an iterator of the removed entries.
    pub fn clear(&mut self) {
        for (_, removed) in self.manifest.entries.drain(..) {
            self.archive.remove(&removed.href);
        }
    }

    /// Returns a read-only view, useful for inspecting state before applying modifications.
    pub fn as_view(&self) -> EpubManifest<'_> {
        EpubManifest::new(
            ResourceProvider::Archive(self.archive),
            self.meta_ctx,
            self.manifest,
            self.metadata,
        )
    }
}

impl Debug for EpubManifestMut<'_> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> Result<(), std::fmt::Error> {
        f.debug_struct("EpubManifestMut")
            .field("href_resolver", &self.href_resolver)
            .field("manifest", &self.manifest)
            .finish_non_exhaustive()
    }
}

impl Extend<DetachedEpubManifestEntry> for EpubManifestMut<'_> {
    fn extend<T: IntoIterator<Item = DetachedEpubManifestEntry>>(&mut self, iter: T) {
        for entry in iter.into_iter() {
            self.insert_detached(entry);
        }
    }
}

/// A lending iterator over all the mutable
/// [entries](EpubManifestEntryMut) contained within [`EpubManifestMut`].
///
/// Compared to a traditional [`Iterator`],
/// yielded items borrow from the iterator itself:
/// - Only one yielded item can exist at a time.
/// - Standard `for` loops and combinators are not supported.
/// - Using a `while let` loop is recommended.
///
/// # See Also
/// - [`EpubManifestMut::iter_mut`] to create an instance of this struct.
pub struct EpubManifestMutIter<'ebook> {
    href_resolver: UriResolver<'ebook>,
    meta_ctx: EpubPackageMetaContext<'ebook>,
    archive: &'ebook mut EpubArchive,
    manifest: &'ebook mut EpubManifestData,
    metadata: &'ebook mut EpubMetadataData,
    spine: &'ebook mut EpubSpineData,
    toc: &'ebook mut EpubTocData,
    index: usize,
}

impl EpubManifestMutIter<'_> {
    /// Advances the iterator and returns the next manifest entry.
    ///
    /// Returns [`None`] when iteration is finished.
    #[allow(clippy::should_implement_trait)]
    pub fn next(&mut self) -> Option<EpubManifestEntryMut<'_>> {
        let entries = &mut self.manifest.entries;

        if self.index < entries.len() {
            let index = self.index;
            self.index += 1;

            Some(EpubManifestEntryMut::new(
                self.meta_ctx,
                ManifestEntryDataHandle::Attached(AttachedEntryContext {
                    href_resolver: self.href_resolver,
                    archive: self.archive,
                    manifest: self.manifest,
                    metadata: self.metadata,
                    spine: self.spine,
                    toc: self.toc,
                    index,
                }),
            ))
        } else {
            None
        }
    }
}

impl Debug for EpubManifestMutIter<'_> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> Result<(), std::fmt::Error> {
        f.debug_struct("EpubManifestMutIter")
            .field("href_resolver", &self.href_resolver)
            .field("index", &self.index)
            .finish_non_exhaustive()
    }
}

// As of now, only one instance can exist at a time
/// Mutable view of [`EpubManifestEntry`], allowing modification of manifest entry (`item`) fields,
/// attributes, and [refinements](Self::refinements_mut).
///
/// # See Also
/// - [`DetachedEpubManifestEntry`] for an owned manifest entry instances.
pub struct EpubManifestEntryMut<'ebook> {
    meta_ctx: EpubPackageMetaContext<'ebook>,
    data: ManifestEntryDataHandle<'ebook>,
}

impl<'ebook> EpubManifestEntryMut<'ebook> {
    fn new(
        meta_ctx: EpubPackageMetaContext<'ebook>,
        data: ManifestEntryDataHandle<'ebook>,
    ) -> Self {
        Self { meta_ctx, data }
    }

    /// Sets the raw byte content of a resource (e.g., XHTML, images, fonts) and returns
    /// the previous content.
    ///
    /// The previous content is **only** returned if it was explicitly stored in-memory via
    /// [`DetachedEpubManifestEntry::content`] or this method prior.
    /// Otherwise, [`None`] is returned.
    ///
    /// # Note
    /// After setting the content, all resource content retrieval methods
    /// (e.g. [`Ebook::copy_resource`](crate::Ebook::copy_resource),
    /// [`ManifestEntry::copy_bytes`](crate::ebook::manifest::ManifestEntry::copy_bytes))
    /// will return the newly set content instead.
    ///
    /// # See Also
    /// - [`DetachedEpubManifestEntry::content`] for more details.
    /// - [`ResourceContent`] for details on providing data from memory (bytes/strings)
    ///   or the OS file system (paths).
    pub fn set_content(&mut self, content: impl Into<ResourceContent>) -> Option<ResourceContent> {
        match &mut self.data {
            ManifestEntryDataHandle::Attached(ctx) => {
                // Update the archive
                let data = &mut ctx.manifest.entries[ctx.index];

                ctx.archive.insert(data.href.clone(), content.into())
            }
            ManifestEntryDataHandle::Detached {
                content: current, ..
            } => current.replace(content.into()),
        }
    }

    /// Sets the `id`, ensuring it is unique within the manifest, and returns the previous value.
    ///
    /// If the manifest already contains the given id, a numeric suffix is appended:
    ///
    /// `my-id` -> `my-id-1` -> `my-id-2`, etc.
    ///
    /// # Cascading Updates
    /// Setting the ID updates references to the old ID for:
    /// - Spine `idref` references.
    /// - The EPUB 2 `cover` metadata entry.
    ///
    /// Cascading updates can be disabled via [`IdOptions::cascade`] (`true` by default).
    ///
    /// # See Also
    /// - [`Self::as_view`] to inspect the resolved ID with [`EpubManifestEntry::id`].
    /// - [`Self::try_set_id`] to retrieve an error if an ID collision occurs.
    ///
    /// # Examples
    /// - Updating the ID of an entry:
    /// ```
    /// # use rbook::Epub;
    /// # use rbook::epub::manifest::DetachedEpubManifestEntry;
    /// # use rbook::epub::spine::DetachedEpubSpineEntry;
    /// let mut epub = Epub::new();
    /// # epub.spine_mut().push([
    /// #     DetachedEpubSpineEntry::new("c1"),
    /// #     DetachedEpubSpineEntry::new("c2000"),
    /// # ]);
    /// # epub.manifest_mut().push([
    /// #     DetachedEpubManifestEntry::new("c1"),
    /// #     DetachedEpubManifestEntry::new("c2000"),
    /// # ]);
    ///
    /// // ..insert entries..
    ///
    /// // Current spine entries
    /// let spine: Vec<_> = epub.spine().iter().map(|e| e.idref()).collect();
    /// assert_eq!(&["c1", "c2000"], spine.as_slice());
    ///
    /// let mut manifest = epub.manifest_mut();
    /// let mut entry = manifest.by_id_mut("c2000").unwrap();
    /// // Updating the manifest entry automatically updates spine references
    /// let old_id = entry.set_id("c2");
    /// assert_eq!("c2000", old_id);
    ///
    /// // Changes cascaded to the spine
    /// let spine: Vec<_> = epub.spine().iter().map(|e| e.idref()).collect();
    /// assert_eq!(&["c1", "c2"], spine.as_slice());
    /// ```
    pub fn set_id(&mut self, id: impl Into<IdOptions>) -> String {
        let mut id_options = id.into();

        // If attached, make the given id unique
        if let ManifestEntryDataHandle::Attached(ctx) = &self.data {
            // If the id is already unique, the same is returned.
            id_options.id = ctx.manifest.generate_unique_id(id_options.id);
        }

        self.try_set_id(id_options.id)
            .expect("The given id should be unique")
    }

    /// Sets the ID and returns the previous value, failing if the given `id` is not unique.
    ///
    /// Any spine entries that reference the old ID are automatically updated to the new ID.
    /// If cascading updates are not desired, disable [`IdOptions::cascade`] (`true` by default).
    ///
    /// # Errors
    /// - [`EpubError::DuplicateItemId`]:
    ///   If another manifest entry already has the given `id`.
    ///
    /// # See Also
    /// - [`Self::set_id`] to automatically suffix the given `id` if it is not unique.
    ///
    /// # Examples
    /// - Attempting to set a duplicate id:
    /// ```
    /// # use rbook::Epub;
    /// # use rbook::epub::manifest::DetachedEpubManifestEntry;
    /// # const C1_XHTML: &[u8] = &[];
    /// # const C2_XHTML: &[u8] = &[];
    /// let mut epub = Epub::new();
    /// let mut manifest = epub.manifest_mut();
    ///
    /// manifest.push([
    ///     DetachedEpubManifestEntry::new("c1")
    ///         .href("c1.xhtml")
    ///         .content(C1_XHTML),
    ///     DetachedEpubManifestEntry::new("c2")
    ///         .href("c2.xhtml")
    ///         .content(C2_XHTML),
    /// ]);
    ///
    /// let mut entry = manifest.by_id_mut("c2").unwrap();
    ///
    /// // Unable to set the ID to `c1` as it exists already
    /// assert!(entry.try_set_id("c1").is_err());
    /// ```
    pub fn try_set_id(&mut self, id: impl Into<IdOptions>) -> Result<String, EpubError> {
        let options = id.into();

        match &mut self.data {
            ManifestEntryDataHandle::Attached(ctx) => ctx.update_entry_id(options),
            // If detached; return early
            ManifestEntryDataHandle::Detached { id, .. } => Ok(std::mem::replace(id, options.id)),
        }
    }

    /// Sets the [raw href](EpubManifestEntry::href_raw) and returns the previous raw value.
    ///
    /// The given raw href is ***expected*** to already be percent encoded.
    /// This method does **not** check for href uniqueness or validity.
    ///
    /// # Cascading Updates
    /// Any ToC [entries](crate::epub::toc::EpubTocEntry) that reference the old resolved href are
    /// updated to the new href.
    /// Any existing query or fragment on the toc href is preserved.
    ///
    /// **Updates do not cascade to XHTML files or any other locations.**
    ///
    /// Cascading updates can be disabled via [`HrefOptions::cascade`] (`true` by default).
    ///
    /// # Detached Entries
    /// Setting this field from a [`DetachedEpubManifestEntry`] will make
    /// [`EpubManifestEntry::href`] return an empty location (`""`), as there is no
    /// [package directory](crate::epub::package::EpubPackage::directory) to resolve against.
    /// The resolved href is computed once the entry is inserted into an [`Epub`](crate::epub::Epub).
    ///
    /// # See Also
    /// - [`DetachedEpubManifestEntry::href`] for important details.
    /// - [`EpubManifestEntry::href_raw`] to retrieve the href given here.
    /// - [`EpubEditor::resource`](crate::epub::EpubEditor::resource) for path details.
    ///   The same path resolution rules apply to this method.
    ///
    /// # Examples
    /// - Relocating a resource:
    /// ```
    /// # use rbook::Epub;
    /// # fn main() -> rbook::ebook::errors::EbookResult<()> {
    /// let mut epub = Epub::open("tests/ebooks/example_epub")?;
    ///
    /// // Resource does not exist:
    /// assert!(epub.read_resource_bytes("/EPUB/chapter/c1.xhtml").is_err());
    ///
    /// let mut manifest = epub.manifest_mut();
    /// let mut entry = manifest.by_id_mut("c1").unwrap();
    /// assert_eq!("/EPUB/c1.xhtml", entry.as_view().href());
    ///
    /// entry.set_href("chapters/c1.xhtml");
    /// assert_eq!("/EPUB/chapters/c1.xhtml", entry.as_view().href());
    ///
    /// // Resource exists at the location now:
    /// assert!(epub.read_resource_bytes("/EPUB/chapters/c1.xhtml").is_ok());
    /// # Ok(())
    /// # }
    /// ```
    pub fn set_href(&mut self, raw_href: impl Into<HrefOptions>) -> String {
        let options = raw_href.into();

        match &mut self.data {
            ManifestEntryDataHandle::Attached(ctx) => ctx.update_entry_href(options),
            ManifestEntryDataHandle::Detached { data, .. } => {
                data.href = String::new();
                std::mem::replace(&mut data.href_raw, options.href)
            }
        }
    }

    /// Sets the media type and returns the previous value.
    ///
    /// The given `media_type` is not validated and ***should*** be a valid
    /// [MIME](https://www.iana.org/assignments/media-types/media-types.xhtml).
    ///
    /// # See Also
    /// - [`DetachedEpubManifestEntry::media_type`] for more details.
    pub fn set_media_type(&mut self, media_type: impl Into<String>) -> String {
        std::mem::replace(&mut self.data.get_mut().media_type, media_type.into())
    }

    /// Sets the fallback and returns the previous fallback, if any.
    ///
    /// # See Also
    /// - [`DetachedEpubManifestEntry::fallback`] for more details.
    pub fn set_fallback(&mut self, idref: impl IntoOption<String>) -> Option<String> {
        std::mem::replace(&mut self.data.get_mut().fallback, idref.into_option())
    }

    /// Sets the media overlay and returns the previous overlay, if any.
    ///
    /// # See Also
    /// - [`DetachedEpubManifestEntry::media_overlay`] for more details.
    pub fn set_media_overlay(&mut self, idref: impl IntoOption<String>) -> Option<String> {
        std::mem::replace(&mut self.data.get_mut().media_overlay, idref.into_option())
    }

    /// Mutable view of all properties (e.g., `cover-image`, `nav`).
    pub fn properties_mut(&mut self) -> &mut Properties {
        &mut self.data.get_mut().properties
    }

    /// Mutable view of all additional `XML` attributes.
    ///
    /// # See Also
    /// - [`DetachedEpubManifestEntry::attribute`] for important details.
    pub fn attributes_mut(&mut self) -> &mut Attributes {
        &mut self.data.get_mut().attributes
    }

    /// Mutable view of all direct refinements.
    ///
    /// # See Also
    /// - [`DetachedEpubManifestEntry::refinement`]
    pub fn refinements_mut(&mut self) -> EpubRefinementsMut<'_> {
        let (id, data) = self.data.get_mut_with_id();

        EpubRefinementsMut::new(self.meta_ctx, Some(id), &mut data.refinements)
    }

    /// Returns a read-only view, useful for inspecting state before applying modifications.
    pub fn as_view(&self) -> EpubManifestEntry<'_> {
        let (id, data) = self.data.get_with_id();

        match &self.data {
            ManifestEntryDataHandle::Attached(ctx) => EpubManifestContext::new(
                ResourceProvider::Archive(ctx.archive),
                self.meta_ctx,
                Some(ctx.manifest),
            )
            .create_entry(id, data),
            ManifestEntryDataHandle::Detached { content, .. } => {
                EpubManifestContext::detached(content.as_ref()).create_entry(id, data)
            }
        }
    }
}

impl Debug for EpubManifestEntryMut<'_> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> Result<(), std::fmt::Error> {
        f.debug_struct("EpubManifestEntryMut")
            .field("data", &self.data)
            .finish_non_exhaustive()
    }
}

/// Options to set the [`id`](EpubManifestEntryMut::set_id)
/// of an [`EpubManifestEntryMut`] instance.
#[derive(Clone, Debug)]
pub struct IdOptions {
    id: String,
    cascade: bool,
}

impl IdOptions {
    /// Creates new options with the given `id`.
    pub fn new(id: String) -> Self {
        Self { cascade: true, id }
    }

    /// Sets whether to cascade updates to [`EpubSpineMut`](crate::epub::spine::EpubSpineMut).
    ///
    /// Spine entries that reference the old ID are automatically updated to the new ID.
    ///
    /// Default: `true`
    pub fn cascade(mut self, cascade: bool) -> Self {
        self.cascade = cascade;
        self
    }
}

impl<I: Into<String>> From<I> for IdOptions {
    fn from(id: I) -> Self {
        Self::new(id.into())
    }
}

/// Options to set the [`href`](EpubManifestEntryMut::set_href)
/// of an [`EpubManifestEntryMut`] instance.
#[derive(Clone, Debug)]
pub struct HrefOptions {
    href: String,
    cascade: bool,
}

impl HrefOptions {
    /// Creates new options with the given `href`.
    pub fn new(href: String) -> Self {
        Self {
            cascade: true,
            href,
        }
    }

    /// Sets whether to cascade updates to [`EpubTocMut`](crate::epub::toc::EpubTocMut).
    ///
    /// ToC entries that reference the old href are automatically updated
    /// (with the query and fragment retained) to the new href.
    ///
    /// Default: `true`
    pub fn cascade(mut self, cascade: bool) -> Self {
        self.cascade = cascade;
        self
    }
}

impl<I: Into<String>> From<I> for HrefOptions {
    fn from(href: I) -> Self {
        Self::new(href.into())
    }
}
