//! Format-agnostic [`Metadata`]-related content.
//!
//! # See Also
//! - [`epub::metadata`][crate::epub::metadata] for the epub-specific metadata module.

pub mod datetime;

use crate::util::Sealed;
use std::fmt::Display;
use std::hash::Hash;

/// The metadata of an [`Ebook`](super::Ebook), encompassing detailed information,
/// such as the [`Version`], [`Title`], and [`Identifier`].
///
/// # See Also
/// - [`EpubMetadata`](crate::epub::metadata::EpubMetadata) for epub-specific metadata information.
///
/// # Examples
/// - Retrieving the [`author`](Metadata::creators) and [`subtitle`](Metadata::title):
/// ```
/// # use rbook::Epub;
/// # use rbook::ebook::metadata::TitleKind;
/// # fn main() -> rbook::ebook::errors::EbookResult<()> {
/// let epub = Epub::open("tests/ebooks/example_epub")?;
/// let metadata = epub.metadata();
///
/// // Retrieving the creator:
/// let author = metadata.creators().next().unwrap();
/// assert_eq!("John Doe", author.value());
/// assert_eq!(Some("Doe, John"), author.file_as());
/// assert_eq!(0, author.order());
///
/// // An EPUB may include multiple titles,
/// // here we retrieve the second title:
/// let title = metadata.titles().nth(1).unwrap();
/// assert_eq!("A subtitle", title.value());
/// assert_eq!(TitleKind::Subtitle, title.kind());
/// assert_eq!(1, title.order());
///
/// // Retrieving the last modified date:
/// let modified = epub.metadata().modified().unwrap();
/// let date = modified.date();
/// let time = modified.time();
/// assert_eq!((2023, 1, 25), (date.year(), date.month(), date.day()));
/// assert_eq!((10, 11, 35), (time.hour(), time.minute(), time.second()));
/// # Ok(())
/// # }
/// ```
pub trait Metadata<'ebook>: Sealed {
    /// The version of an [`ebook's`](super::Ebook) format in the form of a string.
    ///
    /// # See Also
    /// - [`Self::version`] for the parsed representation.
    ///
    /// # Examples
    /// - Retrieving the version of an ebook in EPUB format:
    /// ```
    /// # use rbook::Epub;
    /// # use rbook::ebook::metadata::Metadata;
    /// # fn main() -> rbook::ebook::errors::EbookResult<()> {
    /// let epub = Epub::open("tests/ebooks/example_epub")?;
    /// let metadata = epub.metadata();
    ///
    /// // Calling the trait method directly returns `Option<&str>`
    /// assert_eq!(Some("3.3"), Metadata::version_str(&metadata));
    /// // The inherent method `EpubMetadata::version_str` returns `&str` instead:
    /// assert_eq!("3.3", metadata.version_str());
    /// # Ok(())
    /// # }
    /// ```
    fn version_str(&self) -> Option<&'ebook str>;

    /// The version of an [`Ebook`](super::Ebook).
    ///
    /// # See Also
    /// - [`Self::version_str`] for the original representation.
    ///
    /// # Examples
    /// - Retrieving the version of an ebook in EPUB format:
    /// ```
    /// # use rbook::Epub;
    /// # use rbook::ebook::metadata::Metadata;
    /// # use rbook::ebook::metadata::Version;
    /// # use rbook::epub::metadata::EpubVersion;
    /// # fn main() -> rbook::ebook::errors::EbookResult<()> {
    /// let epub = Epub::open("tests/ebooks/example_epub")?;
    /// let metadata = epub.metadata();
    ///
    /// // Calling the trait method directly returns `Option<Version>`
    /// assert_eq!(Some(Version(3, 3)), Metadata::version(&metadata));
    /// // The inherent method `EpubMetadata::version` returns `EpubVersion` instead:
    /// assert_eq!(EpubVersion::from(Version(3, 3)), metadata.version());
    /// # Ok(())
    /// # }
    /// ```
    fn version(&self) -> Option<Version>;

    /// The publication date; when an [`Ebook`](super::Ebook) was published.
    ///
    /// # Examples
    /// - Retrieving the publication date:
    /// ```
    /// # use rbook::Epub;
    /// # fn main() -> rbook::ebook::errors::EbookResult<()> {
    /// let epub = Epub::open("tests/ebooks/epub2")?;
    /// let published = epub.metadata().published().unwrap();
    /// let entry = epub.metadata().published_entry().unwrap();
    /// let date = published.date();
    ///
    /// assert_eq!("2023-01-25", entry.value());
    /// assert_eq!(2023, date.year());
    /// assert_eq!(1, date.month());
    /// assert_eq!(25, date.day());
    /// # Ok(())
    /// # }
    /// ```
    fn published(&self) -> Option<datetime::DateTime>;

    /// The last modified date; when an [`Ebook`](super::Ebook) was last modified.
    ///
    /// # See Also
    /// - [`Self::published`] to retrieve the data en ebook was published.
    ///
    /// # Examples
    /// - Retrieving the modification date:
    /// ```
    /// # use rbook::Epub;
    /// # fn main() -> rbook::ebook::errors::EbookResult<()> {
    /// let epub = Epub::open("tests/ebooks/epub2")?;
    /// let modified = epub.metadata().modified().unwrap();
    /// let entry = epub.metadata().modified_entry().unwrap();
    ///
    /// assert_eq!("2025-11-27", entry.value());
    /// assert_eq!(2025, modified.date().year());
    /// assert_eq!(0, modified.time().hour());
    /// assert_eq!(false, modified.time().is_utc());
    /// # Ok(())
    /// # }
    /// ```
    fn modified(&self) -> Option<datetime::DateTime>;

    /// The main unique [`Identifier`] of an [`Ebook`](super::Ebook).
    fn identifier(&self) -> Option<impl Identifier<'ebook> + 'ebook>;

    /// Returns an iterator over **all** [identifiers](Identifier)
    /// by [`order`](MetaEntry::order).
    ///
    /// Note that the first entry may not be the main [`Identifier`],
    /// as depending on the ebook, the order of the main identifier may be greater than `0`.
    /// Generally, such a scenario is rare, although possible.
    ///
    /// # See Also
    /// - [`Self::identifier`] to get the main identifier, disregarding
    ///   [`order`](MetaEntry::order).
    fn identifiers(&self) -> impl Iterator<Item = impl Identifier<'ebook> + 'ebook> + 'ebook;

    /// The main [`Language`] with an [`order`](MetaEntry::order) of `0`.
    fn language(&self) -> Option<impl Language<'ebook> + 'ebook>;

    /// Returns an iterator over **all** [`Languages`](Language)
    /// by [`order`](MetaEntry::order).
    fn languages(&self) -> impl Iterator<Item = impl Language<'ebook> + 'ebook> + 'ebook;

    /// The main [`Title`].
    ///
    /// # See Also
    /// - [`Self::titles`] to retrieve all titles by [`order`](MetaEntry::order).
    ///
    /// # Examples
    /// - Retrieving the main title:
    /// ```
    /// # use rbook::Epub;
    /// # use rbook::ebook::metadata::TitleKind;
    /// # fn main() -> rbook::ebook::errors::EbookResult<()> {
    /// let epub = Epub::open("tests/ebooks/example_epub")?;
    /// let title = epub.metadata().title().unwrap();
    ///
    /// assert_eq!("Example EPUB", title.value());
    /// assert_eq!(TitleKind::Main, title.kind());
    /// assert_eq!(2, title.order());
    /// # Ok(())
    /// # }
    /// ```
    fn title(&self) -> Option<impl Title<'ebook> + 'ebook>;

    /// Returns an iterator over **all** [titles](Title)
    /// by [`order`](MetaEntry::order).
    ///
    /// Note that the first entry may not be the main [`Title`],
    /// as depending on the ebook, the order of the main title may be greater than `0`.
    /// Generally, such a scenario is rare, although possible.
    ///
    /// # See Also
    /// - [`Self::title`] to get the main title, disregarding [`order`](MetaEntry::order).
    /// - [`Title::kind`] to get the kind of title.
    ///
    /// # Examples
    /// - Retrieving the titles of an ebook:
    /// ```
    /// # use rbook::Epub;
    /// # use rbook::ebook::metadata::TitleKind;
    /// # fn main() -> rbook::ebook::errors::EbookResult<()> {
    /// let epub = Epub::open("tests/ebooks/example_epub")?;
    /// let mut titles = epub.metadata().titles();
    ///
    /// let title_a = titles.next().unwrap();
    /// assert_eq!("This is not the main title", title_a.value());
    /// assert_eq!(TitleKind::Unknown, title_a.kind());
    /// assert_eq!(0, title_a.order());
    ///
    /// let title_b = titles.next().unwrap();
    /// assert_eq!("A subtitle", title_b.value());
    /// assert_eq!(TitleKind::Subtitle, title_b.kind());
    /// assert_eq!(1, title_b.order());
    ///
    /// let title_c = titles.next().unwrap();
    /// assert_eq!("Example EPUB", title_c.value());
    /// assert_eq!(TitleKind::Main, title_c.kind());
    /// assert_eq!(2, title_c.order());
    /// # Ok(())
    /// # }
    /// ```
    fn titles(&self) -> impl Iterator<Item = impl Title<'ebook> + 'ebook> + 'ebook;

    /// The main description with an [`order`](MetaEntry::order) of `0`.
    fn description(&self) -> Option<impl MetaEntry<'ebook> + 'ebook>;

    /// Returns an iterator over **all** descriptions by [`order`](MetaEntry::order).
    fn descriptions(&self) -> impl Iterator<Item = impl MetaEntry<'ebook> + 'ebook> + 'ebook;

    /// Returns an iterator over **all** [creators](Contributor)
    /// by [`order`](MetaEntry::order).
    fn creators(&self) -> impl Iterator<Item = impl Contributor<'ebook> + 'ebook> + 'ebook;

    /// Returns an iterator over **all** [contributors](Contributor)
    /// by [`order`](MetaEntry::order).
    fn contributors(&self) -> impl Iterator<Item = impl Contributor<'ebook> + 'ebook> + 'ebook;

    /// Returns an iterator over **all** [publishers](Contributor)
    /// by [`order`](MetaEntry::order).
    fn publishers(&self) -> impl Iterator<Item = impl Contributor<'ebook> + 'ebook> + 'ebook;

    /// Returns an iterator over **all** generators.
    ///
    /// A generator indicates the software used to create an [`Ebook`](super::Ebook).
    fn generators(&self) -> impl Iterator<Item = impl MetaEntry<'ebook> + 'ebook> + 'ebook;

    /// Returns an iterator over **all** [tags](Tag)
    /// by [`order`](MetaEntry::order).
    fn tags(&self) -> impl Iterator<Item = impl Tag<'ebook> + 'ebook> + 'ebook;

    /// Returns an iterator over **all** metadata entries.
    fn iter(&self) -> impl Iterator<Item = impl MetaEntry<'ebook> + 'ebook> + 'ebook;
}

