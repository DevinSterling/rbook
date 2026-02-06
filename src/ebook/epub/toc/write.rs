use crate::ebook::element::{Attribute, Attributes};
use crate::ebook::epub::manifest::EpubManifestContext;
use crate::ebook::epub::metadata::EpubVersion;
use crate::ebook::epub::toc::{
    EpubToc, EpubTocContext, EpubTocData, EpubTocEntry, EpubTocEntryData, EpubTocKey,
};
use crate::ebook::metadata::Version;
use crate::ebook::toc::TocEntryKind;
use crate::input::{IntoOption, Many};
use crate::util::iter::IteratorExt;
use crate::util::uri::{self, UriResolver};
use std::fmt::Debug;

////////////////////////////////////////////////////////////////////////////////
// PRIVATE API
////////////////////////////////////////////////////////////////////////////////

impl<'ebook> EpubTocContext<'ebook> {
    const EMPTY: EpubTocContext<'static> = EpubTocContext {
        manifest_ctx: EpubManifestContext::EMPTY,
    };

    fn create(self, data: &'ebook EpubTocData) -> EpubToc<'ebook> {
        EpubToc::new(self.manifest_ctx, data)
    }

    fn create_root_mut(
        self,
        version: EpubVersion,
        href_resolver: Option<UriResolver<'ebook>>,
        data: &'ebook mut EpubTocEntryData,
    ) -> EpubTocEntryMut<'ebook> {
        self.create_entry_mut(version, href_resolver, data, 0)
    }

    fn create_entry_mut(
        self,
        version: EpubVersion,
        href_resolver: Option<UriResolver<'ebook>>,
        data: &'ebook mut EpubTocEntryData,
        depth: usize,
    ) -> EpubTocEntryMut<'ebook> {
        EpubTocEntryMut::new(self, version, href_resolver, data, depth)
    }
}

impl EpubTocData {
    pub(crate) fn recursive_retain(
        &mut self,
        mut f: impl Copy + FnMut(&mut EpubTocEntryData) -> bool,
    ) {
        self.entries.retain(|_, entry| {
            let retain = f(entry);

            // check children
            if retain {
                entry.recursive_retain_children(f);
            }
            retain
        })
    }
}

impl EpubTocEntryData {
    pub(crate) fn cascade_toc_href(&mut self, old: &str, new: &str) {
        // If a toc entry references the old href,
        // update it to point to the new location.
        // * The fragment and query are retained
        if let Some(href) = &mut self.href
            && uri::path(href) == old
        {
            let query_and_fragment = href
                .find(['?', '#'])
                .map(|position| &href[position..])
                .unwrap_or_default();

            *href = new.to_owned() + query_and_fragment;
            // The previous `href_raw` no longer has any association with
            // the new `href`.
            // Setting it to `None` avoids confusion between
            // `href` and `href_raw` pointing to two different locations.
            self.href_raw = None;
        }

        // Update recursively
        for child in &mut self.children {
            child.cascade_toc_href(old, new);
        }
    }

    fn recursive_retain_children(&mut self, mut f: impl Copy + FnMut(&mut Self) -> bool) {
        self.children.retain_mut(|entry| {
            let retain = f(entry);

            // Check children
            if retain {
                entry.recursive_retain_children(f);
            }
            retain
        });
    }

    fn resolve_hrefs(&mut self, resolver: UriResolver<'_>) {
        if let Some(href_raw) = &self.href_raw {
            self.href.replace(resolver.resolve(href_raw));
        }

        for entry in &mut self.children {
            entry.resolve_hrefs(resolver);
        }
    }
}

////////////////////////////////////////////////////////////////////////////////
// PUBLIC API
////////////////////////////////////////////////////////////////////////////////

impl EpubTocEntry<'_> {
    /// Creates an owned detached ToC entry by cloning.
    ///
    /// # Note
    /// If the source ToC entry has an `id`, the detached entry will retain it.
    /// To avoid ID collisions if re-inserting into the same [`Epub`](crate::epub::Epub),
    /// consider clearing or changing the ID using
    /// [`DetachedEpubTocEntry::id`] or [`EpubTocEntryMut::set_id`].
    ///
    /// # See Also
    /// - [`EpubTocMut`] or [`EpubTocEntryMut`] to insert
    ///   detached entries into or remove entries without cloning.
    ///
    /// # Examples
    /// - Cloning ToC entries:
    /// ```
    /// # use rbook::Epub;
    /// # fn main() -> rbook::ebook::errors::EbookResult<()> {
    /// let mut epub = Epub::open("tests/ebooks/example_epub")?;
    ///
    /// let landmarks_root = epub.toc().landmarks().unwrap();
    /// assert_eq!(3, landmarks_root.len());
    ///
    /// // Cloning the entire hierarchy
    /// let detached_landmarks = landmarks_root.to_detached();
    ///
    /// drop(epub);
    ///
    /// // Detached ToC entries are accessible even after `epub` is dropped:
    /// assert_eq!(3, detached_landmarks.as_view().len());
    /// # Ok(())
    /// # }
    /// ```
    pub fn to_detached(&self) -> DetachedEpubTocEntry {
        DetachedEpubTocEntry {
            version: self.version,
            data: self.data.clone(),
        }
    }
}

