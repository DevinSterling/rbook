//! Format-agnostic table-of-contents; [`Toc`]-related content.

use crate::ebook::manifest::ManifestEntry;
use crate::ebook::resource::Resource;
use crate::ebook::toc::macros::toc_entry_kind;
use std::borrow::Cow;
use std::fmt::{Display, Formatter};

/// The table of contents, aiding navigation throughout an ebook [`Ebook`](super::Ebook).
///
/// Each [`TocEntry`] returned by [`Toc`] is a top-level root containing
/// [`TocEntry::children`].
///
/// The methods [`Self::by_kind`] and [`Self::kinds`] can be used to retrieve TOC variants,
/// such as [`landmarks`](TocEntryKind::Landmarks), [`page-list`](TocEntryKind::PageList), etc.
///
/// # See Also
/// - [`EpubToc`](crate::epub::toc::EpubToc) for epub-specific table of contents information.
///
/// # Examples
/// - Iterating over the table of contents:
/// ```
/// # use rbook::ebook::errors::EbookResult;
/// # use rbook::ebook::toc::{Toc, TocChildren, TocEntry};
/// # use rbook::{Ebook, Epub};
/// # fn main() -> EbookResult<()> {
/// let epub = Epub::open("tests/ebooks/example_epub")?;
/// let root = epub.toc().contents().unwrap();
/// let mut children = root.children().iter();
///
/// // A for loop may also be used alternatively
/// assert_eq!("The Cover", children.next().unwrap().label());
/// assert_eq!("rbook Chapter 1", children.next().unwrap().label());
/// assert_eq!("rbook Chapter 2", children.next().unwrap().label());
/// assert_eq!(None, children.next());
/// # Ok(())
/// # }
/// ```
pub trait Toc<'ebook> {
    /// Returns the **root** [`TocEntry`] of the primary TOC, or [`None`] if it does not exist.
    ///
    /// See the [trait-level example](Toc) for how to traverse the hierarchy.
    fn contents(&self) -> Option<impl TocEntry<'ebook> + 'ebook>;

    /// Returns the **root** [`TocEntry`] for the given [`TocEntryKind`],
    /// or [`None`] if it does not exist.
    ///
    /// # Examples
    /// - Retrieving different table of contents by kind:
    /// ```
    /// # use rbook::ebook::errors::EbookResult;
    /// # use rbook::ebook::toc::{Toc, TocEntryKind};
    /// # use rbook::{Ebook, Epub};
    /// # fn main() -> EbookResult<()> {
    /// let epub = Epub::open("tests/ebooks/example_epub")?;
    /// let toc = epub.toc();
    ///
    /// // Providing a string as input:
    /// let contents = toc.by_kind("toc");
    /// // Providing an enum as input:
    /// let landmarks = toc.by_kind(TocEntryKind::Landmarks);
    ///
    /// assert_eq!(contents, toc.by_kind(TocEntryKind::Toc));
    /// assert_eq!(landmarks, toc.by_kind("landmarks"));
    /// assert_eq!(None, toc.by_kind(TocEntryKind::PageList));
    /// # Ok(())
    /// # }
    /// ```
    fn by_kind(
        &self,
        kind: impl Into<TocEntryKind<'ebook>>,
    ) -> Option<impl TocEntry<'ebook> + 'ebook>;

    /// Returns an iterator over all **root** [`entries`](TocEntry).
    /// Each `Item` within the iterator is a tuple containing the
    /// `toc kind` and `root toc entry`.
    ///
    /// Tuple structure: ([`TocEntryKind`], [`TocEntry`])
    fn kinds(
        &self,
    ) -> impl Iterator<Item = (&'ebook TocEntryKind<'ebook>, impl TocEntry<'ebook> + 'ebook)> + 'ebook;
}

