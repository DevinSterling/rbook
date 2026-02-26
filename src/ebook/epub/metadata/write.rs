use crate::ebook::element::{Attribute, Attributes, TextDirection};
use crate::ebook::epub::consts::{dc, marc, opf, xml};
use crate::ebook::epub::metadata::{
    EpubMetaEntry, EpubMetaEntryData, EpubMetaEntryKind, EpubMetadata, EpubMetadataData,
    EpubRefinements, EpubRefinementsData, EpubVersion, InnerMetadataIter,
};
use crate::ebook::epub::package::{EpubPackageData, EpubPackageMetaContext};
use crate::ebook::metadata::TitleKind;
use crate::ebook::metadata::datetime::{Date, DateTime};
use crate::input::{IntoOption, Many};
use crate::util::iter::IteratorExt;
use std::fmt::Debug;
use std::slice::IterMut as SliceIterMut;

////////////////////////////////////////////////////////////////////////////////
// PUBLIC API
////////////////////////////////////////////////////////////////////////////////

impl EpubMetaEntry<'_> {
    /// Creates an owned detached metadata entry by cloning.
    ///
    /// # Note
    /// If the source metadata entry has an `id`, the detached entry will retain it.
    /// To avoid ID collisions if re-inserting into the same [`Epub`](crate::epub::Epub),
    /// consider clearing or changing the ID using
    /// [`DetachedEpubMetaEntry::id`] or [`EpubMetaEntryMut::set_id`].
    ///
    /// # See Also
    /// - [`EpubMetadataMut`], [`EpubRefinementsMut`], or
    ///   [`EpubEditor::meta`](crate::epub::EpubEditor::meta)
    ///   to insert detached entries into or remove entries without cloning.
    ///
    /// # Examples
    /// - Cloning all title metadata entries:
    /// ```
    /// # use rbook::Epub;
    /// # fn main() -> rbook::ebook::errors::EbookResult<()> {
    /// let mut epub = Epub::open("tests/ebooks/example_epub")?;
    ///
    /// // Cloning all titles
    /// let detached: Vec<_> = epub
    ///     .metadata()
    ///     .titles()
    ///     .map(|entry| entry.to_detached())
    ///     .collect();
    ///
    /// drop(epub);
    ///
    /// // Detached metadata entries accessible even after `epub` is dropped:
    /// assert_eq!(3, detached.len());
    /// # Ok(())
    /// # }
    /// ```
    pub fn to_detached(&self) -> DetachedEpubMetaEntry {
        DetachedEpubMetaEntry(self.data.clone(), std::marker::PhantomData)
    }
}