/// An owned [`EpubTocEntry`] detached from an [`Epub`](crate::epub::Epub).
///
/// This struct acts as a builder for creating new ToC entries
/// before insertion into [`EpubTocMut`] or [`EpubTocEntryMut`].
///
/// # Note
/// - Root [`DetachedEpubTocEntry`] instances always have a
///   [depth](crate::ebook::toc::TocEntry::depth) of `0`.
///   Depth is calculated once the entry is inserted into
///   [`EpubTocMut`] or [`EpubTocEntryMut`].
/// - Calling [`EpubTocEntry::version`] from a detached entry created via
///   [`DetachedEpubTocEntry::new`] always returns [`EpubVersion::Unknown`]
///   with a value of `0.0`.
///
/// # Examples
/// - Creating a hierarchy and inserting it into the TOC:
/// ```
/// # use rbook::ebook::epub::toc::DetachedEpubTocEntry;
/// # use rbook::ebook::toc::TocEntryKind;
/// # use rbook::Epub;
/// # fn main() -> rbook::ebook::errors::EbookResult<()> {
/// let mut epub = Epub::open("tests/ebooks/example_epub")?;
///
/// // ...Insert volume 2 data into the manifest/spine...
///
/// let mut toc = epub.toc_mut();
///
/// // Create a "Volume" entry with nested chapters
/// let volume_2 = DetachedEpubTocEntry::new("Volume 2").children([
///      DetachedEpubTocEntry::new("Chapter 1").href("v2_c1.xhtml"),
///      DetachedEpubTocEntry::new("Chapter 2").href("v2_c2.xhtml"),
/// ]);
///
/// // Insert into the main ToC
/// if let Some(mut root) = toc.contents_mut() {
///     root.push(volume_2);
/// }
/// # Ok(())
/// # }
/// ```
#[derive(Clone, Debug, PartialEq)]
pub struct DetachedEpubTocEntry {
    version: EpubVersion,
    data: EpubTocEntryData,
}

impl DetachedEpubTocEntry {
    fn detached(version: EpubVersion, data: EpubTocEntryData) -> Self {
        Self { version, data }
    }

    /// Creates a new ToC entry with the given user-readable [`label`](Self::label).
    pub fn new(label: impl Into<String>) -> Self {
        Self {
            // Since the entry is detached, the associated version information is unknown.
            version: EpubVersion::Unknown(Version(0, 0)),
            data: EpubTocEntryData {
                label: label.into(),
                ..EpubTocEntryData::default()
            },
        }
    }