/// An entry contained within a [`Toc`], encompassing associated metadata.
///
/// # See Also
/// - [`EpubTocEntry`](crate::epub::toc::EpubTocEntry) for epub-specific entry information.
pub trait TocEntry<'ebook> {
    /// The display order of an entry (`0 = first item`).
    fn order(&self) -> usize;

    /// The depth of an entry relative to the root ([`0 = root`](Self::is_root)).
    fn depth(&self) -> usize;

    /// The human-readable label.
    fn label(&self) -> &'ebook str;

    /// The semantic kind of content associated with an entry.
    ///
    /// For example, an entry may point to the
    /// [`appendix`](TocEntryKind::Appendix) or [`cover page`](TocEntryKind::Cover).
    fn kind(&self) -> &'ebook TocEntryKind<'ebook>;

    /// The nested children (toc entries) associated with an entry.
    fn children(&self) -> impl TocChildren<'ebook> + 'ebook;

    /// The [`ManifestEntry`] associated with a [`TocEntry`].
    ///
    /// Returns [`None`] if the toc entry references a non-existent
    /// [`ManifestEntry`] within the [`Manifest`](super::Manifest).
    fn manifest_entry(&self) -> Option<impl ManifestEntry<'ebook> + 'ebook>;

    /// The [`Resource`] intended to navigate from an entry.
    ///
    /// # Examples
    /// - Retrieving the resource associated with an entry:
    /// ```
    /// # use rbook::ebook::errors::EbookResult;
    /// # use rbook::ebook::toc::{Toc, TocEntry};
    /// # use rbook::{Ebook, Epub};
    /// # fn main() -> EbookResult<()> {
    /// let epub = Epub::open("tests/ebooks/example_epub")?;
    /// let main_toc_root = epub.toc().contents().unwrap();
    ///
    /// // Root has no associated resource
    /// assert_eq!(None, main_toc_root.resource());
    ///
    /// for child in main_toc_root {
    ///     let resource = child.resource().unwrap();
    ///     assert_eq!("application/xhtml+xml", resource.kind().as_str());
    ///     
    ///     let content = epub.read_resource_str(resource)?;
    ///     // process content //
    /// }
    /// # Ok(())
    /// # }
    /// ```
    fn resource(&self) -> Option<Resource<'ebook>> {
        self.manifest_entry().map(|entry| entry.resource())
    }

    /// Returns `true` if the depth of a toc entry is `0`, indicating the root.
    ///
    /// # Examples
    /// - Assessing if an entry is a root:
    /// ```
    /// # use rbook::ebook::errors::EbookResult;
    /// # use rbook::ebook::toc::{Toc, TocEntry};
    /// # use rbook::{Ebook, Epub};
    /// # fn main() -> EbookResult<()> {
    /// let epub = Epub::open("tests/ebooks/example_epub")?;
    /// let main_toc_root = epub.toc().contents().unwrap();
    ///
    /// assert!(main_toc_root.is_root());
    ///
    /// for child in main_toc_root {
    ///     // Immediate children are never roots:
    ///     assert!(!child.is_root());
    /// }
    /// # Ok(())
    /// # }
    /// ```
    fn is_root(&self) -> bool {
        self.depth() == 0
    }

    /// Calculates and returns the **maximum** depth relative to an entry.
    /// In other words, how many levels deep is the most-nested child?
    ///
    /// Child [`entries`](TocEntry) have a maximum depth less than the parent.
    /// For example, if an entry has a maximum depth of `5`,
    /// then its immediate children will have a maximum depth of **at most** `4`.
    ///
    /// # Scenarios
    /// The maximum depth indicates the following:
    ///
    /// | Max Depth | Indication                                                         |
    /// |-----------|--------------------------------------------------------------------|
    /// | 0         | No immediate children (Equivalent to [`TocChildren::is_empty`]).   |
    /// | 1         | Only immediate children (Children do not contain nested children). |
    /// | \>1       | At least one immediate child contains nested children.             |
    ///
    /// # See Also
    /// - [`Self::depth`] for the pre-computed depth relative to the root.
    ///
    /// # Examples
    /// - Comparing the calculated maximum depth with [`Self::depth`]:
    /// ```
    /// # use rbook::ebook::errors::EbookResult;
    /// # use rbook::ebook::toc::{Toc, TocEntry, TocChildren};
    /// # use rbook::{Ebook, Epub};
    /// # fn main() -> EbookResult<()> {
    /// let epub = Epub::open("tests/ebooks/example_epub")?;
    /// let main_toc_root = epub.toc().contents().unwrap();
    ///
    /// // Current depth relative to the root
    /// assert_eq!(0, main_toc_root.depth());
    /// // Calculated maximum depth - deepest child entry within the hierarchy
    /// assert_eq!(2, main_toc_root.max_depth());
    ///
    /// let child = main_toc_root.children().get(0).unwrap();
    ///
    /// // Current depth relative to the root
    /// assert_eq!(1, child.depth());
    /// // Calculated maximum depth - `child` entry has no children
    /// assert_eq!(0, child.max_depth());
    /// # Ok(())
    /// # }
    /// ```
    fn max_depth(&self) -> usize {
        self.children()
            .iter()
            .fold(0, |depth, child| depth.max(1 + child.max_depth()))
    }

    /// Calculates and returns the **total** number of all (immediate and nested)
    /// children relative to an entry.
    ///
    /// # Scenarios
    /// The total number of children indicates the following:
    ///
    /// | Total Children         | Indication                                                         |
    /// |------------------------|--------------------------------------------------------------------|
    /// | 0                      | No immediate children (Equivalent to [`TocChildren::is_empty`]).   |
    /// | [`TocChildren::len`]   | Only immediate children (Children do not contain nested children). |
    /// | \>[`TocChildren::len`] | At least one immediate child contains nested children.             |
    ///
    /// # Examples
    /// - Comparing the calculated total length with [`TocChildren::len`]:
    /// ```
    /// # use rbook::ebook::errors::EbookResult;
    /// # use rbook::ebook::toc::{Toc, TocEntry, TocChildren};
    /// # use rbook::{Ebook, Epub};
    /// # fn main() -> EbookResult<()> {
    /// let epub = Epub::open("tests/ebooks/example_epub")?;
    /// let main_toc_root = epub.toc().contents().unwrap();
    ///
    /// assert_eq!(3, main_toc_root.children().len());
    /// // The `4` indicates that there is a single nested
    /// // child that's not an immediate child of the root.
    /// assert_eq!(4, main_toc_root.total_len());
    ///
    /// let child = main_toc_root.children().get(1).unwrap();
    ///
    /// assert_eq!(1, child.children().len());
    /// assert_eq!(1, child.total_len());
    /// # Ok(())
    /// # }
    /// ```
    fn total_len(&self) -> usize {
        self.children()
            .iter()
            .fold(0, |total, child| total + child.total_len() + 1)
    }
}

