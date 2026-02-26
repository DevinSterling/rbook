use crate::ebook::element::{Attribute, Attributes, Properties};
use crate::ebook::epub::manifest::EpubManifestContext;
use crate::ebook::epub::metadata::{DetachedEpubMetaEntry, EpubRefinementsMut};
use crate::ebook::epub::package::EpubPackageMetaContext;
use crate::ebook::epub::spine::{
    EpubSpine, EpubSpineContext, EpubSpineData, EpubSpineEntry, EpubSpineEntryData,
};
use crate::ebook::spine::PageDirection;
use crate::input::{IntoOption, Many};
use crate::util::iter::IteratorExt;
use std::fmt::Debug;

////////////////////////////////////////////////////////////////////////////////
// PRIVATE API
////////////////////////////////////////////////////////////////////////////////

impl<'ebook> EpubSpineContext<'ebook> {
    const EMPTY: EpubSpineContext<'static> = EpubSpineContext {
        manifest_ctx: EpubManifestContext::EMPTY,
        meta_ctx: EpubPackageMetaContext::EMPTY,
    };

    fn create(self, data: &'ebook EpubSpineData) -> EpubSpine<'ebook> {
        EpubSpine::new(self.manifest_ctx, self.meta_ctx, data)
    }

    fn create_entry_mut(
        self,
        data: &'ebook mut EpubSpineEntryData,
        index: usize,
    ) -> EpubSpineEntryMut<'ebook> {
        EpubSpineEntryMut::new(self, data, index)
    }
}

////////////////////////////////////////////////////////////////////////////////
// PUBLIC API
////////////////////////////////////////////////////////////////////////////////

impl EpubSpineEntry<'_> {
    /// Creates an owned detached spine entry by cloning.
    ///
    /// # Note
    /// If the source spine entry has an `id`, the detached entry will retain it.
    /// To avoid ID collisions if re-inserting into the same [`Epub`](crate::epub::Epub),
    /// consider clearing or changing the ID using
    /// [`DetachedEpubSpineEntry::id`] or [`EpubSpineEntryMut::set_id`].
    ///
    /// # See Also
    /// - [`EpubSpineMut`] to insert detached entries into or remove entries without cloning.
    ///
    /// # Examples
    /// - Cloning all spine entries:
    /// ```
    /// # use rbook::Epub;
    /// # fn main() -> rbook::ebook::errors::EbookResult<()> {
    /// let mut epub = Epub::open("tests/ebooks/example_epub")?;
    ///
    /// let spine = epub.spine();
    /// assert_eq!(5, spine.len());
    ///
    /// // Cloning all entries
    /// let detached: Vec<_> = spine
    ///     .iter()
    ///     .map(|entry| entry.to_detached())
    ///     .collect();
    ///
    /// drop(epub);
    ///
    /// // Detached spine entries are accessible even after `epub` is dropped:
    /// assert_eq!(5, detached.len());
    /// # Ok(())
    /// # }
    /// ```
    pub fn to_detached(&self) -> DetachedEpubSpineEntry {
        DetachedEpubSpineEntry(self.data.clone())
    }
}

/// An owned [`EpubSpineEntry`] detached from an [`Epub`](crate::epub::Epub).
///
/// This struct acts as a builder for creating new spine entries
/// before insertion into [`EpubSpineMut`].
///
/// # Note
/// [`DetachedEpubSpineEntry`] instances always have an
/// [`order`](crate::ebook::spine::SpineEntry::order) of `0`.
/// Order is assigned once the entry is inserted into [`EpubSpineMut`].
///
/// # Examples
/// - Adding spine entries after inserting manifest entries:
/// ```
/// # use rbook::Epub;
/// # use rbook::ebook::epub::manifest::DetachedEpubManifestEntry;
/// # use rbook::ebook::epub::spine::DetachedEpubSpineEntry;
/// # fn main() -> rbook::ebook::errors::EbookResult<()> {
/// # const CHAPTER_3_XHTML_DATA: &[u8] = &[];
/// # const CHAPTER_4_XHTML_DATA: &[u8] = &[];
/// let mut epub = Epub::open("tests/ebooks/example_epub")?;
///
/// assert_eq!(5, epub.spine().len());
///
/// // Inserting the data of a new chapter
/// epub.manifest_mut().push([
///     DetachedEpubManifestEntry::new("ch_3")
///         .href("chapters/c3.xhtml")
///         .content(CHAPTER_3_XHTML_DATA),
///     DetachedEpubManifestEntry::new("ch_4")
///         .href("chapters/c4.xhtml")
///         .content(CHAPTER_4_XHTML_DATA),
/// ]);
///
/// // Inserting a reference to the chapter into the spine
/// epub.spine_mut().push(DetachedEpubSpineEntry::new("ch_3"));
/// // Or pass a string
/// epub.spine_mut().push("ch_4");
///
/// assert_eq!(7, epub.spine().len());
/// # Ok(())
/// # }
/// ```
#[derive(Clone, Debug, PartialEq)]
pub struct DetachedEpubSpineEntry(EpubSpineEntryData);