/// Zero-sized marker types used to enforce compile-time safety for metadata entries.
///
/// Used as generic parameters for [`DetachedEpubMetaEntry`],
/// acting as "modes" that enable specific builder methods
/// based on the type of metadata being created.
///
/// For example:
/// - [`Contributor`](marker::Contributor) enables [`DetachedEpubMetaEntry::role`].
/// - [`Title`](marker::Title) enables [`DetachedEpubMetaEntry::kind`] (title-type).
/// - [`Unknown`](marker::Unknown) enables [`DetachedEpubMetaEntry::property`].
pub mod marker {
    macro_rules! markers {
        { $($(#[$attr:meta])* $name:ident,)* } => {
            $(
                $(#[$attr])*
                #[non_exhaustive]
                #[derive(Copy, Clone, Debug, Hash, PartialEq, Eq)]
                pub struct $name;
            )*
        };
    }

    markers! {
        /// Marker for **Identifier** entries (e.g., `dc:identifier`).
        ///
        /// # See Also:
        /// - [`DetachedEpubMetaEntry::identifier`](crate::epub::metadata::DetachedEpubMetaEntry::identifier)
        Identifier,

        /// Marker for **Title** entries (`dc:title`).
        ///
        /// # See Also:
        /// - [`DetachedEpubMetaEntry::title`](crate::epub::metadata::DetachedEpubMetaEntry::title)
        Title,

        /// Marker for **Creators, Contributors, and Publishers**.
        ///
        /// # See Also:
        /// - [`DetachedEpubMetaEntry::creator`](crate::epub::metadata::DetachedEpubMetaEntry::creator)
        /// - [`DetachedEpubMetaEntry::contributor`](crate::epub::metadata::DetachedEpubMetaEntry::contributor)
        /// - [`DetachedEpubMetaEntry::publisher`](crate::epub::metadata::DetachedEpubMetaEntry::publisher)
        Contributor,

        /// Marker for **Subject/Tag** entries (`dc:subject`).
        ///
        /// # See Also:
        /// - [`DetachedEpubMetaEntry::tag`](crate::epub::metadata::DetachedEpubMetaEntry::tag)
        Tag,

        /// Marker for **Description** entries (`dc:description`).
        ///
        /// # See Also:
        /// - [`DetachedEpubMetaEntry::description`](crate::epub::metadata::DetachedEpubMetaEntry::description)
        Description,

        /// Marker for **Language** entries (`dc:language`).
        ///
        /// # See Also:
        /// - [`DetachedEpubMetaEntry::language`](crate::epub::metadata::DetachedEpubMetaEntry::language)
        Language,

        /// Marker for **Date** entries (`dc:date`).
        ///
        /// # See Also:
        /// - [`DetachedEpubMetaEntry::date`](crate::epub::metadata::DetachedEpubMetaEntry::date)
        Date,

        /// Marker for **Rights** entries (`dc:rights`).
        ///
        /// # See Also:
        /// - [`DetachedEpubMetaEntry::rights`](crate::epub::metadata::DetachedEpubMetaEntry::rights)
        Rights,

        /// Marker for **Link** entries (`link`).
        ///
        /// # See Also:
        /// - [`DetachedEpubMetaEntry::link`](crate::epub::metadata::DetachedEpubMetaEntry::link)
        Link,

        /// The default **Type-Erased** marker.
        ///
        /// Allows access to
        /// [`DetachedEpubMetaEntry::property`](crate::epub::metadata::DetachedEpubMetaEntry::property).
        ///
        /// # See Also:
        /// - [`DetachedEpubMetaEntry::dublin_core`](crate::epub::metadata::DetachedEpubMetaEntry::dublin_core)
        /// - [`DetachedEpubMetaEntry::meta`](crate::epub::metadata::DetachedEpubMetaEntry::meta)
        /// - [`DetachedEpubMetaEntry::link`](crate::epub::metadata::DetachedEpubMetaEntry::link)
        Unknown,
    }
}

/// An owned [`EpubMetaEntry`] detached from an [`Epub`](crate::epub::Epub).
///
/// This struct acts as a builder for creating new metadata entries
/// (Titles, Creators, Links, etc.) before insertion into
/// [`EpubMetadataMut`] or [`EpubRefinementsMut`].
///
/// The use of [markers](marker) (`M`) enforce semantic correctness at compile time.
/// For example, a [`role`](DetachedEpubMetaEntry::role) can be added to a
/// [creator](DetachedEpubMetaEntry::creator), although not a [title](DetachedEpubMetaEntry::title).
///
/// # Note
/// - **Order:** [`DetachedEpubMetaEntry`] instances always have an
///   [`order`](crate::ebook::metadata::MetaEntry::order)/display sequence of `0`.
///   Order is assigned once the entry is inserted into
///   [`EpubMetadataMut`] or [`EpubRefinementsMut`].
/// - **Kind:** The [`EpubMetaEntryKind`] is determined at creation time via specific constructors:
///   - [`Self::dublin_core`]
///     - [`Self::creator`]
///     - [`Self::contributor`]
///     - [`Self::date`]
///     - [`Self::description`]
///     - [`Self::identifier`]
///     - [`Self::language`]
///     - [`Self::publisher`]
///     - [`Self::tag`]
///     - [`Self::title`]
///   - [`Self::link`]
///   - [`Self::meta`] (EPUB 3)
///   - [`Self::meta_name`] (EPUB 2)
///
/// # Examples
/// - Attaching a title to an [`Epub`](crate::epub::Epub):
/// ```
/// # use rbook::Epub;
/// # use rbook::epub::metadata::DetachedEpubMetaEntry;
/// let mut epub = Epub::new();
///
/// epub.metadata_mut().push(DetachedEpubMetaEntry::title("Adventure Time"));
///
/// let title = epub.metadata().title().unwrap();
/// assert_eq!("Adventure Time", title.value());
/// ```
/// - Creating a detailed author entry:
/// ```
/// # use rbook::epub::metadata::DetachedEpubMetaEntry;
/// let author = DetachedEpubMetaEntry::creator("Murakami, Haruki")
///     .file_as("Murakami, Haruki")
///     .alternate_script("ja", "村上春樹")
///     .role("aut");
/// ```
#[derive(Clone, Debug, PartialEq)]
pub struct DetachedEpubMetaEntry<M = marker::Unknown>(
    EpubMetaEntryData,
    std::marker::PhantomData<M>,
);

impl<M> DetachedEpubMetaEntry<M> {
    fn new(kind: EpubMetaEntryKind) -> Self {
        Self(
            EpubMetaEntryData {
                kind,
                ..EpubMetaEntryData::default()
            },
            std::marker::PhantomData,
        )
    }

    fn dc(property: impl Into<String>) -> Self {
        Self(
            DetachedEpubMetaEntry::dublin_core(property).0,
            std::marker::PhantomData,
        )
    }

    pub(crate) fn force_property(mut self, property: &str) -> Self {
        if self.0.property != property {
            self.0.property = property.to_owned();
        }
        self
    }

    pub(crate) fn force_kind(mut self, kind: EpubMetaEntryKind) -> Self {
        if self.0.kind != kind {
            self.0.kind = kind;
        }
        self
    }

    /// Erases the specific type information `M` into [`Unknown`](marker::Unknown),
    /// converting this into a generic entry.
    ///
    /// This is useful when storing mixed types in the same collection
    /// is required (e.g. `Vec<DetachedEpubMetaEntry>`).
    ///
    /// # Examples
    /// ```
    /// use rbook::epub::metadata::DetachedEpubMetaEntry;
    /// use rbook::epub::metadata::marker::Contributor;
    ///
    /// // Contains type information with access to additional methods:
    /// let contributor: DetachedEpubMetaEntry<Contributor> = DetachedEpubMetaEntry::contributor("John Doe");
    ///
    /// // Basic plain entry:
    /// let contributor: DetachedEpubMetaEntry = contributor.into_any();
    /// ```
    pub fn into_any(self) -> DetachedEpubMetaEntry {
        DetachedEpubMetaEntry(self.0, std::marker::PhantomData)
    }

    /// Returns a mutable view to modify an entry's data,
    /// useful for modifications without builder-esque methods.
    pub fn as_mut(&mut self) -> EpubMetaEntryMut<'_> {
        EpubMetaEntryMut::new(EpubPackageMetaContext::EMPTY, &mut self.0, None, 0)
    }

    /// Returns a read-only view, useful for inspecting state before applying modifications.
    pub fn as_view(&self) -> EpubMetaEntry<'_> {
        EpubPackageMetaContext::EMPTY.create_entry(&self.0, 0)
    }

    /// Sets the unique `id`.
    ///
    /// # Uniqueness
    /// IDs must be unique within the entire package document (`.opf`).
    /// Duplicate IDs result in invalid XML and behavior is **undefined**
    /// for reading systems.
    ///
    /// Ensure that IDs are unique across:
    /// - Metadata/Refinement entries
    /// - [Manifest entries](crate::epub::manifest::DetachedEpubManifestEntry::id)
    /// - [Spine entries](crate::epub::spine::DetachedEpubSpineEntry::id)
    ///
    /// Other than the EPUB 2 guide,
    /// ToC entries ([`EpubTocEntry`](crate::epub::toc::EpubTocEntry)) are exempt
    /// from this restriction, as they reside in a separate file (`toc.ncx/xhtml`).
    ///
    /// # Refinements
    /// If the entry has refinements (children), their `refines` field
    /// are linked implicitly.
    pub fn id(mut self, id: impl IntoOption<String>) -> Self {
        self.as_mut().set_id(id);
        self
    }

    /// Sets the primary text/value of an entry.
    ///
    /// # XML Escaping
    /// The entry value is stored as plain text (e.g. `"1 < 2 & 3"`)
    /// and is XML-escaped automatically during [writing](crate::epub::Epub::write).
    ///
    /// See the [epub](crate::epub) trait-level documentation for more details.
    ///
    /// # Value Mapping
    /// Depending on the underlying element type and EPUB version, this value is written to
    /// different parts of the XML structure:
    ///
    /// | Element Type         | Mapping                                                             |
    /// |----------------------|---------------------------------------------------------------------|
    /// | Dublin Core `<dc:*>` | Inner text (`<dc:title>value</dc:title>`)                           |
    /// | EPUB 2 `<meta>`      | `content` attribute (`<meta name="…" content="value" />`)           |
    /// | EPUB 3 `<meta>`      | Inner text (`<meta property="…">value</meta>`)                      |
    /// | Link `<link>`        | Effectively **ignored** (Link elements do not support text content) |
    pub fn value(mut self, value: impl Into<String>) -> Self {
        self.as_mut().set_value(value);
        self
    }

    /// Sets the language (`xml:lang`) associated with a metadata entry.
    ///
    /// The given `code` is not validated and ***should*** be in **BCP-47** format.
    pub fn xml_language(mut self, code: impl IntoOption<String>) -> Self {
        self.as_mut().set_xml_language(code);
        self
    }

    /// Sets the text direction (`dir`) associated with a metadata entry.
    ///
    /// # Note
    /// This is an EPUB 3 feature.
    /// When [writing](crate::Epub::write) an EPUB 2 ebook, this field is ignored.
    pub fn text_direction(mut self, direction: TextDirection) -> Self {
        self.as_mut().set_text_direction(direction);
        self
    }

    /// Inserts one or more XML attributes (e.g., `scheme`, `rel`, `href`)
    /// via the [`Many`] trait.
    ///
    /// # Omitted Attributes
    /// The following attributes **should not** be set via this method
    /// as they have dedicated setters.
    /// If set here, they are ignored during [writing](crate::epub::Epub::write):
    /// - [`id`](Self::id)
    /// - [`xml:lang`](Self::xml_language)
    /// - [`dir`](Self::text_direction)
    /// - [`property`](Self::property)
    /// - [`name`](Self::property) (EPUB 2; legacy)
    /// - [`content`](Self::value) (EPUB 2; legacy)
    /// - `refines` (Managed implicitly by the structure)
    ///
    /// # See Also
    /// - [`EpubMetaEntryMut::attributes_mut`] for a modifiable collection of attributes
    ///   through [`Self::as_mut`].
    ///
    /// # Examples
    /// - Setting the href for a link:
    /// ```
    /// # use rbook::epub::metadata::DetachedEpubMetaEntry;
    /// let link = DetachedEpubMetaEntry::link("stylesheet")
    ///     .attribute(("href", "style.css"));
    /// ```
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
    /// However, depending on the content, some refinements may be
    /// [downgraded](crate::epub::EpubWriteOptions::target).
    ///
    /// # See Also
    /// - If [`EpubWriteOptions::target`](crate::epub::EpubWriteOptions::target)
    ///   includes or the version is **EPUB 2**, certain refinements are automatically downgraded
    ///   for backwards compatibility when [writing](crate::Epub::write).
    /// - [`EpubMetaEntryMut::refinements_mut`] for a modifiable collection of refinements
    ///   through [`Self::as_mut`].
    ///
    /// # Examples
    /// - Adding a [`file-as`](Self::file_as) refinement to a Creator:
    /// ```
    /// # use rbook::epub::metadata::DetachedEpubMetaEntry;
    /// let author = DetachedEpubMetaEntry::dublin_core("dc:creator")
    ///     .value("John Doe")
    ///     .refinement(
    ///         DetachedEpubMetaEntry::meta("file-as").value("Doe, John")
    ///     );
    ///
    /// // Alternatively, the same process concisely via builder-esque methods:
    /// let author = DetachedEpubMetaEntry::creator("Jane Doe").file_as("Doe, Jane");
    /// ```
    pub fn refinement(mut self, detached: impl Many<DetachedEpubMetaEntry>) -> Self {
        self.as_mut().refinements_mut().push(detached);
        self
    }

    /// Adds a `file-as` [refinement](Self::refinement),
    /// defining how the value should be treated for **sorting**.
    ///
    /// # Examples
    /// - Specifying to sort `John Doe` as `Doe, John`:
    /// ```
    /// # use rbook::epub::metadata::DetachedEpubMetaEntry;
    /// DetachedEpubMetaEntry::creator("John Doe")
    ///     .file_as("Doe, John");
    /// ```
    pub fn file_as(mut self, file_as: impl Into<String>) -> Self {
        let mut entry = self.as_mut();
        let mut refinements = entry.refinements_mut();

        // There must only be *one* `identifier-type` refinement.
        refinements.retain(|r| r.property().as_str() != opf::FILE_AS);
        refinements.push(DetachedEpubMetaEntry::meta(opf::FILE_AS).value(file_as));

        self
    }

    /// Adds an `alternate-script` [refinement](Self::refinement),
    /// providing an alternate version of a value in another language or script/alphabet.
    ///
    /// This method may be called more than once to append multiple alternate scripts.
    ///
    /// # Notable Arguments
    /// - `bcp_47_language_code` - The given code is not validated and ***should*** be a valid
    ///   [BCP 47](https://tools.ietf.org/html/bcp47) tag (e.g. `en`, `ja`, `fr-CA`).
    /// - `text` - The translated/transliterated text.
    ///
    /// # Examples
    /// - Providing the Kanji representation of a name:
    /// ```
    /// # use rbook::epub::metadata::DetachedEpubMetaEntry;
    /// DetachedEpubMetaEntry::creator("Hanako Yamada")
    ///     .alternate_script("ja", "山田花子");
    /// ```
    pub fn alternate_script(
        mut self,
        bcp_47_language_code: impl Into<String>,
        text: impl Into<String>,
    ) -> Self {
        let mut entry = self.as_mut();
        let mut refinements = entry.refinements_mut();

        // Note: An entry can have multiple alternate scripts
        refinements.push(
            DetachedEpubMetaEntry::meta(opf::ALTERNATE_SCRIPT)
                .attribute((xml::LANG, bcp_47_language_code))
                .value(text),
        );

        self
    }
}

impl DetachedEpubMetaEntry {
    /// Creates a new **Dublin Core** entry (`<dc:*>`) with the given [`property`](Self::property).
    ///
    /// Responsible for required metadata elements such as `title`, `language`, `identifier`, etc.
    pub fn dublin_core(property: impl Into<String>) -> Self {
        Self::new(EpubMetaEntryKind::DublinCore {}).property(property)
    }