/// The scheme of metadata entries, specifying a registry [`source`](Scheme::source)
/// and [`code`](Scheme::code).
///
/// A `source` identifies "who" (such as an authority) that defines the code, such
/// as `BCP 47`, `BISAC`, and `marc:relators`.
///
/// Sources are optional and will not be specified if there is no known
/// registry for a `code`.
///
/// # Equality
/// Two schemes are equal if their [`source`](Scheme::source) and [`code`](Scheme::code)
/// are **case-sensitively** equal.
///
/// # Examples
/// - Retrieving the source and code:
/// ```
/// # use rbook::Epub;
/// # fn main() -> rbook::ebook::errors::EbookResult<()> {
/// let epub = Epub::open("tests/ebooks/example_epub")?;
/// let primary_language = epub.metadata().language().unwrap();
/// let scheme = primary_language.scheme();
///
/// assert_eq!(Some("BCP 47"), scheme.source());
/// assert_eq!("en", scheme.code());
/// # Ok(())
/// # }
/// ```
#[derive(Copy, Clone, Debug, Hash, PartialEq, Eq)]
pub struct Scheme<'ebook> {
    source: Option<&'ebook str>,
    code: &'ebook str,
}

impl<'ebook> Scheme<'ebook> {
    pub(crate) fn new(source: Option<&'ebook str>, code: &'ebook str) -> Self {
        Self { source, code }
    }

