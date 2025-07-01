//! Format-agnostic [`Metadata`]-related content.

use std::fmt::{Display, Formatter};
use std::hash::Hash;

/// The metadata of an [`Ebook`](super::Ebook), encompassing detailed information,
/// such as the [`Version`], [`Title`], and [`Identifier`].
///
/// # Examples
/// - Retrieving the [`author`](Metadata::creators) and [`subtitle`](Metadata::title):
/// ```
/// # use rbook::{Ebook, Epub};
/// # use rbook::ebook::metadata::{Metadata, MetaEntry, Title, TitleKind};
/// # use rbook::ebook::errors::EbookResult;
/// # fn main() -> EbookResult<()> {
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
/// # Ok(())
/// # }
/// ```
pub trait Metadata<'ebook> {
    /// The version of an [`ebook's`](super::Ebook) format in the form of a string.
    ///
    /// # Examples
    /// - Retrieving the version of an ebook in EPUB format:
    /// ```
    /// # use rbook::ebook::errors::EbookResult;
    /// # use rbook::ebook::metadata::Metadata;
    /// # use rbook::{Ebook, Epub};
    /// # fn main() -> EbookResult<()> {
    /// let epub = Epub::open("tests/ebooks/example_epub")?;
    /// let metadata = epub.metadata();
    ///
    /// assert_eq!(Some("3.0"), Metadata::version_str(&metadata));
    /// # Ok(())
    /// # }
    /// ```
    /// See [`Self::version`] for the parsed representation.
    fn version_str(&self) -> Option<&'ebook str>;

    /// The version of an [`Ebook`](super::Ebook).
    ///
    /// # Examples
    /// - Retrieving the version of an ebook in EPUB format:
    /// ```
    /// # use rbook::ebook::errors::EbookResult;
    /// # use rbook::ebook::metadata::{Metadata, Version};
    /// # use rbook::{Ebook, Epub};
    /// # fn main() -> EbookResult<()> {
    /// let epub = Epub::open("tests/ebooks/example_epub")?;
    /// let metadata = epub.metadata();
    ///
    /// assert_eq!(Some(Version(3, 0)), Metadata::version(&metadata));
    /// # Ok(())
    /// # }
    /// ```
    /// See [`Self::version_str`] for the original representation.
    fn version(&self) -> Option<Version>;

    /// The publication date; when an [`Ebook`](super::Ebook) was published.
    fn publication_date(&self) -> Option<DateTime<'ebook>>;

    /// The last modified date; when an [`Ebook`](super::Ebook) was last modified.
    ///
    /// See also: [`Self::publication_date`]
    fn modified_date(&self) -> Option<DateTime<'ebook>>;

    /// The primary unique [`Identifier`] tied to an [`Ebook`](super::Ebook).
    fn identifier(&self) -> Option<impl Identifier<'ebook>>;

    /// Returns an iterator over **all** [`Identifiers`](Identifier)
    /// by [`order`](MetaEntry::order).
    fn identifiers(&self) -> impl Iterator<Item = impl Identifier<'ebook>> + 'ebook;

    /// The main [`Language`].
    fn language(&self) -> Option<impl Language<'ebook>>;

    /// Returns an iterator over **all** [`Languages`](Language)
    /// by [`order`](MetaEntry::order).
    fn languages(&self) -> impl Iterator<Item = impl Language<'ebook>> + 'ebook;

    /// The main [`Title`].
    ///
    /// See [`Self::titles`] to retrieve all titles by [`order`](MetaEntry::order).
    fn title(&self) -> Option<impl Title<'ebook>>;

    /// Returns an iterator over **all** [`Titles`](Title)
    /// by [`order`](MetaEntry::order).
    ///
    /// Note that the first entry may not be the main [`Title`],
    /// as depending on the ebook, it could have a display order greater than `1`.
    ///
    /// To get the main title, disregarding display order, use [`Self::title`].
    fn titles(&self) -> impl Iterator<Item = impl Title<'ebook>> + 'ebook;

    /// The main description of an [`Ebook`](super::Ebook).
    fn description(&self) -> Option<impl MetaEntry<'ebook>>;

    /// Returns an iterator over **all** descriptions by [`order`](MetaEntry::order).
    fn descriptions(&self) -> impl Iterator<Item = impl MetaEntry<'ebook>> + 'ebook;

    /// Returns an iterator over **all** [`Creators`](Contributor)
    /// by [`order`](MetaEntry::order).
    fn creators(&self) -> impl Iterator<Item = impl Contributor<'ebook>> + 'ebook;

    /// Returns an iterator over **all** [`Contributors`](Contributor)
    /// by [`order`](MetaEntry::order).
    fn contributors(&self) -> impl Iterator<Item = impl Contributor<'ebook>> + 'ebook;

    /// Returns an iterator over **all** [`Publishers`](Contributor)
    /// by [`order`](MetaEntry::order).
    fn publishers(&self) -> impl Iterator<Item = impl Contributor<'ebook>> + 'ebook;

    /// Returns an iterator over **all** [`Tags`](Tag)
    /// by [`order`](MetaEntry::order).
    fn tags(&self) -> impl Iterator<Item = impl Tag<'ebook>> + 'ebook;

    /// Returns an iterator over **all** metadata entries.
    fn entries(&self) -> impl Iterator<Item = impl MetaEntry<'ebook>> + 'ebook;
}