    /// Creates a new EPUB 3 **metadata** entry (`<meta property="…">`).
    ///
    /// Responsible for EPUB 3 properties (e.g., `dcterms:modified`) or refinements.
    ///
    /// # Note
    /// This is an EPUB 3 feature.
    /// When [writing](crate::Epub::write) an EPUB 2 ebook, EPUB 3 meta entries are ignored.
    ///
    /// # Examples
    /// - Creating a meta entry:
    /// ```
    /// # use rbook::epub::metadata::DetachedEpubMetaEntry;
    /// use rbook::ebook::metadata::datetime::DateTime;
    ///
    /// let today = DateTime::now();
    /// let entry = DetachedEpubMetaEntry::meta("dcterms:modified").value(today.to_string());
    ///
    /// assert_eq!("dcterms:modified", entry.as_view().property());
    /// ```
    pub fn meta(property: impl Into<String>) -> Self {
        Self::new(EpubMetaEntryKind::Meta {
            version: EpubVersion::EPUB3,
        })
        .property(property)
    }

    /// Creates a new EPUB 2 **metadata** entry (`<meta name="…">`).
    ///
    /// Used for classic metadata such as `cover` or generator information.
    ///
    /// # Note
    /// [`Self::meta`] is recommended over this method for modern metadata.
    ///
    /// # See Also
    /// - [`EpubWriteOptions::target`](crate::epub::EpubWriteOptions::target) for
    ///   auto-generated cover entry details.
    /// - [`EpubEditor::generator`](crate::epub::EpubEditor::generator) to set the generator.
    pub fn meta_name(name: impl Into<String>) -> Self {
        Self::new(EpubMetaEntryKind::Meta {
            version: EpubVersion::EPUB2,
        })
        .property(name)
    }

    /// Sets the property (e.g., `title`, `media:duration`).
    ///
    /// # Dublin Core Auto-Prefixing
    /// If a detached entry was created via [`Self::dublin_core`], and the given property
    /// does not start with `dc:`, the prefix is automatically prepended.
    ///
    /// # Examples
    /// - Creating a title entry (Auto-prefixing):
    /// ```
    /// # use rbook::epub::metadata::DetachedEpubMetaEntry;
    /// let entry = DetachedEpubMetaEntry::dublin_core("placeholder")
    ///     .property("title") // Automatically becomes "dc:title"
    ///     .value("My First Story");
    ///
    /// assert_eq!("dc:title", entry.as_view().property());
    /// ```
    ///
    /// # See Also
    /// - [`EpubMetaEntry::property`] to see mapping details.
    pub fn property(mut self, property: impl Into<String>) -> Self {
        let mut property = property.into();

        if self.0.kind.is_dublin_core() && !property.starts_with(dc::NAMESPACE) {
            // Correct property to include the namespace
            property.insert_str(0, dc::NAMESPACE)
        }

        self.0.property = property;
        self
    }
}

impl DetachedEpubMetaEntry<marker::Contributor> {
    /// Creates a builder for a **Creator** (`dc:creator`).
    ///
    /// Represents a primary author or entity responsible for the publication.
    ///
    /// # Examples
    /// - Adding a creator:
    /// ```
    /// # use rbook::Epub;
    /// # use rbook::epub::metadata::DetachedEpubMetaEntry;
    /// # fn main() {
    /// let mut epub = Epub::new();
    ///
    /// assert_eq!(0, epub.metadata().creators().count());
    ///
    /// epub.metadata_mut().push(
    ///     DetachedEpubMetaEntry::creator("John Doe")
    ///         .id("author")
    ///         .file_as("Doe, John")
    ///         .alternate_script("ja", "山田太郎")
    ///         // Explicitly specifying the role as `author` and `illustrator`
    ///         .role("aut")
    ///         .role("ill"),
    /// );
    ///
    /// let mut creators = epub.metadata().creators();
    ///
    /// assert_eq!("John Doe", creators.next().unwrap().value());
    /// assert_eq!(None, creators.next());
    /// # }
    /// ```
    pub fn creator(name: impl Into<String>) -> Self {
        Self::dc(dc::CREATOR).value(name)
    }

    /// Creates a builder for a **Contributor** (`dc:contributor`)
    /// with the given [`name`](Self::value).
    ///
    /// Represents a secondary contributor (editor, illustrator, etc.).
    pub fn contributor(name: impl Into<String>) -> Self {
        Self::dc(dc::CONTRIBUTOR).value(name)
    }

    /// Creates a builder for a **Publisher** (`dc:publisher`).
    pub fn publisher(name: impl Into<String>) -> Self {
        Self::dc(dc::PUBLISHER).value(name)
    }

    /// Adds a `role` [refinement](Self::refinement) for the given `marc_relators_code`.
    /// The given code is not validated and ***should*** derive from the
    /// [**MARC Relators**](https://www.loc.gov/marc/relators/relaterm.html) standard.
    ///
    /// This method may be called more than once to append multiple roles.
    ///
    /// # Common Codes
    /// - `aut`: Author
    /// - `ill`: Illustrator
    /// - `edt`: Editor
    /// - `trl`: Translator
    ///
    /// # Examples
    /// - Specifying an editor and illustrator:
    /// ```
    /// # use rbook::epub::metadata::DetachedEpubMetaEntry;
    /// let contributor = DetachedEpubMetaEntry::contributor("Jane Doe")
    ///     .role("edt")
    ///     .role("ill");
    /// ```
    pub fn role(mut self, marc_relators_code: impl Into<String>) -> Self {
        let mut entry = self.as_mut();
        let mut refinements = entry.refinements_mut();

        // Note: A contributor can have multiple roles
        refinements.push(
            DetachedEpubMetaEntry::meta(opf::ROLE)
                .attribute((opf::SCHEME, marc::RELATORS))
                .value(marc_relators_code),
        );

        self
    }
}

impl DetachedEpubMetaEntry<marker::Identifier> {
    /// Creates a builder for an **Identifier** (`dc:identifier`)
    /// with the given [`value`](Self::value).
    pub fn identifier(id: impl Into<String>) -> Self {
        Self::dc(dc::IDENTIFIER).value(id.into())
    }

    /// Adds an `identifier-type` [refinement](Self::refinement),
    /// specifying the kind of identifier (e.g., ISBN, DOI, URL).
    ///
    /// The optional `scheme` identifies the source of the given type `code`.
    /// Typically, the `scheme` should be provided, although isn't required.
    ///
    /// If this method is called more than once, the previously provided input is overridden.
    ///
    /// # Examples
    /// - Specifying a DOI identifier:
    /// ```
    /// # use rbook::epub::metadata::DetachedEpubMetaEntry;
    /// let identifier = DetachedEpubMetaEntry::identifier("10.1000/182")
    ///     .scheme("onix:codelist5", "06"); // 06 = DOI in ONIX
    /// ```
    pub fn scheme(mut self, scheme: impl IntoOption<String>, code: impl Into<String>) -> Self {
        let mut entry = self.as_mut();
        let mut refinements = entry.refinements_mut();

        // There must only be *one* `identifier-type` refinement.
        refinements.retain(|r| r.property().as_str() != opf::IDENTIFIER_TYPE);

        let mut refinement = DetachedEpubMetaEntry::meta(opf::IDENTIFIER_TYPE).value(code);

        if let Some(scheme) = scheme.into_option() {
            refinement
                .as_mut()
                .attributes_mut()
                .insert((opf::SCHEME, scheme));
        }

        refinements.push(refinement);

        self
    }
}

impl DetachedEpubMetaEntry<marker::Title> {
    /// Creates a builder for a **Title** (`dc:title`) with the given [`value`](Self::value).
    ///
    /// # Examples
    /// - Adding titles to an [`Epub`](crate::epub::Epub):
    /// ```
    /// # use rbook::Epub;
    /// # use rbook::ebook::metadata::TitleKind;
    /// # use rbook::epub::metadata::DetachedEpubMetaEntry;
    /// let epub = Epub::builder()
    ///     .title([
    ///         DetachedEpubMetaEntry::title("Example EPUB")
    ///             .alternate_script("ja", "サンプルEPUB")
    ///             .kind(TitleKind::Main),
    ///         DetachedEpubMetaEntry::title("The Subtitle")
    ///             .kind(TitleKind::Subtitle),
    ///     ])
    ///     .build();
    ///
    /// assert_eq!(2, epub.metadata().titles().count());
    /// ```
    pub fn title(title: impl Into<String>) -> Self {
        Self::dc(dc::TITLE).value(title)
    }