    /// The authority or registry that defines a [`code`](Self::code)
    /// (e.g. `BCP 47`, `BISAC` `marc:relators`), or [`None`] if unknown.
    pub fn source(&self) -> Option<&'ebook str> {
        self.source
    }

    /// The identification code (e.g., `FIC002000`, `zh-CN`).
    pub fn code(&self) -> &'ebook str {
        self.code
    }
}

/// The language tag, consisting of a [`Scheme`] and [`LanguageKind`].
///
/// Represents a language code (e.g. `en`, `ja`) alongside its source (e.g. `BCP 47`).
///
/// Unlike [`Language`], [`LanguageTag`] complements metadata entries
/// instead of specifying an ebook-wide language.
///
/// # See Also
/// - [`AlternateScript`]
#[derive(Copy, Clone, Debug, Hash, PartialEq, Eq)]
pub struct LanguageTag<'ebook> {
    scheme: Scheme<'ebook>,
    kind: LanguageKind,
}

impl<'ebook> LanguageTag<'ebook> {
    pub(crate) fn new(code: &'ebook str, kind: LanguageKind) -> Self {
        Self {
            scheme: Scheme::new((kind != LanguageKind::Unknown).then(|| kind.as_str()), code),
            kind,
        }
    }

    /// The [`Scheme`] that identifies the authority and language code.
    pub fn scheme(&self) -> Scheme<'ebook> {
        self.scheme
    }

    /// The language tag scheme kind (e.g., `BCP 47`).
    pub fn kind(&self) -> LanguageKind {
        self.kind
    }
}