    /// Returns a mutable view to modify an entry's data,
    /// useful for modifications without builder-esque methods.
    pub fn as_mut(&mut self) -> EpubTocEntryMut<'_> {
        EpubTocContext::EMPTY.create_root_mut(self.version, None, &mut self.data)
    }

    /// Returns a read-only view, useful for inspecting state before applying modifications.
    pub fn as_view(&self) -> EpubTocEntry<'_> {
        EpubTocContext::EMPTY.create_root(self.version, &self.data)
    }

    /// Sets the unique `id`.
    ///
    /// # Uniqueness
    /// IDs must be unique within the scope of their respective documents
    /// (e.g., `.ncx`, `.opf`, `.xhtml`).
    /// Duplicate IDs will result in invalid XML and behavior is **undefined**
    /// for reading systems.
    pub fn id(mut self, id: impl IntoOption<String>) -> Self {
        self.as_mut().set_id(id);
        self
    }

    /// Sets the user-readable label (e.g., Volume 6, Chapter 1539).
    ///
    /// The label is stored as plain text (e.g. `"1 < 2 & 3"`)
    /// and is XML-escaped automatically during [writing](crate::epub::Epub::write).
    pub fn label(mut self, label: impl Into<String>) -> Self {
        self.as_mut().set_label(label);
        self
    }

    /// Sets the semantic kind of content an entry points to.
    ///
    /// For EPUB 3, this maps to the `epub:type` attribute.
    /// Providing the kind categorizes an entry,
    /// which may influence the behavior of reading systems.
    ///
    /// # See Also
    /// - [`TocEntryKind`] to view all variants.
    ///
    /// # Examples
    /// - Categorizing ToC entries:
    /// ```
    /// # use rbook::ebook::toc::TocEntryKind;
    /// # use rbook::epub::toc::DetachedEpubTocEntry;
    /// let toc = DetachedEpubTocEntry::new("Table of Contents").children([
    ///     // Cover Page
    ///     DetachedEpubTocEntry::new("Cover")
    ///         .kind(TocEntryKind::Cover)
    ///         .href("cover.xhtml"),
    ///     // Prologue
    ///     DetachedEpubTocEntry::new("Prologue")
    ///         .kind(TocEntryKind::Prologue)
    ///         .href("prologue.xhtml"),
    ///     // Chapter
    ///     DetachedEpubTocEntry::new("Chapter I")
    ///         .kind(TocEntryKind::Chapter)
    ///         .href("c1.xhtml"),
    /// ]);
    /// ```
    pub fn kind(mut self, kind: impl IntoOption<String>) -> Self {
        self.as_mut().set_kind(kind);
        self
    }

    /// Sets the location (href) where an entry points to.
    ///
    /// If [`Some`], this *should* point to the path of a
    /// resource in the manifest (e.g., `chapters/c1.xhtml`).
    ///
    /// **This method sets the value of the [`EpubTocEntry::href_raw`] field.**
    ///
    /// # Percent Encoding
    /// The given `raw_href` is expected to already be percent encoded.
    ///
    /// # Grouping Headers
    /// If `href` is set to [`None`], the entry acts as a **Grouping Header**
    /// (a label without a link).
    /// This is useful for organizing the ToC (e.g., "Part 1", "Volume 2")
    /// without pointing to a specific resource.
    ///
    /// # Fragments
    /// ToC entries can point to a specific section of a document by
    /// appending a fragment identifier (e.g., `c1.xhtml#section-2`).
    ///
    /// # Note
    /// - This method does not check href validity.
    /// - Calling this method will set [`EpubTocEntry::href`] to [`None`], as there is no
    ///   [package directory](crate::epub::package::EpubPackage::directory) to resolve against
    ///   for a detached entry.
    ///   The resolved href is computed once the entry is inserted into an [`Epub`](crate::epub::Epub).
    ///
    /// # Examples
    /// - Setting hrefs:
    /// ```
    /// # use rbook::ebook::toc::TocEntryKind;
    /// # use rbook::epub::toc::DetachedEpubTocEntry;
    /// let toc = DetachedEpubTocEntry::new("Table of Contents").children([
    ///     // Resource Location
    ///     DetachedEpubTocEntry::new("Chapter 1").href("c1.xhtml"),
    ///     // Resource Location + Fragment
    ///     DetachedEpubTocEntry::new("Chapter 1.a").href("c1.xhtml#section-a"),
    /// ]);
    /// ```
    pub fn href(mut self, raw_href: impl IntoOption<String>) -> Self {
        self.as_mut().set_href(raw_href);
        self
    }

    /// Inserts one or more XML attributes (e.g., `class`, `hidden`) via the [`Many`] trait.
    ///
    /// # Omitted Attributes
    /// The following attributes **should not** be set via this method
    /// as they have dedicated setters.
    /// If set here, they are ignored during [writing](crate::epub::Epub::write):
    /// - [`id`](Self::id)
    /// - [`href`](Self::href)
    /// - [`epub:type`](Self::kind)
    /// - [`src`](Self::href) (EPUB 2; legacy)
    /// - `playOrder` (Managed implicitly by the structure)
    ///
    /// # See Also
    /// - [`EpubTocEntryMut::attributes_mut`] for a modifiable collection of attributes
    ///   through [`Self::as_mut`].
    pub fn attribute(mut self, attribute: impl Many<Attribute>) -> Self {
        self.as_mut().attributes_mut().extend(attribute.iter_many());
        self
    }

    /// Appends one or more children to this entry via the [`Many`] trait.
    ///
    /// # See Also
    /// - [`EpubTocEntryMut`] for a modifiable collection of children
    ///   through [`Self::as_mut`].
    ///
    /// # Examples
    /// - Nesting children:
    /// ```
    /// # use rbook::ebook::toc::TocEntryKind;
    /// # use rbook::epub::toc::DetachedEpubTocEntry;
    /// let chapter_1 = DetachedEpubTocEntry::new("Chapter 1")
    ///     .href("c1.xhtml")
    ///     .children([
    ///         // Batch insertion
    ///         DetachedEpubTocEntry::new("Chapter 1.1")
    ///             .href("c1_1.xhtml"),
    ///         DetachedEpubTocEntry::new("Chapter 1.2")
    ///             .href("c1_2.xhtml"),
    ///     ]);
    ///
    /// let chapter_2 = DetachedEpubTocEntry::new("Chapter 2")
    ///     .href("c2.xhtml")
    ///     .children(
    ///         // Single Item Insertion
    ///         DetachedEpubTocEntry::new("Chapter 2.1")
    ///             .href("c2_1.xhtml"),
    ///     );
    /// ```
    pub fn children(mut self, child: impl Many<DetachedEpubTocEntry>) -> Self {
        self.as_mut().push(child);
        self
    }
}

/// Mutable view of [`EpubToc`] accessible via
/// [`Epub::toc_mut`](crate::epub::Epub::toc_mut).
///
/// # Note
/// If an [`Epub`](crate::epub::Epub) was loaded from an open operation
/// (e.g. [`EpubOpenOptions`](crate::epub::EpubOpenOptions)),
/// all toc information is not loaded.
/// For more details see:
/// - [`EpubOpenOptions::retain_variants`](crate::epub::EpubOpenOptions::retain_variants),
/// - [`EpubOpenOptions::preferred_toc`](crate::epub::EpubOpenOptions::preferred_toc),
///
/// # See Also
/// - [`EpubEditor`](crate::epub::EpubEditor) for simple modification tasks.
pub struct EpubTocMut<'ebook> {
    ctx: EpubTocContext<'ebook>,
    href_resolver: UriResolver<'ebook>,
    toc: &'ebook mut EpubTocData,
}