    /// Adds a `title-type` [refinement](Self::refinement), specifying the kind of title.
    ///
    /// A [`TitleKind::Unknown`] value is ignored and has no effect.
    ///
    /// If this method is called more than once, the previously
    /// provided kind is replaced.
    ///
    /// # Examples
    /// - Creating a subtitle:
    /// ```
    /// # use rbook::ebook::metadata::TitleKind;
    /// # use rbook::epub::metadata::DetachedEpubMetaEntry;
    /// let subtitle = DetachedEpubMetaEntry::title("A Subtitle")
    ///     .kind(TitleKind::Subtitle);
    /// ```
    pub fn kind(mut self, kind: TitleKind) -> Self {
        let Some(title_type) = kind.as_str() else {
            // no-op; `kind` is `TitleKind::Unknown`
            return self;
        };

        let mut entry = self.as_mut();
        let mut refinements = entry.refinements_mut();

        // There must only be *one* `title_type` refinement.
        refinements.retain(|r| r.property().as_str() != opf::TITLE_TYPE);
        refinements.push(DetachedEpubMetaEntry::meta(opf::TITLE_TYPE).value(title_type));

        self
    }
}

impl DetachedEpubMetaEntry<marker::Tag> {
    /// Creates a builder for a **Tag/Subject** (`dc:subject`)
    /// with the given [`value`](Self::value).
    ///
    /// # Examples
    /// - Applying the scheme and adding tags to metadata:
    /// ```
    /// # use rbook::Epub;
    /// # use rbook::epub::metadata::DetachedEpubMetaEntry;
    /// let mut epub = Epub::new();
    ///
    /// assert_eq!(0, epub.metadata().tags().count());
    ///
    /// epub.metadata_mut().push([
    ///     DetachedEpubMetaEntry::tag("Fiction / Science Fiction")
    ///         .scheme("BISAC", "FIC028000"),
    ///     DetachedEpubMetaEntry::tag("Number Theory")
    ///         .scheme("https://www.ams.org/msc/msc2010.html", "11"),
    /// ]);
    ///
    /// assert_eq!(2, epub.metadata().tags().count());
    /// ```
    pub fn tag(tag: impl Into<String>) -> Self {
        Self::dc(dc::SUBJECT).value(tag)
    }

    /// Adds `authority` and `term` [refinements](Self::refinement)
    /// to define the source/origin of a tag.
    ///
    /// If this method is called more than once, the previously provided input is overridden.
    ///
    /// # Examples
    /// - Creating a BISAC tag:
    /// ```
    /// # use rbook::epub::metadata::DetachedEpubMetaEntry;
    /// let tag = DetachedEpubMetaEntry::tag("Fiction / Science Fiction")
    ///     .scheme("BISAC", "FIC028000");
    /// ```
    pub fn scheme(mut self, authority: impl Into<String>, term: impl Into<String>) -> Self {
        let mut entry = self.as_mut();
        let mut refinements = entry.refinements_mut();

        // There must only be *one* `authority` and `term` refinement.
        refinements.retain(|r| {
            let property = r.property().as_str();

            property != opf::AUTHORITY || property != opf::TERM
        });
        refinements.push(DetachedEpubMetaEntry::meta(opf::AUTHORITY).value(authority));
        refinements.push(DetachedEpubMetaEntry::meta(opf::TERM).value(term));

        self
    }
}

impl DetachedEpubMetaEntry<marker::Description> {
    /// Creates a builder for a **Description** (`dc:description`)
    /// with the given [`value`](Self::value).
    pub fn description(description: impl Into<String>) -> Self {
        Self::dc(dc::DESCRIPTION).value(description)
    }
}

impl DetachedEpubMetaEntry<marker::Language> {
    /// Creates a builder for a **Language** (`dc:language`)
    /// with the given [`value`](Self::value) (code).
    ///
    /// The given code is not validated and ***should*** be a valid
    /// [BCP 47](https://tools.ietf.org/html/bcp47) tag (e.g. `en`, `ja`, `fr-CA`).
    pub fn language(bcp47_language_code: impl Into<String>) -> Self {
        Self::dc(dc::LANGUAGE).value(bcp47_language_code)
    }
}

impl DetachedEpubMetaEntry<marker::Date> {
    /// Creates a builder for a **Date** (`dc:date`) with the given [`value`](Self::value).
    ///
    /// The given date is not validated.
    /// However, a date conforming to
    /// [**ISO 8601-1**](https://www.iso.org/iso-8601-date-and-time-format.html)
    /// is strongly recommended.
    ///
    /// # Note
    /// When passed as an argument to
    /// [`EpubEditor::modified_date`](crate::epub::write::EpubEditor::modified_date),
    /// this entry is converted into a `dcterms:modified` meta element.
    ///
    /// # Examples
    /// - Creating a detached date meta entry:
    /// ```
    /// use rbook::ebook::metadata::datetime::{Date, DateTime};
    /// # use rbook::epub::metadata::DetachedEpubMetaEntry;
    ///
    /// assert_eq!(
    ///     DetachedEpubMetaEntry::date("2026-02-18"),
    ///     DetachedEpubMetaEntry::from(Date::new(2026, 2, 18))
    /// );
    ///
    /// // Alternatively, retrieving a meta entry corresponding to the current date and time:
    /// let current_date = DetachedEpubMetaEntry::from(DateTime::now());
    /// ```
    pub fn date(date: impl Into<String>) -> Self {
        Self::dc(dc::DATE).value(date)
    }
}

impl From<DateTime> for DetachedEpubMetaEntry<marker::Date> {
    fn from(datetime: DateTime) -> Self {
        Self::date(datetime.to_string())
    }
}

impl From<Date> for DetachedEpubMetaEntry<marker::Date> {
    fn from(date: Date) -> Self {
        Self::date(date.to_string())
    }
}

impl DetachedEpubMetaEntry<marker::Rights> {
    /// Creates a builder for **Copyright/Licensing** (`dc:rights`)
    /// with the given [`value`](Self::value).
    pub fn rights(copyright: impl Into<String>) -> Self {
        Self::dc(dc::RIGHTS).value(copyright)
    }
}

impl DetachedEpubMetaEntry<marker::Link> {
    /// Creates a new **link** entry (`<link>`) with the given [`rel`](Self::rel).
    ///
    /// Responsible for linking to external resources or establishing relationships.
    ///
    /// # Examples
    /// - Inserting a link into an [`Epub`](crate::epub::Epub):
    /// ```
    /// # use rbook::Epub;
    /// # use rbook::epub::metadata::DetachedEpubMetaEntry;
    /// let mut epub = Epub::new();
    ///
    /// epub.metadata_mut().push(
    ///     DetachedEpubMetaEntry::link("record")
    ///         .id("report-1")
    ///         .href("meta/eval.xml")
    ///         .href_lang("en-US")
    ///         .media_type("application/xml")
    ///         // Appends the given values to properties
    ///         .property("rbook:customProperty")
    ///         .property("rbook:prop2")
    ///         // Appends the given values to rel -> "record dcterms:conformsTo"
    ///         .rel("dcterms:conformsTo"),
    /// );
    /// ```
    pub fn link(rel: impl Into<String>) -> Self {
        let mut link = Self::new(EpubMetaEntryKind::Link {});
        let mut entry = link.as_mut();
        entry.attributes_mut().insert((opf::REL, rel));
        link
    }

    /// Sets the location of the specified resource a link points to.
    pub fn href(self, href: impl Into<String>) -> Self {
        self.attribute((opf::HREF, href))
    }

    /// Sets the language of the resource referenced by [`Self::href`].
    pub fn href_lang(self, href_lang: impl Into<String>) -> Self {
        self.attribute((opf::HREFLANG, href_lang))
    }

    /// Sets the media type, identifying the type of the resource referenced by [`Self::href`].
    ///
    /// The given `media_type` is not validated and ***should*** be a valid
    /// [MIME](https://www.iana.org/assignments/media-types/media-types.xhtml).
    pub fn media_type(self, media_type: impl Into<String>) -> Self {
        self.attribute((opf::MEDIA_TYPE, media_type))
    }

    fn add_property(mut self, name: &str, property: &str) -> Self {
        let mut entry = self.as_mut();
        let attributes = entry.attributes_mut();

        if let Some(value) = attributes.by_name_mut(name) {
            value.as_properties_mut().insert(property);
        } else {
            let mut attribute = Attribute::new(name, String::new());
            attribute.as_properties_mut().insert(property);
            attributes.insert(attribute);
        }
        self
    }

    /// Appends one or more properties via
    /// [`Properties::insert`](crate::ebook::element::Properties::insert).
    pub fn property(self, property: &str) -> Self {
        self.add_property(opf::PROPERTIES, property)
    }

    /// Appends one or more rel values using
    /// [`Properties::insert`](crate::ebook::element::Properties::insert).
    pub fn rel(self, rel: &str) -> Self {
        self.add_property(opf::REL, rel)
    }
}

impl From<EpubMetaEntryData> for DetachedEpubMetaEntry {
    fn from(value: EpubMetaEntryData) -> Self {
        Self(value, std::marker::PhantomData)
    }
}

impl<P: Into<String>, V: Into<String>> From<(P, V)> for DetachedEpubMetaEntry {
    fn from((property, value): (P, V)) -> Self {
        let property = property.into();

        if property.starts_with(dc::NAMESPACE) {
            Self::dublin_core(property)
        } else {
            Self::meta(property)
        }
        .value(value)
    }
}

impl<M, I: Into<DetachedEpubMetaEntry<M>>> Many<DetachedEpubMetaEntry<M>> for I {
    type Iter = std::iter::Once<DetachedEpubMetaEntry<M>>;