/// Alternate script to portray an alternative form of textual content
/// in a different language or script.
///
/// # Examples
/// ```
/// # use rbook::Epub;
/// # use rbook::ebook::metadata::LanguageKind;
/// # fn main() -> rbook::ebook::errors::EbookResult<()> {
/// let epub = Epub::open("tests/ebooks/example_epub")?;
/// let title = epub.metadata().title().unwrap();
///
/// assert_eq!("Example EPUB", title.value());
///
/// let alternate_script = title.alternate_scripts().next().unwrap();
///
/// assert_eq!("サンプルEPUB", alternate_script.value());
/// assert_eq!("ja", alternate_script.language().scheme().code());
/// assert_eq!(LanguageKind::Bcp47, alternate_script.language().kind());
/// # Ok(())
/// # }
/// ```
pub struct AlternateScript<'ebook> {
    script: &'ebook str,
    tag: LanguageTag<'ebook>,
}

impl<'ebook> AlternateScript<'ebook> {
    pub(crate) fn new(script: &'ebook str, tag: LanguageTag<'ebook>) -> Self {
        Self { script, tag }
    }

    /// The alternate form text value.
    pub fn value(&self) -> &'ebook str {
        self.script
    }

    /// The language tag associated with the alternate form.
    pub fn language(&self) -> LanguageTag<'ebook> {
        self.tag
    }
}

