//! Format-agnostic table-of-contents; [`Toc`]-related content.
//!
//! # See Also
//! - [`epub::toc`][crate::epub::toc] for the epub-specific ToC module.

use crate::ebook::manifest::ManifestEntry;
use crate::ebook::resource::Resource;
use crate::ebook::toc::macros::toc_entry_kind;
use crate::util::Sealed;
use std::fmt::Display;

/// The table of contents, aiding navigation throughout an ebook [`Ebook`](super::Ebook).
///
/// Each [`TocEntry`] returned by [`Toc`] is a top-level root containing
/// [children](TocEntry::iter).
///
/// The methods [`Self::by_kind`] and [`Self::iter`] can be used to retrieve TOC variants,
/// such as [`landmarks`](TocEntryKind::Landmarks), [`page-list`](TocEntryKind::PageList), etc.
///
/// # See Also
/// - [`EpubToc`](crate::epub::toc::EpubToc) for epub-specific table of contents information.
///
/// # Examples
/// - Iterating over the table of contents:
/// ```
/// # use rbook::Epub;
/// # fn main() -> rbook::ebook::errors::EbookResult<()> {
/// let epub = Epub::open("tests/ebooks/example_epub")?;
/// let root = epub.toc().contents().unwrap();
/// let mut children = root.iter();
///
/// // A for loop may also be used alternatively
/// assert_eq!("The Cover", children.next().unwrap().label());
/// assert_eq!("rbook Chapter 1", children.next().unwrap().label());
/// assert_eq!("rbook Chapter 2", children.next().unwrap().label());
/// assert_eq!(None, children.next());
/// # Ok(())
/// # }
/// ```
pub trait Toc<'ebook>: Sealed {
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
    /// # use rbook::ebook::toc::TocEntryKind;
    /// # use rbook::Epub;
    /// # fn main() -> rbook::ebook::errors::EbookResult<()> {
    /// let epub = Epub::open("tests/ebooks/example_epub")?;
    /// let toc = epub.toc();
    ///
    /// // Providing a string as input:
    /// let contents = toc.by_kind("toc");
    /// let pagelist = toc.by_kind("page-list");
    /// // Providing an enum as input:
    /// let landmarks = toc.by_kind(TocEntryKind::Landmarks);
    ///
    /// assert_eq!(contents, toc.by_kind(TocEntryKind::Toc));
    /// assert_eq!(pagelist, toc.by_kind(TocEntryKind::PageList));
    /// assert_eq!(landmarks, toc.by_kind("landmarks"));
    /// assert_eq!(None, toc.by_kind(TocEntryKind::ListOfIllustrations));
    /// # Ok(())
    /// # }
    /// ```
    fn by_kind(
        &self,
        kind: impl Into<TocEntryKind<'ebook>>,
    ) -> Option<impl TocEntry<'ebook> + 'ebook>;

    /// Returns an iterator over all **root** [entries](TocEntry).
    ///
    /// # See Also
    /// - [`TocEntry::kind`] to retrieve the [`TocEntryKind`] of each root.
    ///
    /// # Examples
    /// - Iterating over roots and observing their kind:
    /// ```
    /// # use rbook::ebook::toc::TocEntryKind;
    /// # use rbook::Epub;
    /// # fn main() -> rbook::ebook::errors::EbookResult<()> {
    /// let epub = Epub::open("tests/ebooks/example_epub")?;
    /// let mut roots = epub.toc().iter();
    ///
    /// let contents = roots.next().unwrap();
    /// assert_eq!(TocEntryKind::Toc, contents.kind());
    ///
    /// let landmarks = roots.next().unwrap();
    /// assert_eq!(TocEntryKind::Landmarks, landmarks.kind());
    ///
    /// let pagelist = roots.next().unwrap();
    /// assert_eq!(TocEntryKind::PageList, pagelist.kind());
    ///
    /// // No remaining roots
    /// assert_eq!(None, roots.next());
    /// # Ok(())
    /// # }
    /// ```
    fn iter(&self) -> impl Iterator<Item = impl TocEntry<'ebook>> + 'ebook;
}