    fn iter_many(self) -> Self::Iter {
        std::iter::once(self.into())
    }
}

macro_rules! impl_traits_for_detached {
    {$($target:ty => $constructor:ident)+} => {
        $(
        impl From<DetachedEpubMetaEntry<$target>> for DetachedEpubMetaEntry {
            fn from(detached: DetachedEpubMetaEntry<$target>) -> Self {
                detached.into_any()
            }
        }

        impl From<DetachedEpubMetaEntry> for DetachedEpubMetaEntry<$target> {
            fn from(detached: DetachedEpubMetaEntry) -> Self {
                DetachedEpubMetaEntry(
                    detached.0,
                    Default::default(),
                )
            }
        }

        impl From<String> for DetachedEpubMetaEntry<$target> {
            fn from(value: String) -> Self {
                Self::$constructor(value)
            }
        }

        impl From<&String> for DetachedEpubMetaEntry<$target> {
            fn from(value: &String) -> Self {
                Self::$constructor(value)
            }
        }

        impl<'a> From<&'a str> for DetachedEpubMetaEntry<$target> {
            fn from(value: &'a str) -> Self {
                Self::$constructor(value)
            }
        }

        impl<'a> From<std::borrow::Cow<'a, str>> for DetachedEpubMetaEntry<$target> {
            fn from(value: std::borrow::Cow<'a, str>) -> Self {
                Self::$constructor(value.into_owned())
            }
        }
        )+
    };
}

impl_traits_for_detached! {
    marker::Identifier => identifier
    marker::Title => title
    marker::Contributor => contributor
    marker::Tag => tag
    marker::Description => description
    marker::Language => language
    marker::Date => date
    marker::Rights => rights
    marker::Link => link
}

/// Mutable view of [`EpubMetadata`] accessible via
/// [`Epub::metadata_mut`](crate::epub::Epub::metadata_mut).
///
/// Allows creation, modification, and removal of *top-level* (i.e., non-refining)
/// metadata entries (e.g., `<dc:title>`, `<dc:creator>`, `<meta>`).
///
/// # Refinements
/// To modify refinements (nested metadata), the parent must first be retrieved
/// (from [`Self::by_id_mut`] or similar), and then accessed via
/// [`EpubMetaEntryMut::refinements_mut`].
///
/// # See Also
/// - [`EpubEditor`](crate::epub::EpubEditor) for simple modification tasks.
pub struct EpubMetadataMut<'ebook> {
    package: &'ebook mut EpubPackageData,
    metadata: &'ebook mut EpubMetadataData,
}

impl<'ebook> EpubMetadataMut<'ebook> {
    //////////////////////////////////
    // PRIVATE API
    //////////////////////////////////

    pub(in crate::ebook::epub) fn new(
        package: &'ebook mut EpubPackageData,
        metadata: &'ebook mut EpubMetadataData,
    ) -> Self {
        Self { package, metadata }
    }

    fn push_detached(&mut self, detached: impl Iterator<Item = DetachedEpubMetaEntry>) {
        let entries = &mut self.metadata.entries;

        for detached in detached {
            if let Some(category) = entries.get_mut(&detached.0.property) {
                category.push(detached.0);
            } else {
                // Insert new category
                let property = detached.0.property.clone();
                entries.insert(property, vec![detached.0]);
            }
        }
    }

    //////////////////////////////////
    // PUBLIC API
    //////////////////////////////////

    /// Inserts one or more metadata entries via the [`Many`] trait.
    ///
    /// New entries are appended to the end of the list for their specific
    /// [`property`](EpubMetaEntry::property).
    ///
    /// # Examples
    /// - Adding a new creator:
    /// ```
    /// # use rbook::Epub;
    /// # use rbook::epub::metadata::DetachedEpubMetaEntry;
    /// # fn main() -> rbook::ebook::errors::EbookResult<()> {
    /// let mut epub = Epub::open("tests/ebooks/example_epub")?;
    ///
    /// epub.metadata_mut().push(
    ///     DetachedEpubMetaEntry::creator("Jane Doe")
    ///         .id("jane")
    ///         .file_as("Doe, Jane"),
    /// );
    ///
    /// let mut creators = epub.metadata().creators();
    ///
    /// // Initial creator:
    /// let first_creator = creators.next().unwrap();
    /// assert_eq!("John Doe", first_creator.value());
    ///
    /// // Newly added creator:
    /// let added_creator = creators.next().unwrap();
    /// assert_eq!("Jane Doe", added_creator.value());
    /// assert_eq!(Some("Doe, Jane"), added_creator.file_as());
    /// assert_eq!(None, creators.next());
    /// # Ok(())
    /// # }
    /// ```
    /// - Adding multiple tags:
    /// ```
    /// # use rbook::Epub;
    /// # use rbook::epub::metadata::DetachedEpubMetaEntry;
    /// # fn main() -> rbook::ebook::errors::EbookResult<()> {
    /// # let mut epub = Epub::open("tests/ebooks/example_epub")?;
    /// epub.metadata_mut().push([
    ///     ("dc:subject", "Fiction"),
    ///     ("dc:subject", "Romance"),
    ///     // Alternatively:
    ///     // DetachedEpubMetaEntry::tag("Fiction"),
    ///     // ...,
    /// ]);
    ///
    /// let mut tags = epub.metadata().tags();
    /// // Initial tags:
    /// assert_eq!("FICTION / Occult & Supernatural", tags.next().unwrap().value());
    /// assert_eq!("Quests (Expeditions) -- Fiction", tags.next().unwrap().value());
    /// assert_eq!("Fantasy", tags.next().unwrap().value());
    /// // Newly added tags:
    /// assert_eq!("Fiction", tags.next().unwrap().value());
    /// assert_eq!("Romance", tags.next().unwrap().value());
    /// assert_eq!(None, tags.next());
    /// # Ok(())
    /// # }
    /// ```
    pub fn push(&mut self, detached: impl Many<DetachedEpubMetaEntry>) {
        self.push_detached(detached.iter_many());
    }

    /// Inserts one or more entries at the given `index` via the [`Many`] trait,
    /// within their respective [`property`](EpubMetaEntry::property) groups.
    ///
    /// This is useful for defining the **primary** entry,
    /// such as ensuring the main author appears first among `dc:creator` entries.
    ///
    /// # Note
    /// - The `index` is relative to a **property group** (e.g., all `dc:title` entries);
    ///   not all metadata.
    /// - If the `index` is greater than the current number of entries for a property,
    ///   then new entries are appended to the end.
    /// - The relative order of entries inserted as a batch is preserved.
    ///
    /// # Examples
    /// - Setting the primary author:
    /// ```
    /// # use rbook::Epub;
    /// # use rbook::epub::metadata::DetachedEpubMetaEntry;
    /// let mut epub = Epub::new();
    ///
    /// epub.metadata_mut().push(DetachedEpubMetaEntry::creator("Second Author"));
    ///
    /// // Insert "First Author" as the first creator
    /// epub.metadata_mut().insert(0, DetachedEpubMetaEntry::creator("First Author"));
    ///
    /// let mut creators = epub.metadata().creators();
    /// assert_eq!("First Author", creators.next().unwrap().value());
    /// assert_eq!("Second Author", creators.next().unwrap().value());
    /// assert_eq!(None, creators.next());
    /// ```
    pub fn insert(&mut self, index: usize, detached: impl Many<DetachedEpubMetaEntry>) {
        let mut detached = detached.iter_many();

        // Optimized path for a single item (Common use-case)
        if detached.has_one_remaining_hint()
            && let Some(entry) = detached.next()
        {
            let property = &entry.0.property;

            if let Some(group) = self.metadata.entries.get_mut(property) {
                let clamped = index.min(group.len());

                group.insert(clamped, entry.0);
            } else {
                self.metadata
                    .entries
                    .insert(property.to_owned(), vec![entry.0]);
            }
            return;
        }

        // partition
        let mut batches = std::collections::HashMap::<_, Vec<EpubMetaEntryData>>::new();

        for entry in detached {
            let property = entry.0.property.as_str();

            match batches.get_mut(property) {
                Some(group) => group.push(entry.0),
                None => {
                    batches.insert(property.to_owned(), vec![entry.0]);
                }
            }
        }

        for (property, batch) in batches {
            if let Some(group) = self.metadata.entries.get_mut(&property) {
                let clamped = index.min(group.len());

                group.splice(clamped..clamped, batch);
            } else {
                self.metadata.entries.insert(property, batch);
            }
        }
    }