impl DetachedEpubSpineEntry {
    /// Creates a new spine entry (`<itemref>`) referencing the given manifest `id`.
    ///
    /// Upon insertion into an [`EpubSpine`], the given [`idref`](Self::idref) should
    /// match the [`id`](crate::epub::manifest::EpubManifestEntry::id) of an
    /// entry in the [`EpubManifest`](crate::epub::manifest::EpubManifest).
    pub fn new(idref: impl Into<String>) -> Self {
        Self(EpubSpineEntryData {
            idref: idref.into(),
            ..EpubSpineEntryData::default()
        })
        .linear(true)
    }

    /// Returns a mutable view to modify an entry's data,
    /// useful for modifications without builder-esque methods.
    pub fn as_mut(&mut self) -> EpubSpineEntryMut<'_> {
        EpubSpineContext::EMPTY.create_entry_mut(&mut self.0, 0)
    }

    /// Returns a read-only view, useful for inspecting state before applying modifications.
    pub fn as_view(&self) -> EpubSpineEntry<'_> {
        EpubSpineContext::EMPTY.create_entry(&self.0, 0)
    }

    /// Sets the unique `id`.
    ///
    /// # Uniqueness
    /// IDs must be unique within the entire package document (`.opf`).
    /// Duplicate IDs will result in invalid XML and behavior is undefined
    /// for reading systems.
    ///
    /// Ensure that IDs are unique across:
    /// - Spine entries
    /// - [Manifest entries](crate::epub::manifest::DetachedEpubManifestEntry::id)
    /// - [Metadata/Refinement entries](DetachedEpubMetaEntry::id)
    ///
    /// Other than the EPUB 2 guide,
    /// ToC entries ([`EpubTocEntry`](crate::epub::toc::EpubTocEntry)) are exempt
    /// from this restriction, as they reside in a separate file (`toc.ncx/xhtml`).
    ///
    /// # Refinements
    /// If the entry has refinements (children), their `refines` field
    /// are linked implicitly.
    ///
    /// # See Also
    /// - [`Self::idref`] to reference a specific manifest entry by ID.
    pub fn id(mut self, id: impl IntoOption<String>) -> Self {
        self.as_mut().set_id(id);
        self
    }

    /// Sets the `idref` (ID of the referenced manifest entry).
    ///
    /// The `idref` must match the [`id`](crate::epub::manifest::EpubManifestEntry::id) of an
    /// entry in the [`EpubManifest`](crate::epub::manifest::EpubManifest).
    /// This method does not validate if the given `idref` is a valid reference.
    ///
    /// This field determines what content is shown when a reader reaches this point in a book.
    pub fn idref(mut self, idref: impl Into<String>) -> Self {
        self.as_mut().set_idref(idref);
        self
    }

    /// Sets the linearity; whether the content is part of the linear reading order.
    ///
    /// If set to `true`, the entry is part of the default reading order
    /// (e.g., Chapter 1, Chapter 2).
    ///
    /// Otherwise, it is marked as supplementary content (e.g., Answer keys, Footnotes),
    /// which may be skipped or treated differently by applications.
    pub fn linear(mut self, linear: bool) -> Self {
        self.as_mut().set_linear(linear);
        self
    }

    /// Appends one or more properties (e.g., `page-spread-left`) via [`Properties::insert`].
    ///
    /// # Note
    /// This is an EPUB 3 feature.
    /// When [writing](crate::Epub::write) an EPUB 2 ebook, properties are ignored.
    ///
    /// # See Also
    /// - [`EpubSpineEntryMut::properties_mut`] for a modifiable collection of attributes
    ///   through [`Self::as_mut`].
    pub fn property(mut self, property: &str) -> Self {
        self.as_mut().properties_mut().insert(property);
        self
    }

    /// Inserts one or more XML attributes (e.g., `rendition:orientation`)
    /// via the [`Many`] trait.
    ///
    /// # Omitted Attributes
    /// The following attributes **should not** be set via this method
    /// as they have dedicated setters.
    /// If set here, they are ignored during [writing](crate::epub::Epub::write):
    /// - [`id`](Self::id)
    /// - [`idref`](Self::idref)
    /// - [`linear`](Self::linear)
    /// - [`properties`](Self::property)
    ///
    /// # See Also
    /// - [`EpubSpineEntryMut::attributes_mut`] for a modifiable collection of attributes
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
    /// - [`EpubSpineEntryMut::refinements_mut`] for a modifiable collection of refinements
    ///   through [`Self::as_mut`].
    pub fn refinement(mut self, detached: impl Many<DetachedEpubMetaEntry>) -> Self {
        self.as_mut().refinements_mut().push(detached);
        self
    }
}