/// A collection of child [`entries`](TocEntry) retrieved from [`TocEntry::children`].
///
/// Provides two forms of iterators:
/// - [`TocChildren::iter`]: Immediate children (nested form).
/// - [`TocChildren::flatten`]: All children sorted in ascending [`order`](TocEntry::order).
pub trait TocChildren<'ebook> {
    /// Returns the associated immediate child [`TocEntry`] if the given `index` is less than
    /// [`Self::len`], otherwise [`None`].
    fn get(&self, index: usize) -> Option<impl TocEntry<'ebook> + 'ebook>;

    /// Returns an iterator over immediate child entries
    /// (whose [`depth`](TocEntry::depth) is one greater than the parent).
    ///
    /// # See Also
    /// - [`Self::flatten`] for ***all*** children, sorted by their [`order`](TocEntry::order).
    fn iter(&self) -> impl Iterator<Item = impl TocEntry<'ebook> + 'ebook> + 'ebook;

    /// Returns a recursive iterator over **all** children in ascending [`order`](TocEntry::order).
    fn flatten(&self) -> impl Iterator<Item = impl TocEntry<'ebook> + 'ebook> + 'ebook;

    /// The total number of immediate [`children`](Self::iter) a toc entry has.
    fn len(&self) -> usize;

    /// Returns `true` if there are no children.
    fn is_empty(&self) -> bool {
        self.len() == 0
    }
}