    /// Searches the metadata hierarchy, including refinements, and returns the
    /// [`EpubMetaEntryMut`] matching the given `id`, or [`None`] if not found.
    ///
    /// # Note
    /// This method has the same limitations mentioned in [`EpubMetadata::by_id`].
    ///
    /// # See Also
    /// - [`Self::by_property_mut`] to retrieve entries by their
    ///   [`property`](EpubMetaEntry::property) (e.g. all titles).
    ///
    /// # Examples
    /// - Updating an entry:
    /// ```
    /// # use rbook::Epub;
    /// # fn main() -> rbook::ebook::errors::EbookResult<()> {
    /// let mut epub = Epub::open("tests/ebooks/example_epub")?;
    ///
    /// // Original author name
    /// assert_eq!("John Doe", epub.metadata().by_id("author").unwrap().value());
    ///
    /// let mut metadata = epub.metadata_mut();
    /// // `None` is returned if a non-existent `id` is given
    /// assert!(metadata.by_id_mut("doesn't exist").is_none());
    ///
    /// let mut entry = metadata.by_id_mut("author").unwrap();
    /// entry.set_value("Jane Doe");
    ///
    /// // New author name
    /// assert_eq!("Jane Doe", epub.metadata().by_id("author").unwrap().value());
    /// # Ok(())
    /// # }
    /// ```
    pub fn by_id_mut(&mut self, id: &str) -> Option<EpubMetaEntryMut<'_>> {
        // Returns the matched entry along with its local `index` and `refines` field:
        // `(index, refines, entry)`
        fn dfs_by_id_mut<'a>(
            // ID to search for
            id: &str,
            entry: &'a mut EpubMetaEntryData,
            // The parent ID of the given `entry`
            refines: Option<&'a str>,
            // The index of `entry` within its parent collection.
            // - The index is passed in by the caller because entries do not store
            //   their position within the parent collection.
            //   The index is a required field for `EpubMetaEntryMut` views.
            index: usize,
        ) -> Option<(usize, Option<&'a str>, &'a mut EpubMetaEntryData)> {
            if entry.id.as_deref() == Some(id) {
                return Some((index, refines, entry));
            }

            // Check refinements
            entry
                .refinements
                .iter_mut()
                .enumerate()
                .find_map(|(i, refinement)| dfs_by_id_mut(id, refinement, entry.id.as_deref(), i))
        }

        let meta_ctx = EpubPackageMetaContext::new(self.package);

        self.metadata
            .entries
            .values_mut()
            .flat_map(|group| group.iter_mut().enumerate())
            .find_map(|(i, entry)| dfs_by_id_mut(id, entry, None, i))
            .map(|(i, refines, data)| EpubMetaEntryMut::new(meta_ctx, data, refines, i))
    }

    /// Returns an iterator over all mutable entries matching
    /// the given [`property`](EpubMetaEntry::property)
    /// (e.g., `dc:title`, `dc:creator`, `dcterms:modified`).
    ///
    /// # Examples
    /// - Making all tags uppercase:
    /// ```
    /// # use rbook::Epub;
    /// # fn main() -> rbook::ebook::errors::EbookResult<()> {
    /// let mut epub = Epub::open("tests/ebooks/example_epub")?;
    ///
    /// // Original tags
    /// let mut tags = epub.metadata().tags();
    /// assert_eq!("FICTION / Occult & Supernatural", tags.next().unwrap().value());
    /// assert_eq!("Quests (Expeditions) -- Fiction", tags.next().unwrap().value());
    /// assert_eq!("Fantasy", tags.next().unwrap().value());
    /// assert_eq!(0, tags.count());
    ///
    /// // Making all tags uppercase:
    /// for mut subject in epub.metadata_mut().by_property_mut("dc:subject") {
    ///     let old_value = subject.as_view().value().to_string();
    ///     subject.set_value(old_value.to_uppercase());
    /// }
    ///
    /// // Modified tags
    /// let mut tags = epub.metadata().tags();
    /// assert_eq!("FICTION / OCCULT & SUPERNATURAL", tags.next().unwrap().value());
    /// assert_eq!("QUESTS (EXPEDITIONS) -- FICTION", tags.next().unwrap().value());
    /// assert_eq!("FANTASY", tags.next().unwrap().value());
    /// assert_eq!(None, tags.next());
    /// # Ok(())
    /// # }
    /// ```
    pub fn by_property_mut(
        &mut self,
        property: &str,
    ) -> impl Iterator<Item = EpubMetaEntryMut<'_>> {
        let meta_ctx = EpubPackageMetaContext::new(self.package);

        self.metadata
            .entries
            .get_mut(property)
            .into_iter()
            .flat_map(|category| category.iter_mut().enumerate())
            .map(move |(i, entry)| EpubMetaEntryMut::new(meta_ctx, entry, None, i))
    }

    /// Returns an iterator over non-refining link entries.
    ///
    /// # Note
    /// This method has the same restrictions as [`EpubMetadata::links`].
    pub fn links_mut(&mut self) -> impl Iterator<Item = EpubMetaEntryMut<'_>> {
        self.iter_mut()
            .filter(|entry| entry.as_view().kind().is_link())
    }

    /// Returns an iterator over non-refining metadata entries.
    ///
    /// # Note
    /// - This method has the same restrictions as [`EpubMetadata::iter`].
    /// - The iteration order is arbitrary between property groups, but deterministic
    ///   within a specific property group (based on insertion order).
    pub fn iter_mut(&mut self) -> EpubMetadataIterMut<'_> {
        EpubMetadataIterMut {
            meta_ctx: EpubPackageMetaContext::new(self.package),
            iter: self
                .metadata
                .entries
                .values_mut()
                .flat_map(|category| category.iter_mut().enumerate()),
        }
    }

    /// Searches the metadata hierarchy, including refinements, and removes the
    /// entry matching the given `id`, or [`None`] if not found.
    ///
    /// # Note
    /// This method has the same limitations mentioned in [`EpubMetadata::by_id`].
    ///
    /// # Examples
    /// - Removing an entry:
    /// ```
    /// # use rbook::Epub;
    /// # fn main() -> rbook::ebook::errors::EbookResult<()> {
    /// let mut epub = Epub::open("tests/ebooks/example_epub")?;
    ///
    /// // Existing entry
    /// assert_eq!("John Doe", epub.metadata().by_id("author").unwrap().value());
    ///
    /// let mut metadata = epub.metadata_mut();
    /// // `None` is returned if a non-existent `id` is given
    /// assert_eq!(None, metadata.remove_by_id("doesn't exist"));
    ///
    /// // Removing an entry returns it as an owned instance
    /// let mut author = metadata.remove_by_id("author").unwrap();
    /// assert_eq!("John Doe", author.as_view().value());
    ///
    /// // The ebook no longer contains the entry
    /// assert_eq!(None, epub.metadata().by_id("author"));
    /// # Ok(())
    /// # }
    /// ```
    pub fn remove_by_id(&mut self, id: &str) -> Option<DetachedEpubMetaEntry> {
        fn dfs_remove_by_id(
            id: &str,
            entries: &mut Vec<EpubMetaEntryData>,
        ) -> Option<DetachedEpubMetaEntry> {
            for (i, entry) in entries.iter_mut().enumerate() {
                if entry.id.as_deref() == Some(id) {
                    return Some(DetachedEpubMetaEntry::from(entries.remove(i)));
                }
                // Check refinements
                if let Some(removed) = dfs_remove_by_id(id, &mut entry.refinements) {
                    return Some(removed);
                }
            }
            None
        }

        self.metadata
            .entries
            .values_mut()
            .find_map(|entries| dfs_remove_by_id(id, entries))
    }

    /// Removes and returns **all** non-refining entries matching the given `property`.
    ///
    /// # Examples
    /// - Clearing all creators:
    /// ```
    /// # use rbook::Epub;
    /// # fn main() -> rbook::ebook::errors::EbookResult<()> {
    /// let mut epub = Epub::open("tests/ebooks/example_epub")?;
    ///
    /// // Existing creators:
    /// assert_eq!(1, epub.metadata().creators().count());
    ///
    /// let mut metadata = epub.metadata_mut();
    /// // Removing all creators and collecting them into a `Vec`
    /// let removed: Vec<_> = metadata.remove_by_property("dc:creator").collect();
    /// assert_eq!(1, removed.len());
    /// assert_eq!("John Doe", removed[0].as_view().value());
    ///
    /// // The ebook no longer contains any creators
    /// assert_eq!(0, epub.metadata().creators().count());
    /// # Ok(())
    /// # }
    /// ```
    pub fn remove_by_property(
        &mut self,
        property: &str,
    ) -> impl Iterator<Item = DetachedEpubMetaEntry> {
        self.metadata
            .entries
            .shift_remove(property)
            .unwrap_or_default()
            .into_iter()
            .map(DetachedEpubMetaEntry::from)
    }

    /// Retains only the non-refining entries specified by the predicate.
    ///
    /// If the closure returns `false`, the entry is retained.
    /// Otherwise, the entry is removed.
    ///
    /// This method operates in place and visits every entry exactly once.
    ///
    /// # See Also
    /// - [`Self::extract_if`] to retrieve an iterator of the removed entries.
    ///
    /// # Examples
    /// - Removing legacy EPUB 2 `<meta>` elements:
    /// ```
    /// # use rbook::Epub;
    /// # fn main() -> rbook::ebook::errors::EbookResult<()> {
    /// let mut epub = Epub::open("tests/ebooks/example_epub")?;
    ///
    /// // Checking the number of EPUB 2 meta elements
    /// assert_eq!(
    ///     2,
    ///     epub.metadata().iter().filter(|entry| entry.kind().is_epub2_meta()).count(),
    /// );
    ///
    /// // Retain only entries that are not EPUB 2 meta elements
    /// epub.metadata_mut().retain(|entry| !entry.kind().is_epub2_meta());
    ///
    /// // The ebook no longer contains any EPUB 2 meta elements
    /// assert_eq!(
    ///     0,
    ///     epub.metadata().iter().filter(|entry| entry.kind().is_epub2_meta()).count(),
    /// );
    /// # Ok(())
    /// # }
    /// ```
    pub fn retain(&mut self, mut f: impl FnMut(EpubMetaEntry<'_>) -> bool) {
        let context = EpubPackageMetaContext::new(self.package);

        self.metadata.entries.retain(|_, category| {
            let mut index = 0;

            category.retain(|entry| {
                let retain = f(EpubMetaEntry::new(context, None, entry, index));
                index += 1;
                retain
            });

            // Retain the category if there is at least one entry
            !category.is_empty()
        });
    }

    /// Removes and returns only the non-refining entries specified by the predicate.
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
    /// - Extracting all Dublin Core metadata entries:
    /// ```
    /// # use rbook::Epub;
    /// # fn main() -> rbook::ebook::errors::EbookResult<()> {
    /// let mut epub = Epub::open("tests/ebooks/example_epub")?;
    ///
    /// let dublin_core: Vec<_> = epub.metadata_mut()
    ///     .extract_if(|entry| entry.kind().is_dublin_core())
    ///     .collect();
    /// // A total of 12 entries were removed (e.g., dc:title, dc:subject, etc.)
    /// assert_eq!(12, dublin_core.len());
    ///
    /// // The ebook no longer contains any Dublin Core entries
    /// assert_eq!(
    ///     0,
    ///     epub.metadata().iter().filter(|entry| entry.kind().is_dublin_core()).count(),
    /// );
    /// # Ok(())
    /// # }
    /// ```
    pub fn extract_if(
        &mut self,
        mut f: impl FnMut(EpubMetaEntry<'_>) -> bool,
    ) -> impl Iterator<Item = DetachedEpubMetaEntry> {
        let provider = EpubPackageMetaContext::new(self.package);
        let entries = &mut self.metadata.entries;
        let mut category_index = 0;
        let mut index = 0;

        std::iter::from_fn(move || {
            // Outer loop: iterate through categories
            while category_index < entries.len() {
                let category = &mut entries[category_index];

                // Inner loop: iterate through the current Vec
                while index < category.len() {
                    if f(provider.create_entry(&category[index], index)) {
                        // The length of metadata entries is typically very small (1-5)
                        let data = category.remove(index);

                        return Some(DetachedEpubMetaEntry::from(data));
                    }
                    index += 1;
                }

                // If the category is empty, remove it
                if category.is_empty() {
                    entries.shift_remove_index(category_index);
                } else {
                    category_index += 1;
                }
                index = 0;
            }
            None
        })
    }

    /// Removes and returns all non-refining metadata entries.
    pub fn drain(&mut self) -> impl Iterator<Item = DetachedEpubMetaEntry> {
        self.metadata
            .entries
            .drain(..)
            .flat_map(|(_, category)| category)
            .map(DetachedEpubMetaEntry::from)
    }

    /// Removes all top-level (non-refining) metadata entries.
    ///
    /// # See Also
    /// - [`Self::drain`] to retrieve an iterator of the removed entries.
    pub fn clear(&mut self) {
        self.metadata.entries.clear();
    }

    /// Returns a read-only view, useful for inspecting state before applying modifications.
    pub fn as_view(&self) -> EpubMetadata<'_> {
        EpubMetadata::new(self.package, self.metadata)
    }
}