/// A [`Metadata`] entry containing information associated with an [`Ebook`](super::Ebook).
///
/// This trait provides access to details such as its [`value`](MetaEntry::value),
/// [`order`](MetaEntry::order), [`sort key`](MetaEntry::file_as), and
/// [`alternate scripts`](MetaEntry::alternate_scripts).
///
/// # See Also
/// - [`EpubMetaEntry`](crate::epub::metadata::EpubMetaEntry) for epub-specific entry information.
pub trait MetaEntry<'ebook>: Sealed {
    /// The plain text value of an entry.
    ///
    /// # Example
    /// - Retrieving the value of a description:
    /// ```
    /// # use rbook::Epub;
    /// # fn main() -> rbook::ebook::errors::EbookResult<()> {
    /// let epub = Epub::open("tests/ebooks/example_epub")?;
    /// let description = epub.metadata().description().unwrap();
    ///
    /// assert_eq!(
    ///     "Hello world! CData 1. A descriptive statement. CData 2. Another statement !",
    ///     description.value(),
    /// );
    /// # Ok(())
    /// # }
    /// ```
    fn value(&self) -> &'ebook str;

    /// The (0-based) order/display-sequence of an entry relative to another associated entry.
    ///
    /// For example, if there are multiple creators, this field indicates who
    /// is ordered before one another.
    ///
    /// A value of `0` means first, `1` means second, and so on.
    ///
    /// # Example
    /// - Retrieving the order of tags:
    /// ```
    /// # use rbook::Epub;
    /// # fn main() -> rbook::ebook::errors::EbookResult<()> {
    /// let epub = Epub::open("tests/ebooks/example_epub")?;
    /// let mut tags = epub.metadata().tags();
    ///
    /// let tag_a = tags.next().unwrap();
    /// assert_eq!("FICTION / Occult & Supernatural", tag_a.value());
    /// assert_eq!(0, tag_a.order());
    ///
    /// let tag_b = tags.next().unwrap();
    /// assert_eq!("Quests (Expeditions) -- Fiction", tag_b.value());
    /// assert_eq!(1, tag_b.order());
    ///
    /// let tag_c = tags.next().unwrap();
    /// assert_eq!("Fantasy", tag_c.value());
    /// assert_eq!(2, tag_c.order());
    /// # Ok(())
    /// # }
    /// ```
    fn order(&self) -> usize;

    /// The `file-as` sort key, if present.
    ///
    /// # Example
    /// - Retrieving the sort key:
    /// ```
    /// # use rbook::Epub;
    /// # fn main() -> rbook::ebook::errors::EbookResult<()> {
    /// let epub = Epub::open("tests/ebooks/example_epub")?;
    ///
    /// let creator = epub.metadata().creators().next().unwrap();
    /// assert_eq!("John Doe", creator.value());
    /// assert_eq!(Some("Doe, John"), creator.file_as());
    /// # Ok(())
    /// # }
    /// ```
    fn file_as(&self) -> Option<&'ebook str>;

    /// Returns an iterator over **all** [`AlternateScript`].
    ///
    /// Alternate script is an **alternative** form of [`Self::value`]
    /// in a different language or script.
    fn alternate_scripts(&self) -> impl Iterator<Item = AlternateScript<'ebook>> + 'ebook;
}

/// A language that an [`Ebook`](super::Ebook) supports.
///
/// Provides both the raw scheme string and a parsed kind.
///
/// # See Also
/// - [`EpubLanguage`](crate::epub::metadata::EpubLanguage) for epub-specific information.
///
/// # Examples
/// - Retrieving a language's kind and scheme:
/// ```
/// # use rbook::Epub;
/// # use rbook::ebook::metadata::LanguageKind;
/// # fn main() -> rbook::ebook::errors::EbookResult<()> {
/// let epub = Epub::open("tests/ebooks/example_epub")?;
/// let language = epub.metadata().language().unwrap();
///
/// assert_eq!(LanguageKind::Bcp47, language.kind());
/// assert_eq!("en", language.value());
///
/// let scheme = language.scheme();
///
/// assert_eq!(Some("BCP 47"), scheme.source());
/// assert_eq!("en", scheme.code());
/// # Ok(())
/// # }
/// ```
pub trait Language<'ebook>: MetaEntry<'ebook> {
    /// The language's scheme, such as `BCP 47`.
    ///
    /// This is a lower-level call than [`Self::kind`] to retrieve the raw string value.
    fn scheme(&self) -> Scheme<'ebook>;