impl From<&str> for DetachedEpubSpineEntry {
    fn from(value: &str) -> Self {
        Self::new(value.to_owned())
    }
}

impl From<String> for DetachedEpubSpineEntry {
    fn from(value: String) -> Self {
        Self::new(value)
    }
}

impl<'a> From<std::borrow::Cow<'a, str>> for DetachedEpubSpineEntry {
    fn from(value: std::borrow::Cow<'a, str>) -> Self {
        Self::new(value.into_owned())
    }
}

/// Mutable view of [`EpubSpine`] accessible via
/// [`Epub::spine_mut`](crate::epub::Epub::spine_mut).
///
/// Allows the management of reading order, including
/// adding, removing, and reordering of spine entries.
///
/// # See Also
/// - [`EpubEditor`](crate::epub::EpubEditor) for simple modification tasks.
pub struct EpubSpineMut<'ebook> {
    ctx: EpubSpineContext<'ebook>,
    spine: &'ebook mut EpubSpineData,
}

impl<'ebook> EpubSpineMut<'ebook> {
    pub(in crate::ebook::epub) fn new(
        ctx: EpubSpineContext<'ebook>,
        spine: &'ebook mut EpubSpineData,
    ) -> Self {
        Self { ctx, spine }
    }

    fn index_of(&mut self, predicate: impl Fn(&EpubSpineEntryData) -> bool) -> Option<usize> {
        self.spine.entries.iter().position(predicate)
    }

    fn insert_detached(
        &mut self,
        index: usize,
        mut detached: impl Iterator<Item = DetachedEpubSpineEntry>,
    ) {
        if detached.has_one_remaining_hint()
            && let Some(entry) = detached.next()
        {
            self.spine.entries.insert(index, entry.0);
        } else {
            self.spine
                .entries
                .splice(index..index, detached.map(|e| e.0));
        }
    }

    //////////////////////////////////
    // PUBLIC API
    //////////////////////////////////

    /// Sets the global page direction hint (e.g., `rtl` for Manga) and returns the previous value.
    ///
    /// This corresponds to the `page-progression-direction` attribute on the `<spine>` element.
    ///
    /// # Note
    /// This is an EPUB 3 feature.
    /// When [writing](crate::Epub::write) an EPUB 2 ebook, this field is ignored.
    pub fn set_page_direction(&mut self, page_direction: PageDirection) -> PageDirection {
        std::mem::replace(&mut self.spine.page_direction, page_direction)
    }

