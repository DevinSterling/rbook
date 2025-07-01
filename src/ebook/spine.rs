//! Format-agnostic [`Spine`]-related content.

use crate::ebook::manifest::ManifestEntry;
use crate::ebook::resource::Resource;
use std::fmt::{Display, Formatter};

/// The "spine" of an [`Ebook`](super::Ebook) encompassing the canonical
/// reading order of textual resources intended for end-user reading.
///
/// # Examples
/// - Traversing the contents in canonical order:
/// ```
/// # use rbook::ebook::errors::EbookResult;
/// # use rbook::ebook::spine::Spine;
/// # use rbook::{Ebook, Epub};
/// # fn main() -> EbookResult<()> {
/// let epub = Epub::open("tests/ebooks/example_epub")?;
/// let mut entries = epub.spine().entries();
///
/// // Traversing to the cover page (START)
/// assert_eq!("cover", entries.next().unwrap().idref());
/// // Traversing to the table of contents
/// assert_eq!("toc", entries.next().unwrap().idref());
/// // Traversing to chapter 1
/// assert_eq!("c1", entries.next().unwrap().idref());
/// // Traversing to chapter 1a
/// assert_eq!("c1a", entries.next().unwrap().idref());
/// // Traversing to chapter 2
/// assert_eq!("c2", entries.next().unwrap().idref());
/// // End of readable contents (END)
/// assert_eq!(None, entries.next());
/// # Ok(())
/// # }
/// ```
pub trait Spine<'ebook> {
    /// The [`PageDirection`] hint, indicating how readable content flows.
    fn page_direction(&self) -> PageDirection;

    /// The total number of [`entries`](SpineEntry) that make up the spine.
    ///
    /// # Examples
    /// - Retrieving the count:
    /// ```
    /// # use rbook::ebook::errors::EbookResult;
    /// # use rbook::ebook::spine::Spine;
    /// # use rbook::{Ebook, Epub};
    /// # fn main() -> EbookResult<()> {
    /// let epub = Epub::open("tests/ebooks/example_epub")?;
    ///
    /// // This ebook has 4 readable entries.
    /// assert_eq!(5, epub.spine().len());
    /// // Invoking `len` is preferred as it
    /// // does not need to consume an iterator.
    /// assert_eq!(5, epub.spine().entries().count());
    /// # Ok(())
    /// # }
    /// ```
    fn len(&self) -> usize;

    /// Returns the associated [`SpineEntry`] if the provided `order` is less than
    /// [`Self::len`], otherwise [`None`].
    ///
    /// # Examples
    /// - Retrieving a spine entry by its order:
    /// ```
    /// # use rbook::ebook::errors::EbookResult;
    /// # use rbook::ebook::spine::{Spine, SpineEntry};
    /// # use rbook::{Ebook, Epub};
    /// # fn main() -> EbookResult<()> {
    /// let epub = Epub::open("tests/ebooks/example_epub")?;
    ///
    /// let spine_entry = epub.spine().by_order(2).unwrap();
    /// assert_eq!(2, spine_entry.order());
    /// assert_eq!("c1", spine_entry.idref());
    ///
    /// let spine_entry = epub.spine().by_order(4).unwrap();
    /// assert_eq!(4, spine_entry.order());
    /// assert_eq!("c2", spine_entry.idref());
    /// # Ok(())
    /// # }
    /// ```
    fn by_order(&self, order: usize) -> Option<impl SpineEntry<'ebook>>;

    /// Returns an iterator over all [`entries`](SpineEntry) within
    /// the spine in canonical order.
    ///
    /// See [`Spine::len`] to retrieve the total number of entries.
    fn entries(&self) -> impl Iterator<Item = impl SpineEntry<'ebook>> + 'ebook;

    /// Returns `true` if there are no [`entries`](SpineEntry), otherwise `false`.
    fn is_empty(&self) -> bool {
        self.len() == 0
    }
}

/// An entry contained within a [`Spine`], encompassing associated metadata.
pub trait SpineEntry<'ebook>: Ord {
    /// The canonical order of an entry (`0 = first entry`).
    fn order(&self) -> usize;

    /// The [`ManifestEntry`] associated with a [`SpineEntry`].
    ///
    /// Returns [`None`] if the spine entry references a non-existent
    /// [`ManifestEntry`] within the [`Manifest`](super::Manifest).
    fn manifest_entry(&self) -> Option<impl ManifestEntry<'ebook>>;

    /// The textual [`Resource`] intended for end-user reading an entry points to.
    fn resource(&self) -> Option<Resource<'ebook>> {
        self.manifest_entry().map(|entry| entry.resource())
    }
}

/// The default page direction preference for an [`ebook`](crate::Ebook).
///
/// This preference may or may not be honored if an application supports overriding
/// the default direction via **style configuration**, **user preferences**, etc.
///
/// The page direction does **not** affect the canonical
/// order of [`spine`](Spine) [`entries`](SpineEntry).
/// Instead, it is a hint indicating how the content flows, such as
/// [`left-to-right (ltr)`](PageDirection::LeftToRight),
/// [`right-to-left (rtl)`](PageDirection::RightToLeft),
/// and [`no preference (default)`](PageDirection::Default).
///
/// [`PageDirection::as_str`] can be used to retrieve the string form.
///
/// Default: [`PageDirection::Default`]
#[non_exhaustive]
#[derive(Debug, Default, Copy, Clone, PartialEq, Eq, Hash)]
pub enum PageDirection {
    /// Pages flow from left-to-right (`ltr`).
    LeftToRight,
    /// Pages flow from right-to-left (`rtl`).
    RightToLeft,
    /// No specified page direction preference (`default`).
    #[default]
    Default,
}

impl PageDirection {
    const DEFAULT: &'static str = "default";
    const LEFT_TO_RIGHT: &'static str = "ltr";
    const RIGHT_TO_LEFT: &'static str = "rtl";
    const LEFT_TO_RIGHT_BYTES: &'static [u8] = Self::LEFT_TO_RIGHT.as_bytes();
    const RIGHT_TO_LEFT_BYTES: &'static [u8] = Self::RIGHT_TO_LEFT.as_bytes();

    pub(crate) fn from_bytes(bytes: &[u8]) -> Self {
        match bytes {
            Self::LEFT_TO_RIGHT_BYTES => Self::LeftToRight,
            Self::RIGHT_TO_LEFT_BYTES => Self::RightToLeft,
            _ => Self::Default,
        }
    }

    /// Returns the string representation of a [`PageDirection`] preference.
    ///
    /// # Examples
    /// - Observing the string representations:
    /// ```
    /// # use rbook::ebook::spine::PageDirection;
    /// assert_eq!("ltr", PageDirection::LeftToRight.as_str());
    /// assert_eq!("rtl", PageDirection::RightToLeft.as_str());
    /// assert_eq!("default", PageDirection::Default.as_str());
    /// ```
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::LeftToRight => Self::LEFT_TO_RIGHT,
            Self::RightToLeft => Self::RIGHT_TO_LEFT,
            Self::Default => Self::DEFAULT,
        }
    }
}

impl Display for PageDirection {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        f.write_str(self.as_str())
    }
}