    /// The language kind enum.
    ///
    /// If [`LanguageKind::Unknown`] is returned, [`Self::scheme`]
    /// can be used to retrieve the string value of the unknown language kind.
    fn kind(&self) -> LanguageKind;
}

/// A unique identifier for an [`Ebook`](super::Ebook), such as `ISBN`, `DOI`, and `URL`.
///
/// # See Also
/// - [`EpubIdentifier`](crate::epub::metadata::EpubIdentifier) for epub-specific information.
///
/// # Examples
/// - Retrieving the main identifier:
/// ```
/// # use rbook::Epub;
/// # fn main() -> rbook::ebook::errors::EbookResult<()> {
/// let epub = Epub::open("tests/ebooks/example_epub")?;
/// let identifier = epub.metadata().identifier().unwrap();
/// let scheme = identifier.scheme().unwrap();
///
/// assert_eq!("https://github.com/devinsterling/rbook", identifier.value());
/// assert_eq!("URL", scheme.code());
/// assert_eq!(None, scheme.source());
/// # Ok(())
/// # }
/// ```
pub trait Identifier<'ebook>: MetaEntry<'ebook> + Eq + Hash {
    /// The identifier’s scheme or [`None`] if unspecified.
    fn scheme(&self) -> Option<Scheme<'ebook>>;
}

/// A title of an [`Ebook`](super::Ebook).
///
/// Titles may have an optional scheme for further categorization (e.g. `subtitle`, `edition`).
///
/// # See Also
/// - [`EpubTitle`](crate::epub::metadata::EpubTitle) for epub-specific information.
///
/// # Examples
/// - Retrieving a title's kind:
/// ```
/// # use rbook::Epub;
/// # use rbook::ebook::metadata::TitleKind;
/// # fn main() -> rbook::ebook::errors::EbookResult<()> {
/// let epub = Epub::open("tests/ebooks/example_epub")?;
/// let subtitle = epub.metadata().titles().nth(1).unwrap();
///
/// assert_eq!(TitleKind::Subtitle, subtitle.kind());
///
/// let scheme = subtitle.scheme().unwrap();
///
/// assert_eq!(None, scheme.source());
/// assert_eq!("subtitle", scheme.code());
/// # Ok(())
/// # }
/// ```
pub trait Title<'ebook>: MetaEntry<'ebook> {
    /// The title’s scheme or [`None`] if unspecified.
    ///
    /// This is a lower-level call than [`Self::kind`] to retrieve the raw string value, if any.
    fn scheme(&self) -> Option<Scheme<'ebook>>;

    /// The kind of title.
    ///
    /// If [`TitleKind::Unknown`] is returned, [`Self::scheme`]
    /// can be used to retrieve the string value of the unknown title kind, if present.
    fn kind(&self) -> TitleKind;
}

/// A tag that categorizes an [`Ebook`](super::Ebook).
///
/// # See Also
/// - [`EpubTag`](crate::epub::metadata::EpubTag) for epub-specific information.
///
/// # Examples
/// - Retrieving all tags:
/// ```
/// # use rbook::Epub;
/// # fn main() -> rbook::ebook::errors::EbookResult<()> {
/// let epub = Epub::open("tests/ebooks/example_epub")?;
/// let tags = epub.metadata().tags().collect::<Vec<_>>();
///
/// // Book Industry Standards and Communications (BISAC) tag
/// let bisac = tags[0].scheme().unwrap();
/// assert_eq!("FICTION / Occult & Supernatural", tags[0].value());
/// assert_eq!("FIC024000", bisac.code());
/// assert_eq!(Some("BISAC"), bisac.source());
///
/// // The Library of Congress Subject Headings (LCSH) tag
/// let lcsh = tags[1].scheme().unwrap();
/// assert_eq!("Quests (Expeditions) -- Fiction", tags[1].value());
/// assert_eq!("sh2008110314", lcsh.code());
/// assert_eq!(Some("LCSH"), lcsh.source());
///
/// // Plain tag (No scheme specified)
/// assert_eq!("Fantasy", tags[2].value());
/// assert_eq!(None, tags[2].scheme());
/// # Ok(())
/// # }
/// ```
pub trait Tag<'ebook>: MetaEntry<'ebook> {
    /// The tag’s scheme or [`None`] if unspecified.
    fn scheme(&self) -> Option<Scheme<'ebook>>;
}