toc_entry_kind! {
    Acknowledgments => "acknowledgments",
    Afterword => "afterword",
    Appendix => "appendix",
    BackMatter => "backmatter",
    Bibliography => "bibliography",
    BodyMatter => "bodymatter",
    Chapter => "chapter",
    Colophon => "colophon",
    Conclusion => "conclusion",
    Contributors => "contributors",
    CopyrightPage => "copyright-page",
    Cover => "cover",
    Dedication => "dedication",
    Endnotes => "endnotes",
    Epigraph => "epigraph",
    Epilogue => "epilogue",
    Errata => "errata",
    Footnotes => "footnotes",
    Foreword => "foreword",
    FrontMatter => "frontmatter",
    Glossary => "glossary",
    Imprint => "imprint",
    Index => "index",
    Introduction => "introduction",
    Landmarks => "landmarks",
    PageList => "page-list",
    Part => "part",
    Preamble => "preamble",
    Preface => "preface",
    Prologue => "prologue",
    Qna => "qna",
    TitlePage => "titlepage",
    Toc => "toc",
    Volume => "volume",
}

mod macros {
    macro_rules! toc_entry_kind {
        {
            $($map_enum:ident => $map_string:literal,)*
        } => {
            /// The kinds of content that may be associated with table of content
            /// [`entries`](TocEntry).
            ///
            /// The variants are based on the EPUB 3 Structural Semantics Vocabulary.
            /// See more at: <https://www.w3.org/TR/epub-ssv-11>
            ///
            /// Uncommon semantics not directly included here are retrievable
            /// through [`TocEntryKind::Other`].
            #[non_exhaustive]
            #[derive(Clone, Debug, Default, Hash, PartialEq, Eq)]
            pub enum TocEntryKind<'ebook> {
                $(
                /// Maps to
                #[doc = concat!("`", $map_string, "`.")]
                ///
                /// More details at:
                #[doc = concat!("<https://www.w3.org/TR/epub-ssv-11/#", $map_string, ">.")]
                /// # Examples
                /// - Conversion from a string using [`TocEntryKind::from`]:
                /// ```
                #[doc = concat!(
                    " # use rbook::ebook::toc::TocEntryKind::{self, ",
                    stringify!($map_enum),
                    "};"
                )]
                #[doc = concat!(
                    "assert_eq!(TocEntryKind::",
                    stringify!($map_enum),
                    ", TocEntryKind::from(\"",
                    $map_string,
                    "\"))"
                )]
                /// ```
                $map_enum,
                )*
                /// An unknown entry kind.
                #[default]
                Unknown,
                /// An entry kind not mapped to any other variants.
                Other(Cow<'ebook, str>),
            }

            impl TocEntryKind<'_> {
                /// Returns the string form of a [`TocEntryKind`].
                ///
                /// # Examples
                /// - Conversion from a string and comparison:
                /// ```
                /// # use rbook::ebook::toc::TocEntryKind;
                /// let title_page_kind = TocEntryKind::from("titlepage");
                /// let chapter_kind = TocEntryKind::from("chapter");
                ///
                /// assert_eq!("titlepage", title_page_kind.as_str());
                /// assert_eq!("chapter", chapter_kind.as_str());
                /// ```
                #[must_use]
                pub fn as_str(&self) -> &str {
                    match self {
                        $(Self::$map_enum => $map_string,)*
                        Self::Unknown => "unknown",
                        Self::Other(value) => value.as_ref(),
                    }
                }
            }

            impl<'ebook, T: Into<Cow<'ebook, str>>> From<T> for TocEntryKind<'ebook> {
                fn from(value: T) -> Self {
                    let value = value.into();

                    match value.as_ref() {
                        $($map_string => Self::$map_enum,)*
                        "" => Self::Unknown,
                        _ => Self::Other(value)
                    }
                }
            }

            impl<'ebook> From<&'ebook Self> for TocEntryKind<'ebook> {
                fn from(value: &'ebook Self) -> Self {
                    match value {
                        $(Self::$map_enum => Self::$map_enum,)*
                        Self::Unknown => Self::Unknown,
                        Self::Other(cow) => Self::Other(Cow::Borrowed(cow.as_ref()))
                    }
                }
            }

            impl Display for TocEntryKind<'_> {
                fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
                    f.write_str(self.as_str())
                }
            }
        };
    }

    pub(super) use toc_entry_kind;
}