impl Debug for EpubMetadataMut<'_> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> Result<(), std::fmt::Error> {
        f.debug_struct("EpubMetadataMut")
            .field("metadata", &self.metadata)
            .finish_non_exhaustive()
    }
}

impl<M> Extend<DetachedEpubMetaEntry<M>> for EpubMetadataMut<'_> {
    fn extend<T: IntoIterator<Item = DetachedEpubMetaEntry<M>>>(&mut self, iter: T) {
        self.push_detached(iter.into_iter().map(DetachedEpubMetaEntry::into_any))
    }
}

impl<'a, 'ebook: 'a> IntoIterator for &'a mut EpubMetadataMut<'ebook> {
    type Item = EpubMetaEntryMut<'a>;
    type IntoIter = EpubMetadataIterMut<'a>;

    fn into_iter(self) -> Self::IntoIter {
        self.iter_mut()
    }
}

impl<'ebook> IntoIterator for EpubMetadataMut<'ebook> {
    type Item = EpubMetaEntryMut<'ebook>;
    type IntoIter = EpubMetadataIterMut<'ebook>;

    fn into_iter(self) -> Self::IntoIter {
        EpubMetadataIterMut {
            meta_ctx: EpubPackageMetaContext::new(self.package),
            iter: self
                .metadata
                .entries
                .values_mut()
                .flat_map(|category| category.iter_mut().enumerate()),
        }
    }
}

/// Returns an iterator over non-refining mutable metadata [entries](EpubMetaEntryMut)
/// contained within [`EpubMetadataMut`].
///
/// # See Also
/// - [`EpubMetadataMut::iter_mut`] to create an instance of this struct.
pub struct EpubMetadataIterMut<'ebook> {
    meta_ctx: EpubPackageMetaContext<'ebook>,
    iter: InnerMetadataIter<
        indexmap::map::ValuesMut<'ebook, String, Vec<EpubMetaEntryData>>,
        SliceIterMut<'ebook, EpubMetaEntryData>,
        &'ebook mut Vec<EpubMetaEntryData>,
    >,
}

impl Debug for EpubMetadataIterMut<'_> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> Result<(), std::fmt::Error> {
        f.debug_struct("EpubMetadataIterMut")
            .field("iter", &self.iter)
            .finish_non_exhaustive()
    }
}

impl<'ebook> Iterator for EpubMetadataIterMut<'ebook> {
    type Item = EpubMetaEntryMut<'ebook>;

    fn next(&mut self) -> Option<Self::Item> {
        self.iter
            .next()
            .map(|(i, entry)| EpubMetaEntryMut::new(self.meta_ctx, entry, None, i))
    }
}

/// Mutable view of [`EpubMetaEntry`], allowing modification of metadata fields,
/// attributes, and [refinements](Self::refinements_mut).
///
/// # See Also
/// - [`DetachedEpubMetaEntry`] for an owned metadata entry instances.
pub struct EpubMetaEntryMut<'ebook> {
    meta_ctx: EpubPackageMetaContext<'ebook>,
    data: &'ebook mut EpubMetaEntryData,
    refines: Option<&'ebook str>,
    index: usize,
}

impl<'ebook> EpubMetaEntryMut<'ebook> {
    fn new(
        meta_ctx: EpubPackageMetaContext<'ebook>,
        data: &'ebook mut EpubMetaEntryData,
        refines: Option<&'ebook str>,
        index: usize,
    ) -> Self {
        Self {
            meta_ctx,
            data,
            refines,
            index,
        }
    }

    /// Sets the unique `id` and returns the previous value.
    ///
    /// # See Also
    /// - [`DetachedEpubMetaEntry::id`] for important details.
    pub fn set_id(&mut self, id: impl IntoOption<String>) -> Option<String> {
        std::mem::replace(&mut self.data.id, id.into_option())
    }

    /// Sets the value and returns the previous value.
    ///
    /// # See Also
    /// - [`DetachedEpubMetaEntry::value`] for more details.
    pub fn set_value(&mut self, value: impl Into<String>) -> String {
        std::mem::replace(&mut self.data.value, value.into())
    }

    /// Sets the language (`xml:lang`) and returns the previous code.
    ///
    /// # See Also
    /// - [`DetachedEpubMetaEntry::xml_language`] for more details.
    pub fn set_xml_language(&mut self, code: impl IntoOption<String>) -> Option<String> {
        std::mem::replace(&mut self.data.language, code.into_option())
    }

    /// Sets the text direction (`dir`) and returns the previous value.
    ///
    /// # See Also
    /// - [`DetachedEpubMetaEntry::text_direction`] for more details.
    pub fn set_text_direction(&mut self, direction: TextDirection) -> TextDirection {
        std::mem::replace(&mut self.data.text_direction, direction)
    }

    /// Mutable view of all additional `XML` attributes.
    ///
    /// Used for attributes like `scheme`, `media-type`, or custom namespaced attributes.
    ///
    /// # See Also
    /// - [`DetachedEpubMetaEntry::attribute`] for important details.
    pub fn attributes_mut(&mut self) -> &mut Attributes {
        &mut self.data.attributes
    }

    /// Mutable view of all direct refinements.
    ///
    /// # ID Generation
    /// If this parent entry does not have an `id`
    /// ([`self.as_view().id()`](EpubMetaEntry::id)), a unique ID will be
    /// auto-generated during [writing](crate::epub::Epub::write).
    /// This ensures that all refinements correctly reference their parent.
    ///
    /// # Note
    /// If parent entries lack an ID, the [`refines`](EpubMetaEntry::refines)
    /// field of its refinements will return [`None`].
    ///
    /// # See Also
    /// - [`Self::set_id`] to set the ID and override ID generation.
    /// - [`DetachedEpubMetaEntry::refinement`]
    pub fn refinements_mut(&mut self) -> EpubRefinementsMut<'_> {
        EpubRefinementsMut::new(
            self.meta_ctx,
            self.data.id.as_deref(),
            &mut self.data.refinements,
        )
    }

    /// Returns a read-only view, useful for inspecting state before applying modifications.
    pub fn as_view(&self) -> EpubMetaEntry<'_> {
        self.meta_ctx
            .create_refining_entry(self.refines, self.data, self.index)
    }
}

impl Debug for EpubMetaEntryMut<'_> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> Result<(), std::fmt::Error> {
        f.debug_struct("EpubMetaEntryMut")
            .field("index", &self.index)
            .field("refines", &self.refines)
            .field("data", &self.data)
            .finish_non_exhaustive()
    }
}

/// Mutable view of [`EpubRefinements`],
/// allowing management of nested metadata entries (children).
///
/// # Parent Linking
/// The `refines` field of refinements added to this collection
/// is implicitly linked to the parent entry.
///
/// # Nested Refinements
/// Iterators returned by this collection are not recursive; nested refinements are not yielded.
/// **Specifically, only direct children are included.**
/// If a refinement itself contains additional refinements, those are excluded.
///
/// # Examples
/// - Adding the [`role`](DetachedEpubMetaEntry::role) refinement to a creator.
/// ```
/// # use rbook::epub::metadata::DetachedEpubMetaEntry;
/// // Manually creating a refinement:
/// let mut creator = DetachedEpubMetaEntry::dublin_core("dc:creator").value("John Doe");
///
/// creator.as_mut().refinements_mut().push(
///     DetachedEpubMetaEntry::meta("role")
///         .attribute(("scheme", "marc:relators"))
///         .value("aut"),
/// );
///
/// // Alternatively, same process concisely via builder-esque methods:
/// let creator = DetachedEpubMetaEntry::creator("John Doe").role("aut");
/// ```
pub struct EpubRefinementsMut<'ebook> {
    meta_ctx: EpubPackageMetaContext<'ebook>,
    parent_id: Option<&'ebook str>,
    data: &'ebook mut EpubRefinementsData,
}