impl<'ebook> EpubTocMut<'ebook> {
    pub(in crate::ebook::epub) fn new(
        ctx: EpubTocContext<'ebook>,
        href_resolver: UriResolver<'ebook>,
        toc: &'ebook mut EpubTocData,
    ) -> Self {
        Self {
            ctx,
            toc,
            href_resolver,
        }
    }

    fn by_toc_key(&mut self, kind: &str, version: EpubVersion) -> Option<EpubTocEntryMut<'_>> {
        self.toc.entries.get_mut(&(kind, version)).map(|data| {
            self.ctx
                .create_root_mut(version, Some(self.href_resolver), data)
        })
    }

    fn remove_by_toc_key<'a>(
        &mut self,
        kind: impl Into<TocEntryKind<'a>>,
        version: EpubVersion,
    ) -> Option<DetachedEpubTocEntry> {
        let kind = kind.into();

        self.toc
            .entries
            .shift_remove(&(kind.as_str(), version))
            .map(|data| DetachedEpubTocEntry::detached(version, data))
    }

    //////////////////////////////////
    // PUBLIC API
    //////////////////////////////////

    /// Returns the preferred **table of contents** root entry.
    ///
    /// This maps to:
    /// - **EPUB 3:** XHTML `nav` where `epub:type` is `toc`.
    /// - **EPUB 2:** NCX `navMap`.
    ///
    /// # Note
    /// This method is equivalent to calling [`EpubTocMut::by_kind_mut`]
    /// with [`TocEntryKind::Toc`] as the argument.
    ///
    /// # See Also
    /// - **[`Self::by_kind_mut`] to see selection and fallback behavior, which this method uses.*
    /// - [`Self::by_kind_version_mut`] to retrieve a specific format
    ///   (e.g. explicitly editing the NCX).
    pub fn contents_mut(&mut self) -> Option<EpubTocEntryMut<'_>> {
        self.by_kind_mut(TocEntryKind::Toc)
    }

    /// Returns the preferred **guide/landmarks** root entry.
    ///
    /// This maps to:
    /// - **EPUB 3:** XHTML `nav` where `epub:type` is `landmarks`.
    /// - **EPUB 2:** OPF `guide`.
    ///
    /// # Note
    /// This method is equivalent to calling [`EpubTocMut::by_kind_mut`]
    /// with [`TocEntryKind::Landmarks`] as the argument.
    ///
    /// # See Also
    /// - **[`Self::by_kind_mut`] to see selection and fallback behavior, which this method uses.*
    pub fn landmarks_mut(&mut self) -> Option<EpubTocEntryMut<'_>> {
        self.by_kind_mut(TocEntryKind::Landmarks)
    }

    /// Returns the preferred **page list** root entry.
    ///
    /// This maps to:
    /// - **EPUB 3:** XHTML `nav` where `epub:type` is `page-list`.
    /// - **EPUB 2:** NCX `pageList`.
    ///
    /// # Note
    /// This method is equivalent to calling [`EpubTocMut::by_kind_mut`]
    /// with [`TocEntryKind::PageList`] as the argument.
    ///
    /// # See Also
    /// - **[`Self::by_kind_mut`] to see selection and fallback behavior, which this method uses.*
    pub fn page_list_mut(&mut self) -> Option<EpubTocEntryMut<'_>> {
        self.by_kind_mut(TocEntryKind::PageList)
    }

    /// Returns the root entry associated with the given `kind` and `version`, if present.
    ///
    /// Example mappings:
    /// - [`TocEntryKind::Landmarks`] + [`EpubVersion::Epub2`] = Legacy EPUB 2 guide.
    /// - [`TocEntryKind::Landmarks`] + [`EpubVersion::Epub3`] = EPUB 3 XHTML landmarks.
    /// - [`TocEntryKind::PageList`] + [`EpubVersion::Epub2`] = Legacy EPUB 2 NCX page list.
    /// - [`TocEntryKind::PageList`] + [`EpubVersion::Epub3`] = EPUB 3 XHTML page list.
    pub fn by_kind_version_mut<'a>(
        &mut self,
        kind: impl Into<TocEntryKind<'a>>,
        version: EpubVersion,
    ) -> Option<EpubTocEntryMut<'_>> {
        let kind = kind.into();
        self.by_toc_key(kind.as_str(), version.as_major())
    }

    // NOTE: This doc is nearly identical to EpubToc::by_kind
    /// Returns the root entry associated with the given `kind` and preferred variant.
    ///
    /// The specific variant returned (EPUB 3 or EPUB 2 NCX) depends on:
    /// 1. Which variants an [`Epub`](crate::epub::Epub) contains when opened, as dictated by
    ///    [`EpubOpenOptions`](crate::epub::EpubOpenOptions).
    /// 2. Preferences such as
    ///    [`EpubOpenOptions::preferred_toc`](crate::epub::EpubOpenOptions::preferred_toc).
    ///
    ///    If an [`Epub`](crate::epub::Epub) was created in-memory via
    ///    [`new`](crate::epub::Epub::new) or [`builder`](crate::epub::Epub::builder),
    ///    all preferences are set to [`EpubVersion::EPUB3`].
    ///
    /// If the preferred variant is not present, the other variant
    /// (EPUB 3 or EPUB 2 NCX) is returned instead.
    /// If neither variant exists, [`None`] is returned.
    ///
    /// # See Also
    /// - [`Self::by_kind_version_mut`]
    ///   to retrieve a specific root entry without any fallback behavior.
    pub fn by_kind_mut<'a>(
        &mut self,
        kind: impl Into<TocEntryKind<'a>>,
    ) -> Option<EpubTocEntryMut<'_>> {
        let kind = kind.into();
        let preferred_version = self.toc.get_preferred_version(kind);
        let attempts = std::iter::once(preferred_version)
            // If preferred version isn't available, try all standard versions
            // Note: If the preferred version is EPUB2/3, it is also included in `VERSIONS`.
            //       Despite the redundancy, the cost is negligible.
            .chain(EpubVersion::VERSIONS);

        // Mutable key for quick lookup & modification
        let mut key = (kind.as_str(), preferred_version);

        for version in attempts {
            key.1 = version;

            // Calling `self.by_toc_key` here causes a compilation error here
            // due to an invalid mutable borrow during iteration.
            // To avoid such, break with the valid key, then lookup.
            if self.toc.entries.contains_key(&key) {
                break;
            }
        }
        self.by_toc_key(key.0, key.1)
    }

    /// Inserts the provided root using the given `kind` and `version`.
    ///
    /// If a root with the same `kind` and `version` already exists,
    /// it is replaced and returned.
    ///
    /// # Href Normalization
    /// Upon insertion, [`EpubTocEntry::href`] is re-calculated recursively
    /// using [`EpubPackage::directory`](crate::epub::package::EpubPackage::directory)
    /// in the given hierarchy.
    ///
    /// For example, if the package directory is `/OEBPS`, then
    /// the href `text/c1.xhtml` is resolved to `/OEBPS/text/c1.xhtml`.
    pub fn insert_root<'a>(
        &mut self,
        kind: impl Into<TocEntryKind<'a>>,
        version: impl Into<EpubVersion>,
        detached: DetachedEpubTocEntry,
    ) -> Option<DetachedEpubTocEntry> {
        let kind = kind.into().to_string();
        let version = version.into().as_major();
        let key = EpubTocKey::new(kind.clone(), version);
        let mut root = detached.data;

        root.kind = Some(kind);
        root.resolve_hrefs(self.href_resolver);

        self.toc
            .entries
            .insert(key, root)
            .map(|root| DetachedEpubTocEntry::detached(version, root))
    }

    /// Returns an iterator over all root entries.
    ///
    /// # Roots Only
    /// The returned iterator only yields roots; not their children.
    /// To iterate over their children, call [`EpubTocEntryMut::iter_mut`] on yielded entries.
    pub fn iter_mut(&mut self) -> EpubTocIterMut<'_> {
        EpubTocIterMut {
            ctx: self.ctx,
            href_resolver: self.href_resolver,
            iter: self.toc.entries.iter_mut(),
        }
    }

    /// Removes and returns the root entry associated with the given `kind` and `version`,
    /// if present.
    ///
    /// # See Also
    /// - [`Self::retain`]/[`Self::extract_if`] to remove all entries by kind,
    ///   regardless of the version.
    pub fn remove_by_kind_version<'a>(
        &mut self,
        kind: impl Into<TocEntryKind<'a>>,
        version: EpubVersion,
    ) -> Option<DetachedEpubTocEntry> {
        self.remove_by_toc_key(kind.into(), version)
    }

    /// Retains only the root entries specified by the predicate.
    ///
    /// If the closure returns `false`, the root is retained.
    /// Otherwise, the root is removed.
    ///
    /// This method operates in place and visits every root exactly once.
    ///
    /// # See Also
    /// - [`Self::extract_if`] to retrieve an iterator of the removed roots.
    pub fn retain(&mut self, mut f: impl FnMut(EpubTocEntry) -> bool) {
        self.toc
            .entries
            .retain(|key, entry| f(self.ctx.create_root(key.version, entry)))
    }

    /// Removes and returns only the root entries specified by the predicate.
    ///
    /// If the closure returns `true`, the root is removed and yielded.
    /// Otherwise, the root is retained.
    ///
    /// # Drop
    /// If the returned iterator is not exhausted,
    /// (e.g. dropped without iterating or iteration short-circuits),
    /// then the remaining root entries are retained.
    ///
    /// Prefer [`Self::retain`] with a negated predicate if the returned iterator is not needed.
    ///
    /// # Examples
    /// - Extracting all EPUB 2-specific root entries:
    /// ```
    /// # use rbook::Epub;
    /// # fn main() -> rbook::ebook::errors::EbookResult<()> {
    /// let mut epub = Epub::open("tests/ebooks/example_epub")?;
    ///
    /// let epub2_roots: Vec<_> = epub.toc_mut()
    ///     .extract_if(|root| root.version().is_epub2())
    ///     .collect();
    /// # Ok(())
    /// # }
    /// ```
    pub fn extract_if(
        &mut self,
        mut f: impl FnMut(EpubTocEntry) -> bool,
    ) -> impl Iterator<Item = DetachedEpubTocEntry> {
        let ctx = self.ctx;

        self.toc
            .entries
            .extract_if(.., move |key, data| f(ctx.create_root(key.version, data)))
            .map(|(key, data)| DetachedEpubTocEntry::detached(key.version, data))
    }

    /// Removes and returns all root entries within the given `range`.
    pub fn drain(&mut self) -> impl Iterator<Item = DetachedEpubTocEntry> {
        self.toc
            .entries
            .drain(..)
            .map(|(key, data)| DetachedEpubTocEntry::detached(key.version, data))
    }

    /// Removes all root entries.
    ///
    /// # See Also
    /// - [`Self::drain`] to retrieve an iterator of the removed roots.
    pub fn clear(&mut self) {
        self.toc.entries.clear();
    }

    /// Returns a read-only view, useful for inspecting state before applying modifications.
    pub fn as_view(&mut self) -> EpubToc<'_> {
        self.ctx.create(self.toc)
    }
}