    /// Appends one or more entries to the end via the [`Many`] trait.
    ///
    /// # Examples
    /// - Appending spine entries:
    /// ```
    /// # use rbook::Epub;
    /// # use rbook::epub::spine::DetachedEpubSpineEntry;
    /// let mut epub = Epub::new();
    /// // ..Add resources..
    ///
    /// // ..Add spine entries..
    /// epub.spine_mut().push([
    ///     DetachedEpubSpineEntry::new("c1"),
    ///     DetachedEpubSpineEntry::new("c2"),
    ///     DetachedEpubSpineEntry::new("c3"),
    ///     DetachedEpubSpineEntry::new("c4"),
    ///     DetachedEpubSpineEntry::new("extra").linear(false),
    /// ]);
    ///
    /// assert_eq!(5, epub.spine().len());
    /// ```
    /// - Alternatively, strings can be given instead for simpler cases:
    /// ```
    /// # let mut epub = rbook::Epub::new();
    /// epub.spine_mut().push([
    ///     "c1",
    ///     "c2",
    ///     "c3",
    ///     "c4",
    /// ]);
    /// ```
    pub fn push(&mut self, detached: impl Many<DetachedEpubSpineEntry>) {
        self.insert(self.spine.entries.len(), detached);
    }

    /// Inserts one or more entries at the given `index` via the [`Many`] trait.
    ///
    /// # Panics
    /// Panics if the given `index` to insert at is greater than
    /// [`Spine::len`](crate::ebook::spine::Spine::len).
    pub fn insert(&mut self, index: usize, detached: impl Many<DetachedEpubSpineEntry>) {
        self.insert_detached(index, detached.iter_many());
    }