impl<'ebook> EpubRefinementsMut<'ebook> {
    pub(in crate::ebook::epub) fn new(
        meta_ctx: EpubPackageMetaContext<'ebook>,
        parent_id: Option<&'ebook str>,
        data: &'ebook mut EpubRefinementsData,
    ) -> Self {
        Self {
            meta_ctx,
            parent_id,
            data,
        }
    }

    fn insert_detached(
        &mut self,
        index: usize,
        mut detached: impl Iterator<Item = DetachedEpubMetaEntry>,
    ) {
        if detached.has_one_remaining_hint()
            && let Some(entry) = detached.next()
        {
            self.data.insert(index, entry.0);
        } else {
            self.data.splice(index..index, detached.map(|e| e.0));
        }
    }

    /// Appends one or more refinements to the end via the [`Many`] trait.
    ///
    /// # Examples
    /// - Adding refinements to the last tag (subject):
    /// ```
    /// # use rbook::Epub;
    /// # use rbook::epub::metadata::DetachedEpubMetaEntry;
    /// # fn main() -> rbook::ebook::errors::EbookResult<()> {
    /// let mut epub = Epub::open("tests/ebooks/example_epub")?;
    /// let mut metadata_mut = epub.metadata_mut();
    ///
    /// // Retrieve the last tag
    /// let mut last_tag = metadata_mut.by_property_mut("dc:subject").last().unwrap();
    /// let mut refinements = last_tag.refinements_mut();
    ///
    /// // Remove any existing refinements
    /// refinements.clear();
    /// refinements.push([
    ///     DetachedEpubMetaEntry::meta("authority").value("A"),
    ///     DetachedEpubMetaEntry::meta("term").value("B"),
    /// ]);
    ///
    /// // Checking refinements
    /// let updated_tag = epub.metadata().tags().last().unwrap();
    /// let scheme = updated_tag.scheme().unwrap();
    /// assert_eq!("Fantasy", updated_tag.value());
    /// assert_eq!(Some("A"), scheme.source());
    /// assert_eq!("B", scheme.code());
    /// # Ok(())
    /// # }
    /// ```
    pub fn push(&mut self, detached: impl Many<DetachedEpubMetaEntry>) {
        self.insert(self.data.len(), detached);
    }

    /// Inserts one or more refinements at the given `index` via the [`Many`] trait.
    ///
    /// # Panics
    /// Panics if the given `index` to insert at is greater than [`EpubRefinements::len`].
    pub fn insert(&mut self, index: usize, detached: impl Many<DetachedEpubMetaEntry>) {
        self.insert_detached(index, detached.iter_many());
    }

    /// Returns the associated refinement ([`EpubMetaEntryMut`])
    /// if the given `index` is less than [`EpubRefinements::len`], otherwise [`None`].
    pub fn get_mut(&mut self, index: usize) -> Option<EpubMetaEntryMut<'_>> {
        self.data
            .get_mut(index)
            .map(|entry| EpubMetaEntryMut::new(self.meta_ctx, entry, self.parent_id, index))
    }

    /// Returns an iterator over all direct refinements matching
    /// the given [`property`](EpubMetaEntry::property)
    /// (e.g., `alternate-script`, `role`, `title-type`).
    pub fn by_property_mut(
        &mut self,
        property: &str,
    ) -> impl Iterator<Item = EpubMetaEntryMut<'_>> {
        let provider = self.meta_ctx;
        let parent_id = self.parent_id;

        self.data
            .iter_mut()
            .enumerate()
            .filter(move |(_, refinement)| refinement.property == property)
            .map(move |(i, entry)| EpubMetaEntryMut::new(provider, entry, parent_id, i))
    }

    /// Returns an iterator over all direct refinements.
    ///
    /// # Nested Refinements
    /// The returned iterator is not recursive.
    /// To iterate over nested refinements, call [`Self::iter_mut`] on yielded entries.
    pub fn iter_mut(&mut self) -> EpubRefinementsIterMut<'_> {
        EpubRefinementsIterMut {
            meta_ctx: self.meta_ctx,
            parent_id: self.parent_id,
            iter: self.data.iter_mut().enumerate(),
        }
    }

    /// Removes and returns the refinement at the given `index`.
    ///
    /// # Panics
    /// Panics if the given `index` is out of bounds
    /// (has a value greater than or equal to [`EpubRefinements::len`]).
    pub fn remove(&mut self, index: usize) -> DetachedEpubMetaEntry {
        DetachedEpubMetaEntry::from(self.data.remove(index))
    }

    /// Retains only the direct refinements specified by the predicate.
    ///
    /// If the closure returns `false`, the refinement is retained.
    /// Otherwise, the refinement is removed.
    ///
    /// This method operates in place and visits every direct refinement exactly once.
    ///
    /// # See Also
    /// - [`Self::extract_if`] to retrieve an iterator of the removed refinements.
    pub fn retain(&mut self, mut f: impl FnMut(EpubMetaEntry<'_>) -> bool) {
        let mut index = 0;

        self.data.retain(|entry| {
            let retain = f(self.meta_ctx.create_entry(entry, index));
            index += 1;
            retain
        });
    }

    /// Removes and returns only the direct refinements specified by the predicate.
    ///
    /// If the closure returns `true`, the refinement is removed and yielded.
    /// Otherwise, the entry is retained.
    ///
    /// # Drop
    /// If the returned iterator is not exhausted,
    /// (e.g. dropped without iterating or iteration short-circuits),
    /// then the remaining refinements are retained.
    ///
    /// Prefer [`Self::retain`] with a negated predicate if the returned iterator is not needed.
    pub fn extract_if(
        &mut self,
        mut f: impl FnMut(EpubMetaEntry<'_>) -> bool,
    ) -> impl Iterator<Item = DetachedEpubMetaEntry> {
        let provider = self.meta_ctx;
        let mut index = 0;

        self.data
            .extract_if(.., move |data| {
                let extract = f(provider.create_entry(data, index));
                index += 1;
                extract
            })
            .map(DetachedEpubMetaEntry::from)
    }

    /// Removes and returns all direct refinements within the given `range`.
    ///
    /// # Panics
    /// For the given `range`, this method panics if:
    /// - The starting point is greater than the end point.
    /// - The end point is greater than [`EpubRefinements::len`].
    pub fn drain(
        &mut self,
        range: impl std::ops::RangeBounds<usize>,
    ) -> impl Iterator<Item = DetachedEpubMetaEntry> {
        self.data.drain(range).map(DetachedEpubMetaEntry::from)
    }

    /// Removes all direct refinements.
    ///
    /// # See Also
    /// - [`Self::drain`] to retrieve an iterator of the removed refinements.
    pub fn clear(&mut self) {
        self.data.clear();
    }

    /// Returns a read-only view, useful for inspecting state before applying modifications.
    pub fn as_view(&self) -> EpubRefinements<'_> {
        self.meta_ctx.create_refinements(self.parent_id, self.data)
    }
}

impl Debug for EpubRefinementsMut<'_> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> Result<(), std::fmt::Error> {
        f.debug_struct("EpubRefinementsMut")
            .field("parent_id", &self.parent_id)
            .field("data", &self.data)
            .finish_non_exhaustive()
    }
}

impl Extend<DetachedEpubMetaEntry> for EpubRefinementsMut<'_> {
    fn extend<T: IntoIterator<Item = DetachedEpubMetaEntry>>(&mut self, iter: T) {
        self.insert_detached(self.data.len(), iter.into_iter());
    }
}

impl<'a, 'ebook> IntoIterator for &'a mut EpubRefinementsMut<'ebook> {
    type Item = EpubMetaEntryMut<'a>;
    type IntoIter = EpubRefinementsIterMut<'a>;

    fn into_iter(self) -> Self::IntoIter {
        self.iter_mut()
    }
}

impl<'ebook> IntoIterator for EpubRefinementsMut<'ebook> {
    type Item = EpubMetaEntryMut<'ebook>;
    type IntoIter = EpubRefinementsIterMut<'ebook>;

    fn into_iter(self) -> Self::IntoIter {
        EpubRefinementsIterMut {
            meta_ctx: self.meta_ctx,
            parent_id: self.parent_id,
            iter: self.data.iter_mut().enumerate(),
        }
    }
}

/// An iterator over the direct mutable [entries](EpubMetaEntryMut)
/// within [`EpubRefinementsMut`].
///
/// # See Also
/// - [`EpubRefinementsMut::iter_mut`] to create an instance of this struct.
pub struct EpubRefinementsIterMut<'ebook> {
    meta_ctx: EpubPackageMetaContext<'ebook>,
    parent_id: Option<&'ebook str>,
    iter: std::iter::Enumerate<std::slice::IterMut<'ebook, EpubMetaEntryData>>,
}

impl Debug for EpubRefinementsIterMut<'_> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> Result<(), std::fmt::Error> {
        f.debug_struct("EpubRefinementsIterMut")
            .field("parent_id", &self.parent_id)
            .field("iter", &self.iter)
            .finish_non_exhaustive()
    }
}

impl<'ebook> Iterator for EpubRefinementsIterMut<'ebook> {
    type Item = EpubMetaEntryMut<'ebook>;

    fn next(&mut self) -> Option<Self::Item> {
        self.iter
            .next()
            .map(|(i, entry)| EpubMetaEntryMut::new(self.meta_ctx, entry, self.parent_id, i))
    }
}