impl Debug for EpubTocMut<'_> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("EpubTocMut")
            .field("href_resolver", &self.href_resolver)
            .field("toc", &self.toc)
            .finish_non_exhaustive()
    }
}

impl<'a, 'ebook: 'a> IntoIterator for &'a mut EpubTocMut<'ebook> {
    type Item = EpubTocEntryMut<'a>;
    type IntoIter = EpubTocIterMut<'a>;

    fn into_iter(self) -> Self::IntoIter {
        self.iter_mut()
    }
}

impl<'ebook> IntoIterator for EpubTocMut<'ebook> {
    type Item = EpubTocEntryMut<'ebook>;
    type IntoIter = EpubTocIterMut<'ebook>;

    fn into_iter(self) -> Self::IntoIter {
        EpubTocIterMut {
            ctx: self.ctx,
            href_resolver: self.href_resolver,
            iter: self.toc.entries.iter_mut(),
        }
    }
}

/// An iterator over all mutable ToC roots contained within [`EpubTocMut`].
///
/// # See Also
/// - [`EpubTocMut::iter_mut`] to create an instance of this struct.
pub struct EpubTocIterMut<'ebook> {
    ctx: EpubTocContext<'ebook>,
    href_resolver: UriResolver<'ebook>,
    iter: indexmap::map::IterMut<'ebook, EpubTocKey, EpubTocEntryData>,
}