/// An entry contained within a [`Toc`], encompassing associated metadata.
///
/// Provides two forms of iterators:
/// - [`TocEntry::iter`]: Direct children (nested form).
/// - [`TocEntry::flatten`]: **All** children recursively.
///
/// # See Also
/// - [`EpubTocEntry`](crate::epub::toc::EpubTocEntry) for epub-specific entry information.
pub trait TocEntry<'ebook>: Sealed {
    /// The depth of an entry relative to the root ([`0 = root`](Self::is_root)).
    fn depth(&self) -> usize;

    /// The human-readable label.
    ///
    /// The label is the text displayed to the user in a reading system's navigation menu.
    fn label(&self) -> &'ebook str;

    /// The semantic kind of content associated with an entry.
    ///
    /// For example, an entry may point to the
    /// [`appendix`](TocEntryKind::Appendix) or [`cover page`](TocEntryKind::Cover).
    fn kind(&self) -> TocEntryKind<'ebook>;

    /// The [`ManifestEntry`] associated with a [`TocEntry`].
    ///
    /// Returns [`None`] if the toc entry references a non-existent
    /// [`ManifestEntry`] within the [`Manifest`](super::Manifest).
    fn manifest_entry(&self) -> Option<impl ManifestEntry<'ebook> + 'ebook>;

    /// The [`Resource`] intended to navigate to from an entry.
    ///
    /// # Examples
    /// - Retrieving the resource associated with an entry:
    /// ```
    /// # use rbook::Epub;
    /// # fn main() -> rbook::ebook::errors::EbookResult<()> {
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

    /// Returns the associated direct child [`TocEntry`] if the given `index` is less than
    /// [`Self::len`], otherwise [`None`].
    fn get(&self, index: usize) -> Option<impl TocEntry<'ebook> + 'ebook>;

    /// Returns an iterator over direct child entries
    /// (whose [`depth`](TocEntry::depth) is one greater than the parent).
    ///
    /// # See Also
    /// - [`Self::flatten`] for ***all*** children recursively.
    fn iter(&self) -> impl Iterator<Item = impl TocEntry<'ebook> + 'ebook> + 'ebook;

    /// Returns a recursive iterator over **all** children.
    fn flatten(&self) -> impl Iterator<Item = impl TocEntry<'ebook> + 'ebook> + 'ebook;

    /// The total number of direct [`children`](Self::iter) a toc entry has.
    fn len(&self) -> usize;

    /// Returns `true` if there are no children.
    fn is_empty(&self) -> bool {
        self.len() == 0
    }

    /// Returns `true` if the depth of a toc entry is `0`, indicating the root.
    ///
    /// # Examples
    /// - Assessing if an entry is a root:
    /// ```
    /// # use rbook::Epub;
    /// # fn main() -> rbook::ebook::errors::EbookResult<()> {
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
    /// Child [entries](TocEntry) have a maximum depth less than the parent.
    /// For example, if an entry has a maximum depth of `5`,
    /// then its direct children will have a maximum depth of **at most** `4`.
    ///
    /// # Scenarios
    /// The maximum depth indicates the following:
    ///
    /// | Max Depth | Indication                                                      |
    /// |-----------|-----------------------------------------------------------------|
    /// | 0         | No direct children (Equivalent to [`TocEntry::is_empty`]).      |
    /// | 1         | Only direct children (Children do not contain nested children). |
    /// | \>1       | At least one direct child contains nested children.             |
    ///
    /// # See Also
    /// - [`Self::depth`] for the pre-computed depth relative to the root.
    ///
    /// # Examples
    /// - Comparing the calculated maximum depth with [`Self::depth`]:
    /// ```
    /// # use rbook::Epub;
    /// # fn main() -> rbook::ebook::errors::EbookResult<()> {
    /// let epub = Epub::open("tests/ebooks/example_epub")?;
    /// let main_toc_root = epub.toc().contents().unwrap();
    ///
    /// // Current depth relative to the root
    /// assert_eq!(0, main_toc_root.depth());
    /// // Calculated maximum depth - deepest child entry within the hierarchy
    /// assert_eq!(2, main_toc_root.max_depth());
    ///
    /// let child = main_toc_root.get(0).unwrap();
    ///
    /// // Current depth relative to the root
    /// assert_eq!(1, child.depth());
    /// // Calculated maximum depth - `child` entry has no children
    /// assert_eq!(0, child.max_depth());
    /// # Ok(())
    /// # }
    /// ```
    fn max_depth(&self) -> usize {
        self.iter()
            .fold(0, |depth, child| depth.max(1 + child.max_depth()))
    }

    /// Calculates and returns the **total** number of all (direct and nested)
    /// children relative to an entry.
    ///
    /// # Scenarios
    /// The total number of children indicates the following:
    ///
    /// | Total Children  | Indication                                                      |
    /// |-----------------|-----------------------------------------------------------------|
    /// | 0               | No direct children (Equivalent to [`Self::is_empty`]).          |
    /// | [`Self::len`]   | Only direct children (Children do not contain nested children). |
    /// | \>[`Self::len`] | At least one direct child contains nested children.             |
    ///
    /// # Examples
    /// - Comparing the calculated total length with [`Self::len`]:
    /// ```
    /// # use rbook::Epub;
    /// # fn main() -> rbook::ebook::errors::EbookResult<()> {
    /// let epub = Epub::open("tests/ebooks/example_epub")?;
    /// let main_toc_root = epub.toc().contents().unwrap();
    ///
    /// assert_eq!(3, main_toc_root.len());
    /// // The `4` indicates that there is a single nested
    /// // child that's not a direct child of the root.
    /// assert_eq!(4, main_toc_root.total_len());
    ///
    /// let child = main_toc_root.get(1).unwrap();
    ///
    /// assert_eq!(1, child.len());
    /// assert_eq!(1, child.total_len());
    /// # Ok(())
    /// # }
    /// ```
    fn total_len(&self) -> usize {
        self.iter()
            .fold(0, |total, child| total + child.total_len() + 1)
    }
}