/// The scheme of metadata entries, specifying a registry [`source`](Scheme::source)
/// and [`code`](Scheme::code).
///
/// A `source` identifies "who" (such as an authority) defines the `code`, such
/// as `BCP 47`, `BISAC`, and `marc:relators`.
///
/// Sources are optional and will not be specified if there is no known
/// registry for a `code`.
///
/// # Examples
/// - Retrieving the source and code:
/// ```
/// # use rbook::ebook::errors::EbookResult;
/// # use rbook::ebook::metadata::{Metadata, Language};
/// # use rbook::{Ebook, Epub};
/// # fn main() -> EbookResult<()> {
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
/// See also: [`AlternateScript`]
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
/// # use rbook::ebook::errors::EbookResult;
/// # use rbook::ebook::metadata::{Metadata, MetaEntry, LanguageKind};
/// # use rbook::{Ebook, Epub};
/// # fn main() -> EbookResult<()> {
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
pub trait MetaEntry<'ebook> {
    /// The text value of an entry.
    ///
    /// # Example
    /// - Retrieving the value of a description:
    /// ```
    /// # use rbook::ebook::errors::EbookResult;
    /// # use rbook::ebook::metadata::{Metadata, MetaEntry};
    /// # use rbook::{Ebook, Epub};
    /// # fn main() -> EbookResult<()> {
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
    /// # use rbook::ebook::errors::EbookResult;
    /// # use rbook::ebook::metadata::{Metadata, MetaEntry};
    /// # use rbook::{Ebook, Epub};
    /// # fn main() -> EbookResult<()> {
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
    /// # use rbook::ebook::errors::EbookResult;
    /// # use rbook::ebook::metadata::{Metadata, MetaEntry};
    /// # use rbook::{Ebook, Epub};
    /// # fn main() -> EbookResult<()> {
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
/// # Examples
/// - Retrieving a language's kind and scheme:
/// ```
/// # use rbook::ebook::errors::EbookResult;
/// # use rbook::ebook::metadata::{Metadata, Language, LanguageKind};
/// # use rbook::{Ebook, Epub};
/// # fn main() -> EbookResult<()> {
/// use rbook::ebook::metadata::MetaEntry;
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
/// # Examples
/// - Retrieving an identifier:
/// ```
/// # use rbook::ebook::errors::EbookResult;
/// # use rbook::ebook::metadata::{Metadata, MetaEntry, Identifier};
/// # use rbook::{Ebook, Epub};
/// # fn main() -> EbookResult<()> {
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
/// # Examples
/// - Retrieving a title's kind:
/// ```
/// # use rbook::ebook::errors::EbookResult;
/// # use rbook::ebook::metadata::{Metadata, Title, TitleKind};
/// # use rbook::{Ebook, Epub};
/// # fn main() -> EbookResult<()> {
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
    /// This is a lower-level call than [`Self::kind`] to retrieve the raw string value.
    fn scheme(&self) -> Option<Scheme<'ebook>>;

    /// The title kind enum.
    ///
    /// If [`TitleKind::Unknown`] is returned, [`Self::scheme`]
    /// can be used to retrieve the string value of the unknown title kind, if present.
    fn kind(&self) -> TitleKind;
}

/// A tag that categorizes an [`Ebook`](super::Ebook).
///
/// # Examples
/// - Retrieving all tags:
/// ```
/// # use rbook::ebook::errors::EbookResult;
/// # use rbook::ebook::metadata::{Metadata, MetaEntry, Tag};
/// # use rbook::{Ebook, Epub};
/// # fn main() -> EbookResult<()> {
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
/// # Examples
/// - Retrieving an author:
/// ```
/// # use rbook::ebook::errors::EbookResult;
/// # use rbook::ebook::metadata::{Metadata, MetaEntry, Contributor};
/// # use rbook::{Ebook, Epub};
/// # fn main() -> EbookResult<()> {
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
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
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
    /// An expanded more detailed version of the main title.
    Expanded,
    /// The title of a collection that an [`Ebook`](super::Ebook) belongs.
    Collection,
    /// The title of a particular edition.
    Edition,
    /// An unrecognized title kind.
    Unknown,
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
    pub(crate) fn from_str(version: &str) -> Option<Version> {
        let mut components = version.trim().split('.').map(|x| x.parse());

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

impl Display for Version {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}.{}", self.0, self.1)
    }
}

/// A (non-parsed) datetime from an [`Ebook`](super::Ebook),
/// typically in `ISO-8601-1` format.
///
/// [`DateTime::as_str`] may provide dates in different formats:
/// - `2025-12-01` (ISO-8601-1)
/// - `2025-12-01T00:00:00Z` (ISO-8601-1)
///
/// Rare and generally not recommended, although possible:
/// - `December 2025`
/// - `1.12.2025`
#[derive(Copy, Clone, Debug, PartialEq)]
pub struct DateTime<'ebook>(
    /// Wraps a &str because the format of a date from an ebook is unreliable
    &'ebook str,
);

impl<'ebook> DateTime<'ebook> {
    pub(crate) fn new(datetime: &'ebook str) -> Self {
        Self(datetime)
    }

    /// The raw date string.
    ///
    /// See the [`DateTime`] struct-level doc for more details.
    ///
    /// # Examples
    /// - Retrieving the publication date of an ebook:
    /// ```
    /// # use rbook::ebook::errors::EbookResult;
    /// # use rbook::ebook::metadata::{Metadata, Version};
    /// # use rbook::{Ebook, Epub};
    /// # fn main() -> EbookResult<()> {
    /// let epub = Epub::open("tests/ebooks/example_epub")?;
    /// let date = epub.metadata().publication_date().unwrap();
    ///
    /// assert_eq!("2023-01-25", date.as_str());
    /// # Ok(())
    /// # }
    /// ```
    pub fn as_str(&self) -> &'ebook str {
        self.0
    }
}

impl<'ebook> AsRef<str> for DateTime<'ebook> {
    fn as_ref(&self) -> &'ebook str {
        self.0
    }
}

impl Display for DateTime<'_> {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        f.write_str(self.as_str())
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