impl Debug for EpubTocIterMut<'_> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("EpubTocIterMut")
            .field("href_resolver", &self.href_resolver)
            .field("iter", &self.iter)
            .finish_non_exhaustive()
    }
}

impl<'ebook> Iterator for EpubTocIterMut<'ebook> {
    type Item = EpubTocEntryMut<'ebook>;

    fn next(&mut self) -> Option<Self::Item> {
        self.iter.next().map(|(key, root)| {
            self.ctx
                .create_root_mut(key.version, Some(self.href_resolver), root)
        })
    }
}

/// Mutable view of [`EpubTocEntry`], allowing modification of ToC entry fields,
/// attributes, and management of nested children.
///
/// # See Also
/// - [`DetachedEpubTocEntry`] for an owned ToC entry instance.
pub struct EpubTocEntryMut<'ebook> {
    ctx: EpubTocContext<'ebook>,
    version: EpubVersion,
    href_resolver: Option<UriResolver<'ebook>>,
    data: &'ebook mut EpubTocEntryData,
    depth: usize,
}

impl<'ebook> EpubTocEntryMut<'ebook> {
    fn new(
        ctx: EpubTocContext<'ebook>,
        version: EpubVersion,
        href_resolver: Option<UriResolver<'ebook>>,
        data: &'ebook mut EpubTocEntryData,
        depth: usize,
    ) -> Self {
        Self {
            ctx,
            version,
            href_resolver,
            data,
            depth,
        }
    }

    fn insert_detached(
        &mut self,
        index: usize,
        detached: impl Iterator<Item = DetachedEpubTocEntry>,
    ) {
        let children = &mut self.data.children;

        // Update each entry before insertion
        let mut detached = detached.map(|mut entry| {
            if let Some(resolver) = self.href_resolver {
                entry.data.resolve_hrefs(resolver);
            }
            entry
        });

        if detached.has_one_remaining()
            && let Some(entry) = detached.next()
        {
            children.insert(index, entry.data);
        } else {
            children.splice(index..index, detached.map(|e| e.data));
        }
    }

    /// Sets the unique `id` and returns the previous value.
    ///
    /// # See Also
    /// - [`DetachedEpubTocEntry::id`] for important details.
    pub fn set_id(&mut self, id: impl IntoOption<String>) -> Option<String> {
        std::mem::replace(&mut self.data.id, id.into_option())
    }

    /// Sets the user-readable label and returns the previous value.
    ///
    /// # See Also
    /// - [`DetachedEpubTocEntry::label`] for more details.
    pub fn set_label(&mut self, label: impl Into<String>) -> String {
        std::mem::replace(&mut self.data.label, label.into())
    }