toc_entry_kind! {
    Acknowledgments => "acknowledgments",
    Afterword => "afterword",
    Appendix => "appendix",
    BackMatter => "backmatter",
    Bibliography => "bibliography",
    // https://idpf.org/epub/20/spec/OPF_2.0_final_spec.html#Section2.6
    // specifies "text" as **First "real" page of content (e.g. "Chapter 1")**.
    BodyMatter => "bodymatter" | "text",
    Chapter => "chapter",
    Colophon => "colophon",
    Conclusion => "conclusion",
    Contributors => "contributors",
    CopyrightPage => "copyright-page" | "copyright",
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
    ListOfIllustrations => "loi",
    ListOfAudio => "loa",
    ListOfTables => "lot",
    ListOfVideos => "lov",
    PageList => "page-list",
    Part => "part",
    Preamble => "preamble",
    Preface => "preface",
    Prologue => "prologue",
    Qna => "qna",
    TitlePage => "titlepage" | "title-page",
    Toc => "toc",
    Volume => "volume",
}

mod macros {
    macro_rules! toc_entry_kind {
        {
            $($map_enum:ident => $map_string:literal $(| $additional_mapping:literal)*,)*
        } => {
            /// The kinds of content that may be associated with table of content
            /// [entries](TocEntry).
            ///
            /// The variants are based on the EPUB 3 Structural Semantics Vocabulary.
            /// See more at: <https://www.w3.org/TR/epub-ssv-11>
            ///
            /// Uncommon semantics not directly included here are retrievable
            /// through [`TocEntryKind::Other`].
            #[non_exhaustive]
            #[derive(Copy, Clone, Debug, Default, Hash, PartialEq, Eq)]
            pub enum TocEntryKind<'ebook> {
                $(
                #[doc = concat!("Maps to `", $map_string, "`.")]
                $(
                #[doc = concat!("- `", $additional_mapping, "` → `", $map_string, "`")]
                )*
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
                Other(&'ebook str),
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
                pub fn as_str(&self) -> &str {
                    match self {
                        $(Self::$map_enum => $map_string,)*
                        Self::Unknown => "unknown",
                        Self::Other(value) => value,
                    }
                }
            }

            impl<'ebook, S: AsRef<str> + ?Sized> From<&'ebook S> for TocEntryKind<'ebook> {
                fn from(value: &'ebook S) -> Self {
                    let value = value.as_ref();

                    match value {
                        $($map_string $(| $additional_mapping)* => Self::$map_enum,)*
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
                        Self::Other(other) => Self::Other(other)
                    }
                }
            }

            impl Display for TocEntryKind<'_> {
                fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
                    f.write_str(self.as_str())
                }
            }
        };
    }

    pub(super) use toc_entry_kind;
}