    /// Returns the associated entry if the given `index`
    /// is less than [`Spine::len`](crate::ebook::spine::Spine::len), otherwise [`None`].
    pub fn get_mut(&mut self, index: usize) -> Option<EpubSpineEntryMut<'_>> {
        self.spine
            .entries
            .get_mut(index)
            .map(|entry| self.ctx.create_entry_mut(entry, index))
    }

    /// Returns the associated entry matching the given `id`, or [`None`] if not found.
    pub fn by_id_mut(&mut self, id: &str) -> Option<EpubSpineEntryMut<'_>> {
        self.iter_mut()
            .find(|entry| entry.data.id.as_deref() == Some(id))
    }

    /// Returns an iterator over **all** mutable entries matching the given
    /// [`idref`](EpubSpineEntry::idref).
    ///
    /// Albeit uncommon, more than one spine entry can reference the same manifest entry.
    pub fn by_idref_mut(&mut self, idref: &str) -> impl Iterator<Item = EpubSpineEntryMut<'_>> {
        self.iter_mut()
            .filter(move |entry| entry.data.idref == idref)
    }

    /// Returns an iterator over **all** spine entries.
    pub fn iter_mut(&mut self) -> EpubSpineIterMut<'_> {
        EpubSpineIterMut {
            ctx: self.ctx,
            iter: self.spine.entries.iter_mut().enumerate(),
        }
    }

    /// Removes and returns the entry at the given `index`.
    ///
    /// # Panics
    /// Panics if the given `index` is out of bounds
    /// (has a value greater than or equal to [`Spine::len`](crate::ebook::spine::Spine::len))
    pub fn remove(&mut self, index: usize) -> DetachedEpubSpineEntry {
        DetachedEpubSpineEntry(self.spine.entries.remove(index))
    }

    /// Removes and returns the first entry matching the given `id`, if present.
    ///
    /// # Note
    /// This method refers to the **unique ID of a spine entry**,
    /// ***not*** the `idref` which points to an entry in the manifest.
    ///
    /// To remove spine entries based on the `idref`,
    /// see [`Self::extract_if`] or [`Self::retain`].
    pub fn remove_by_id(&mut self, id: &str) -> Option<DetachedEpubSpineEntry> {
        self.index_of(|e| e.id.as_deref() == Some(id))
            .map(|i| self.remove(i))
    }

    /// Retains only the entries specified by the predicate.
    ///
    /// If the closure returns `false`, the entry is retained.
    /// Otherwise, the entry is removed.
    ///
    /// This method operates in place and visits every entry exactly once.
    ///
    /// # See Also
    /// - [`Self::extract_if`] to retrieve an iterator of the removed entries.
    pub fn retain(&mut self, mut f: impl FnMut(EpubSpineEntry<'_>) -> bool) {
        let mut index = 0;

        self.spine.entries.retain(|entry| {
            let retain = f(self.ctx.create_entry(entry, index));
            // Increment the index for the next entry
            index += 1;
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
    /// # Examples
    /// - Extracting all non-linear entries:
    /// ```
    /// # use rbook::Epub;
    /// # fn main() -> rbook::ebook::errors::EbookResult<()> {
    /// let mut epub = Epub::open("tests/ebooks/example_epub")?;
    ///
    /// let non_linear: Vec<_> = epub.spine_mut()
    ///     .extract_if(|e| !e.is_linear())
    ///     .collect();
    /// # Ok(())
    /// # }
    /// ```
    pub fn extract_if(
        &mut self,
        mut f: impl FnMut(EpubSpineEntry<'_>) -> bool,
    ) -> impl Iterator<Item = DetachedEpubSpineEntry> {
        let ctx = self.ctx;
        let mut index = 0;

        self.spine
            .entries
            .extract_if(.., move |e| {
                let extract = f(ctx.create_entry(e, index));
                index += 1;
                extract
            })
            .map(DetachedEpubSpineEntry)
    }

    /// Removes and returns all spine entries within the given `range`.
    ///
    /// # Panics
    /// For the given `range`, this method panics if:
    /// - The starting point is greater than the end point.
    /// - The end point is greater than [`Spine::len`](crate::ebook::spine::Spine::len).
    pub fn drain(
        &mut self,
        range: impl std::ops::RangeBounds<usize>,
    ) -> impl Iterator<Item = DetachedEpubSpineEntry> {
        self.spine.entries.drain(range).map(DetachedEpubSpineEntry)
    }

    /// Removes all spine entries.
    ///
    /// # See Also
    /// - [`Self::drain`] to retrieve an iterator of the removed entries.
    pub fn clear(&mut self) {
        self.spine.entries.clear();
    }

    /// Returns a read-only view, useful for inspecting state before applying modifications.
    pub fn as_view(&self) -> EpubSpine<'_> {
        self.ctx.create(self.spine)
    }
}

impl Debug for EpubSpineMut<'_> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("EpubSpineMut")
            .field("spine", &self.spine)
            .finish_non_exhaustive()
    }
}

impl Extend<DetachedEpubSpineEntry> for EpubSpineMut<'_> {
    fn extend<T: IntoIterator<Item = DetachedEpubSpineEntry>>(&mut self, iter: T) {
        self.insert_detached(self.spine.entries.len(), iter.into_iter());
    }
}

impl<'a, 'ebook: 'a> IntoIterator for &'a mut EpubSpineMut<'ebook> {
    type Item = EpubSpineEntryMut<'a>;
    type IntoIter = EpubSpineIterMut<'a>;

    fn into_iter(self) -> Self::IntoIter {
        self.iter_mut()
    }
}

impl<'ebook> IntoIterator for EpubSpineMut<'ebook> {
    type Item = EpubSpineEntryMut<'ebook>;
    type IntoIter = EpubSpineIterMut<'ebook>;

    fn into_iter(self) -> Self::IntoIter {
        EpubSpineIterMut {
            ctx: self.ctx,
            iter: self.spine.entries.iter_mut().enumerate(),
        }
    }
}

/// An iterator over all the mutable
/// [entries](EpubSpineEntryMut) contained within [`EpubSpineMut`].
///
/// # See Also
/// - [`EpubSpineMut::iter_mut`] to create an instance of this struct.
pub struct EpubSpineIterMut<'ebook> {
    ctx: EpubSpineContext<'ebook>,
    iter: std::iter::Enumerate<std::slice::IterMut<'ebook, EpubSpineEntryData>>,
}

impl Debug for EpubSpineIterMut<'_> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("EpubSpineIterMut")
            .field("iter", &self.iter)
            .finish_non_exhaustive()
    }
}