    /// Sets the [raw](EpubTocEntry::kind_raw) semantic kind
    /// (e.g., chapter, epilogue) and returns the previous value.
    ///
    /// # Root Entry Note
    /// This method has no effect if the entry is **attached** to an [`Epub`](crate::epub::Epub)
    /// and is a [`root`](crate::ebook::toc::TocEntry::is_root) entry.
    /// This avoids inconsistent behavior and prevents desynchronizing
    /// the internal lookup keys from the entry’s actual kind.
    ///
    /// Note that root entries always have a [`kind`](EpubTocEntry::kind_raw) associated,
    /// and attempting to change it will always return [`None`].
    /// ```
    /// # use rbook::Epub;
    /// # use rbook::ebook::toc::TocEntryKind;
    /// # fn main() -> rbook::ebook::errors::EbookResult<()> {
    /// let mut epub = Epub::open("tests/ebooks/example_epub")?;
    /// let mut toc = epub.toc_mut();
    ///
    /// let mut landmarks = toc.by_kind_mut(TocEntryKind::Landmarks).unwrap();
    /// assert_eq!(TocEntryKind::Landmarks, landmarks.as_view().kind());
    /// // Setting the kind to `Appendix` is ignored
    /// assert_eq!(None, landmarks.set_kind(TocEntryKind::Appendix));
    /// // `None` is returned because the entry is a root attached to an Epub
    /// assert!(landmarks.as_view().is_root());
    ///
    /// // The underlying hashmap key and the root entry kind remain synchronized
    /// let mut landmarks = toc.by_kind_mut(TocEntryKind::Landmarks).unwrap();
    /// assert_eq!(TocEntryKind::Landmarks, landmarks.as_view().kind());
    /// # Ok(())
    /// # }
    /// ```
    ///
    /// # See Also
    /// - [`DetachedEpubTocEntry::kind`] for more details.
    pub fn set_kind(&mut self, kind: impl IntoOption<String>) -> Option<String> {
        // We can determine if the entry is attached by checking the resolver.
        // Detached entries do not have an href resolver.
        if self.depth == 0 && self.href_resolver.is_some() {
            None
        } else {
            std::mem::replace(&mut self.data.kind, kind.into_option())
        }
    }

    /// Sets the [raw href](EpubTocEntry::href_raw) and returns the previous raw value.
    ///
    /// The given raw href is ***expected*** to already be percent encoded.
    /// This method does **not** check for href uniqueness or validity.
    ///
    /// # Detached Entries
    /// Setting this field from a [`DetachedEpubTocEntry`] will
    /// set [`EpubTocEntry::href`] to [`None`], as there is no
    /// [package directory](crate::epub::package::EpubPackage::directory) to resolve against
    /// for a detached entry.
    /// The resolved href is computed once the entry is inserted into an [`Epub`](crate::epub::Epub).
    ///
    /// # See Also
    /// - [`DetachedEpubTocEntry::href`] for important details.
    /// - [`EpubEditor::resource`](crate::epub::EpubEditor::resource) for path details.
    ///   The same path resolution rules apply to this method.
    pub fn set_href(&mut self, raw_href: impl IntoOption<String>) -> Option<String> {
        let data = &mut self.data;

        if let Some(href_raw) = raw_href.into_option() {
            data.href = self
                .href_resolver
                .map(|resolver| resolver.resolve(&href_raw));
            data.href_raw.replace(href_raw)
        } else {
            data.href = None;
            data.href_raw.take()
        }
    }

    /// Mutable view of all additional `XML` attributes.
    ///
    /// Used for attributes like `class`, `hidden`, or custom namespaced attributes.
    ///
    /// # See Also
    /// - [`DetachedEpubTocEntry::attribute`] for important details.
    pub fn attributes_mut(&mut self) -> &mut Attributes {
        &mut self.data.attributes
    }

    /// Appends one or more children to the end via the [`Many`] trait.
    pub fn push(&mut self, detached: impl Many<DetachedEpubTocEntry>) {
        self.insert(self.data.children.len(), detached);
    }

    /// Inserts one or more children at the given `index` via the [`Many`] trait.
    ///
    /// # Panics
    /// Panics if the given `index` to insert at is greater than
    /// [`TocEntry::len`](crate::ebook::toc::TocEntry::len).
    pub fn insert(&mut self, index: usize, detached: impl Many<DetachedEpubTocEntry>) {
        self.insert_detached(index, detached.iter_many());
    }