/// Individuals or organizations that helped with the creation of an [`Ebook`](super::Ebook),
/// such as, `authors`, `illustrators`, and `publishers`.
///
/// # See Also
/// - [`EpubContributor`](crate::epub::metadata::EpubContributor) for epub-specific information.
///
/// # Examples
/// - Retrieving an author:
/// ```
/// # use rbook::Epub;
/// # fn main() -> rbook::ebook::errors::EbookResult<()> {
/// let epub = Epub::open("tests/ebooks/example_epub")?;
/// let author = epub.metadata().creators().next().unwrap();
/// let role = author.main_role().unwrap();
///
/// assert_eq!("John Doe", author.value());
/// assert_eq!("aut", role.code());
/// assert_eq!(Some("marc:relators"), role.source());
/// # Ok(())
/// # }
/// ```
pub trait Contributor<'ebook>: MetaEntry<'ebook> {
    /// The primary role of a contributor or [`None`] if unspecified.
    ///
    /// For example, a contributor could be the `author` and `illustrator` of an ebook.
    /// However, their main role would remain as the `author`.
    fn main_role(&self) -> Option<Scheme<'ebook>>;

    /// Returns an iterator over **all** roles by the order of importance (display sequence).
    fn roles(&self) -> impl Iterator<Item = Scheme<'ebook>> + 'ebook;
}

/// The kind of `language`.
#[non_exhaustive]
#[derive(Copy, Clone, Debug, Hash, PartialEq, Eq)]
pub enum LanguageKind {
    /// `BCP 47`: See <https://www.rfc-editor.org/info/bcp47>.
    Bcp47,
    // ISO639,
    /// An unknown language kind.
    Unknown,
}

impl LanguageKind {
    /// Returns the string form of a [`LanguageKind`].
    ///
    /// # Examples
    /// - Retrieving the string form:
    /// ```
    /// # use rbook::ebook::metadata::LanguageKind;
    /// assert_eq!("BCP 47", LanguageKind::Bcp47.as_str());
    /// assert_eq!("unknown", LanguageKind::Unknown.as_str());
    /// ```
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::Bcp47 => "BCP 47",
            Self::Unknown => "unknown",
        }
    }
}

impl Display for LanguageKind {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(self.as_str())
    }
}

/// The kind of [`Title`].
#[non_exhaustive]
#[derive(Copy, Clone, Debug, Hash, PartialEq, Eq)]
pub enum TitleKind {
    /// The primary main title.
    Main,
    /// The secondary title (subtitle).
    Subtitle,
    /// A shortened version of the main title.
    Short,
    /// An expanded detailed version of the main title.
    Expanded,
    /// The title of a collection that an [`Ebook`](super::Ebook) belongs.
    Collection,
    /// The title of a particular edition.
    Edition,
    /// An unrecognized title kind.
    Unknown,
}

impl TitleKind {
    const MAIN: &'static str = "main";
    const SUBTITLE: &'static str = "subtitle";
    const SHORT: &'static str = "short";
    const EXPANDED: &'static str = "expanded";
    const COLLECTION: &'static str = "collection";
    const EDITION: &'static str = "edition";

    // **For now**, there is no public From<&str>/as_str method for TitleKind because
    // other ebook formats may have different (and potentially conflicting)
    // mappings (e.g., main-title, primary, etc.)
    pub(super) fn from(kind: &str) -> Self {
        match kind {
            Self::MAIN => Self::Main,
            Self::SUBTITLE => Self::Subtitle,
            Self::SHORT => Self::Short,
            Self::COLLECTION => Self::Collection,
            Self::EDITION => Self::Edition,
            Self::EXPANDED => Self::Expanded,
            _ => Self::Unknown,
        }
    }