impl<'ebook> Iterator for EpubSpineIterMut<'ebook> {
    type Item = EpubSpineEntryMut<'ebook>;

    fn next(&mut self) -> Option<Self::Item> {
        self.iter
            .next()
            .map(|(i, entry)| self.ctx.create_entry_mut(entry, i))
    }
}

/// Mutable view of [`EpubSpineEntry`], allowing modification of spine entry (`itemref`) fields,
/// attributes, and [refinements](Self::refinements_mut).
///
/// # See Also
/// - [`DetachedEpubSpineEntry`] for an owned spine entry instances.
pub struct EpubSpineEntryMut<'ebook> {
    ctx: EpubSpineContext<'ebook>,
    data: &'ebook mut EpubSpineEntryData,
    index: usize,
}

impl<'ebook> EpubSpineEntryMut<'ebook> {
    fn new(
        ctx: EpubSpineContext<'ebook>,
        data: &'ebook mut EpubSpineEntryData,
        index: usize,
    ) -> Self {
        Self { ctx, data, index }
    }

    /// Sets the unique `id` and returns the previous value.
    ///
    /// # See Also
    /// - [`DetachedEpubSpineEntry::id`] for important details.
    pub fn set_id(&mut self, id: impl IntoOption<String>) -> Option<String> {
        std::mem::replace(&mut self.data.id, id.into_option())
    }

    /// Sets the `idref` and returns the previous value.
    ///
    /// # Note
    /// Setting the `idref` does not update the [`id`](crate::epub::manifest::EpubManifestEntry::id)
    /// of a manifest entry. To cascade changes, see
    /// [`EpubManifestEntryMut::set_id`](crate::epub::manifest::EpubManifestEntryMut::set_id).
    ///
    /// # See Also
    /// - [`DetachedEpubSpineEntry::idref`] for important details.
    pub fn set_idref(&mut self, idref: impl Into<String>) -> String {
        std::mem::replace(&mut self.data.idref, idref.into())
    }

    /// Sets the linearity and returns the previous value.
    ///
    /// # See Also
    /// - [`DetachedEpubSpineEntry::linear`] for more details.
    pub fn set_linear(&mut self, linear: bool) -> bool {
        std::mem::replace(&mut self.data.linear, linear)
    }

    /// Mutable view of all properties (e.g., `page-spread-left`).
    pub fn properties_mut(&mut self) -> &mut Properties {
        &mut self.data.properties
    }

    /// Mutable view of all additional `XML` attributes.
    ///
    /// Used for attributes like `rendition:orientation` or custom namespaced attributes.
    ///
    /// # See Also
    /// - [`DetachedEpubSpineEntry::attribute`] for important details.
    pub fn attributes_mut(&mut self) -> &mut Attributes {
        &mut self.data.attributes
    }

    /// Mutable view of all direct refinements.
    ///
    /// # ID Generation
    /// If this parent entry does not have an ID
    /// ([`self.as_view().id()`](EpubSpineEntry::id)), a unique ID will be
    /// auto-generated during [writing](crate::epub::Epub::write).
    /// This ensures that all refinements correctly reference their parent.
    ///
    /// # Note
    /// If parent entries lack an ID, the [`refines`](crate::epub::metadata::EpubMetaEntry::refines)
    /// field of its refinements will return [`None`].
    ///
    /// # See Also
    /// - [`Self::set_id`] to set the ID and override ID generation.
    /// - [`DetachedEpubSpineEntry::refinement`]
    pub fn refinements_mut(&mut self) -> EpubRefinementsMut<'_> {
        EpubRefinementsMut::new(
            self.ctx.meta_ctx,
            self.data.id.as_deref(),
            &mut self.data.refinements,
        )
    }

    /// Returns a read-only view, useful for inspecting state before applying modifications.
    pub fn as_view(&self) -> EpubSpineEntry<'_> {
        self.ctx.create_entry(self.data, self.index)
    }
}

impl Debug for EpubSpineEntryMut<'_> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("EpubSpineEntryMut")
            .field("data", &self.data)
            .field("index", &self.index)
            .finish_non_exhaustive()
    }
}