    /// Removes and returns the entry at the given `index`.
    ///
    /// # Panics
    /// Panics if the given `index` is out of bounds
    /// (has a value greater than or equal to [`TocEntry::len`](crate::ebook::toc::TocEntry::len))
    pub fn get_mut(&mut self, index: usize) -> Option<EpubTocEntryMut<'_>> {
        self.data.children.get_mut(index).map(|data| {
            self.ctx
                .create_entry_mut(self.version, self.href_resolver, data, self.depth + 1)
        })
    }

    /// Returns an iterator over all direct children.
    ///
    /// # Nested Children
    /// The returned iterator is not recursive.
    /// To iterate over nested children, call [`Self::iter_mut`] on yielded entries.
    pub fn iter_mut(&mut self) -> EpubTocEntryIterMut<'_> {
        EpubTocEntryIterMut {
            ctx: self.ctx,
            version: self.version,
            href_resolver: self.href_resolver,
            next_depth: self.depth + 1,
            iter: self.data.children.iter_mut(),
        }
    }

    /// Removes and returns the entry at the given `index`.
    ///
    /// # Panics
    /// Panics if the given `index` is out of bounds
    /// (has a value greater than or equal to [`TocEntry::len`](crate::ebook::toc::TocEntry::len)).
    pub fn remove(&mut self, index: usize) -> DetachedEpubTocEntry {
        DetachedEpubTocEntry::detached(self.version, self.data.children.remove(index))
    }

    /// Retains only the children specified by the predicate.
    ///
    /// If the closure returns `false`, the entry is retained.
    /// Otherwise, the entry is removed.
    ///
    /// This method operates in place and visits every direct entry exactly once.
    ///
    /// # See Also
    /// - [`Self::extract_if`] to retrieve an iterator of the removed entries.
    pub fn retain(&mut self, mut f: impl FnMut(EpubTocEntry<'_>) -> bool) {
        self.data
            .children
            .retain(|child| f(self.ctx.create_entry(self.version, child, self.depth + 1)))
    }

    /// Removes and returns only the direct children specified by the predicate.
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
    pub fn extract_if(
        &mut self,
        mut f: impl FnMut(EpubTocEntry<'_>) -> bool,
    ) -> impl Iterator<Item = DetachedEpubTocEntry> {
        let ctx = self.ctx;
        let version = self.version;
        let next_depth = self.depth + 1;

        self.data
            .children
            .extract_if(.., move |child| {
                f(ctx.create_entry(version, child, next_depth))
            })
            .map(|data| DetachedEpubTocEntry::detached(self.version, data))
    }

    /// Removes and returns all direct children within the given `range`.
    ///
    /// # Panics
    /// For the given `range`, this method panics if:
    /// - The starting point is greater than the end point.
    /// - The end point is greater than [`TocEntry::len`](crate::ebook::toc::TocEntry::len).
    pub fn drain(
        &mut self,
        range: impl std::ops::RangeBounds<usize>,
    ) -> impl Iterator<Item = DetachedEpubTocEntry> {
        self.data
            .children
            .drain(range)
            .map(|data| DetachedEpubTocEntry::detached(self.version, data))
    }

    /// Removes all direct children.
    ///
    /// # See Also
    /// - [`Self::drain`] to retrieve an iterator of the removed entries.
    pub fn clear(&mut self) {
        self.data.children.clear();
    }

    /// Returns a read-only view, useful for inspecting state before applying modifications.
    pub fn as_view(&self) -> EpubTocEntry<'_> {
        self.ctx.create_entry(self.version, self.data, self.depth)
    }
}

impl Debug for EpubTocEntryMut<'_> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("EpubTocEntryMut")
            .field("href_resolver", &self.href_resolver)
            .field("version", &self.version)
            .field("depth", &self.depth)
            .field("data", &self.data)
            .finish_non_exhaustive()
    }
}

impl Extend<DetachedEpubTocEntry> for EpubTocEntryMut<'_> {
    fn extend<T: IntoIterator<Item = DetachedEpubTocEntry>>(&mut self, iter: T) {
        self.data.children.extend(iter.into_iter().map(|e| e.data));
    }
}

impl<'a, 'ebook: 'a> IntoIterator for &'a mut EpubTocEntryMut<'ebook> {
    type Item = EpubTocEntryMut<'a>;
    type IntoIter = EpubTocEntryIterMut<'a>;

    fn into_iter(self) -> Self::IntoIter {
        self.iter_mut()
    }
}

impl<'ebook> IntoIterator for EpubTocEntryMut<'ebook> {
    type Item = EpubTocEntryMut<'ebook>;
    type IntoIter = EpubTocEntryIterMut<'ebook>;

    fn into_iter(self) -> Self::IntoIter {
        EpubTocEntryIterMut {
            ctx: self.ctx,
            version: self.version,
            href_resolver: self.href_resolver,
            next_depth: self.depth + 1,
            iter: self.data.children.iter_mut(),
        }
    }
}

/// An iterator over the direct mutable [`children`](EpubTocEntryMut) of an [`EpubTocEntryMut`].
///
/// # See Also
/// - [`EpubTocEntryMut::iter_mut`] to create an instance of this struct.
pub struct EpubTocEntryIterMut<'ebook> {
    ctx: EpubTocContext<'ebook>,
    version: EpubVersion,
    href_resolver: Option<UriResolver<'ebook>>,
    next_depth: usize,
    iter: std::slice::IterMut<'ebook, EpubTocEntryData>,
}

impl<'ebook> Iterator for EpubTocEntryIterMut<'ebook> {
    type Item = EpubTocEntryMut<'ebook>;

    fn next(&mut self) -> Option<Self::Item> {
        self.iter.next().map(|entry| {
            self.ctx
                .create_entry_mut(self.version, self.href_resolver, entry, self.next_depth)
        })
    }
}

impl Debug for EpubTocEntryIterMut<'_> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("EpubTocEntryIterMut")
            .field("href_resolver", &self.href_resolver)
            .field("version", &self.version)
            .field("next_depth", &self.next_depth)
            .field("iter", &self.iter)
            .finish_non_exhaustive()
    }
}