    #[cfg(feature = "write")]
    pub(super) fn as_str(&self) -> Option<&'static str> {
        match self {
            Self::Main => Some(Self::MAIN),
            Self::Subtitle => Some(Self::SUBTITLE),
            Self::Short => Some(Self::SHORT),
            Self::Collection => Some(Self::COLLECTION),
            Self::Edition => Some(Self::EDITION),
            Self::Expanded => Some(Self::EXPANDED),
            _ => None,
        }
    }

    /// Returns `true` if the title kind is [`TitleKind::Main`]
    pub fn is_main(&self) -> bool {
        matches!(self, Self::Main)
    }

    /// Returns `true` if the title kind is [`TitleKind::Subtitle`]
    pub fn is_subtitle(&self) -> bool {
        matches!(self, Self::Subtitle)
    }

    /// Returns `true` if the title kind is [`TitleKind::Short`]
    pub fn is_short(&self) -> bool {
        matches!(self, Self::Short)
    }

    /// Returns `true` if the title kind is [`TitleKind::Collection`]
    pub fn is_collection(&self) -> bool {
        matches!(self, Self::Collection)
    }

    /// Returns `true` if the title kind is [`TitleKind::Edition`]
    pub fn is_edition(&self) -> bool {
        matches!(self, Self::Edition)
    }

    /// Returns `true` if the title kind is [`TitleKind::Expanded`]
    pub fn is_expanded(&self) -> bool {
        matches!(self, Self::Expanded)
    }

    /// Returns `true` if the title kind is [`TitleKind::Unknown`]
    pub fn is_unknown(&self) -> bool {
        matches!(self, Self::Unknown)
    }
}

/// The version of an [`Ebook`](super::Ebook) format (e.g. `3.3`).
///
/// A version consists of:
/// - [`Major`](Version::major): Paradigm-shifting release.
/// - [`Minor`](Version::minor): Backwards-compatible, gradual update.
///
/// # Examples
/// - Retrieving the major and minor of a version:
/// ```
/// use rbook::ebook::metadata::Version;
///
/// let version = Version(2, 8);
/// assert_eq!(2, version.major());
/// assert_eq!(8, version.minor());
/// ```
#[derive(Copy, Clone, Debug, Hash, PartialEq, Eq, PartialOrd, Ord)]
pub struct Version(
    /// Paradigm-shifting release.
    pub u16,
    /// Backwards-compatible, gradual update.
    pub u16,
);

impl Version {
    pub(crate) fn from_str(version: &str) -> Option<Self> {
        let mut components = version.trim().split('.').map(str::parse);

        Some(Self(
            // Required
            components.next()?.ok()?,
            // Optional
            components.next().unwrap_or(Ok(0)).ok()?,
        ))
    }

    /// Paradigm-shifting release number.
    pub fn major(&self) -> u16 {
        self.0
    }

    /// Backwards-compatible, gradual update number.
    pub fn minor(&self) -> u16 {
        self.1
    }
}

impl From<u16> for Version {
    fn from(version: u16) -> Self {
        Self(version, 0)
    }
}

impl Display for Version {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}.{}", self.0, self.1)
    }
}

#[cfg(test)]
mod tests {
    use crate::ebook::metadata::Version;

    #[test]
    fn test_version_from_str() {
        let expected = [
            ("2.0", Some(Version(2, 0))),
            ("3.1", Some(Version(3, 1))),
            ("3", Some(Version(3, 0))),
            (" 3.2 ", Some(Version(3, 2))),
            ("", None),
            ("x.y", None),
            ("2.3-", None),
        ];

        for (raw, expected_version) in expected {
            assert_eq!(expected_version, Version::from_str(raw));
        }
    }
}
