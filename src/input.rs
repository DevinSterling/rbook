//! Flexible input traits for [`Ebook`](super::Ebook) operations.
//!
//! This module primarily exists to reduce boilerplate and API surface area, using:
//! - [`Many`]: Flexibly pass one-to-many arguments (single items, arrays, or vectors) to methods.
//! - [`IntoOption`]: Flexibly pass an [`Option`] or the underlying type directly.

/// A helper trait for flexible one-to-many arguments.
///
/// This trait abstracts over single items and collections (e.g., arrays, vectors),
/// allowing methods to accept both uniformly.
///
/// # See Also
/// - [`Batch`] to pass iterators as arguments to methods that accept `impl Many<T>`.
///
/// # Examples
/// - Passing arguments to a method which accepts the `Many` trait:
/// ```
/// # #[cfg(feature = "write")]
/// # {
/// # use rbook::Epub;
/// use rbook::epub::spine::DetachedEpubSpineEntry;
///
/// # let mut epub = Epub::new();
/// let mut spine = epub.spine_mut();
///
/// // Pass a single item
/// spine.push(DetachedEpubSpineEntry::new("prologue"));
///
/// // Pass an array (Many)
/// spine.push([
///     DetachedEpubSpineEntry::new("c1"),
///     DetachedEpubSpineEntry::new("c2"),
/// ]);
///
/// // Pass a vec (Many)
/// spine.push(vec![
///     DetachedEpubSpineEntry::new("c3"),
///     DetachedEpubSpineEntry::new("c4"),
/// ]);
///
/// assert_eq!(5, epub.spine().len());
/// # }
/// ```
/// - `Many` supports types that implement [`Into`]:
/// ```
/// # #[cfg(feature = "write")]
/// # {
/// # use rbook::ebook::epub::spine::DetachedEpubSpineEntry;
/// # let mut epub = rbook::Epub::new();
/// # let mut spine = epub.spine_mut();
/// struct Itemref<'a>(&'a str, bool);
///
/// // Map custom type
/// impl<'a> Into<DetachedEpubSpineEntry> for Itemref<'a> {
///     fn into(self) -> DetachedEpubSpineEntry {
///         DetachedEpubSpineEntry::new(self.0).linear(self.1)
///     }
/// }
///
/// // Pass a single item
/// spine.push(Itemref("prologue", true));
///
/// // Pass an array (Many)
/// spine.push([
///     Itemref("c1", true),
///     Itemref("c2", true),
///     Itemref("supplementary", false),
///     Itemref("c3", true),
/// ]);
/// # }
/// ```
pub trait Many<T> {
    /// The iterator produced.
    type Iter: Iterator<Item = T>;

    /// Returns an iterator over one-to-many items.
    fn iter_many(self) -> Self::Iter;
}

impl<T, I: Into<T>, const N: usize> Many<T> for [I; N] {
    type Iter = std::iter::Map<std::array::IntoIter<I, N>, fn(I) -> T>;

    fn iter_many(self) -> Self::Iter {
        self.into_iter().map(Into::into)
    }
}

impl<T, I: Into<T>> Many<T> for Vec<I> {
    type Iter = std::iter::Map<std::vec::IntoIter<I>, fn(I) -> T>;

    fn iter_many(self) -> Self::Iter {
        self.into_iter().map(Into::into)
    }
}

macro_rules! impl_single_insertable {
    ($(
        impl$(<$($life:lifetime),+>)? $target:path,
    )+) => {
        $(
        impl<$($($life,)+)? I: Into<$target>> Many<$target> for I {
            type Iter = std::iter::Once<$target>;

            fn iter_many(self) -> Self::Iter {
                std::iter::once(self.into())
            }
        }

        impl$(<$($life),+>)? Many<$target> for Option<$target> {
            type Iter = std::option::IntoIter<$target>;

            fn iter_many(self) -> Self::Iter {
                self.into_iter()
            }
        }
        )+
    };
}

impl_single_insertable! {
    impl crate::epub::metadata::EpubVersion,
    impl crate::ebook::element::Attribute,
    impl<'a> crate::ebook::resource::Resource<'a>,
    impl<'a> crate::ebook::resource::ResourceKind<'a>,
}

#[cfg(feature = "write")]
impl_single_insertable! {
    impl crate::epub::manifest::DetachedEpubManifestEntry,
    impl crate::epub::spine::DetachedEpubSpineEntry,
    impl crate::epub::toc::DetachedEpubTocEntry,
    impl crate::epub::EpubChapter,
}

/// Convenience adapter for passing an [`Iterator`] to methods
/// that accept [`impl Many<T>`](Many).
///
/// This adapter exists as methods that accept `impl Many<T>`
/// do not accept arbitrary implementations of [`IntoIterator`].
///
/// For example, this will fail to compile without the adapter:
/// ```compile_fail
/// # use rbook::Epub;
/// // Creating a reversed iterator
/// let creators = ["a", "b", "c"].into_iter().rev();
///
/// // Fails to compile...
/// Epub::create().creator(creators);
/// ```
/// However, if the iterator is passed through the [`Batch`] adapter,
/// then the code compiles:
/// ```
/// # #[cfg(feature = "write")]
/// # {
/// # use rbook::Epub;
/// use rbook::input::Batch;
///
/// let creators = ["a", "b", "c"].into_iter().rev();
///
/// // Compilation successful!
/// Epub::builder()
///     .creator(Batch(creators))
///     // If Batch is not needed, there's built-in support for vectors and arrays
///     .creator(vec!["x", "y", "z"])
///     .creator(["d", "e", "f"]);
/// # }
/// ```
///
/// # Specialization
/// Once [specialization](https://github.com/rust-lang/rust/issues/31844) is available,
/// this adapter may no longer be necessary.
#[derive(Clone, Debug, PartialEq)]
pub struct Batch<Iter>(pub Iter);

impl<T, I: Into<T>, It: IntoIterator<Item = I>> Many<T> for Batch<It> {
    type Iter = std::iter::Map<It::IntoIter, fn(I) -> T>;

    fn iter_many(self) -> Self::Iter {
        self.0.into_iter().map(Into::into)
    }
}

/// A helper trait for flexible [`Option`] arguments,
/// where standard Rust [`Into<Option<T>>`] cannot convert
/// values like `T` directly into `Option<T>` (e.g., `&str` into `Option<String>`).
///
/// This trait abstracts over **values** and **options** (e.g., `&str`, `String`, `Option<String>`),
/// allowing methods to accept both uniformly to reduce boilerplate conversions.
///
/// # Examples
/// Passing arguments to a method, `fn set_id(id: impl IntoOption<String>)`:
/// ```
/// # use rbook::input::IntoOption;
/// # struct Entry(Option<String>);
/// # impl Entry {
/// #     fn set_id(&mut self, id: impl IntoOption<String>) {
/// #         self.0 = id.into_option();
/// #     }
/// # }
/// # let mut entry = Entry(None);
/// entry.set_id("my-id");                  // &str
/// entry.set_id(String::from("my-id"));    // Owned String
/// entry.set_id(None);                     // None
/// entry.set_id(Some("my-id".to_owned())); // Explicit Option<String>
/// ```
#[cfg(feature = "write")]
pub trait IntoOption<T> {
    /// Consumes self and returns an [`Option`].
    fn into_option(self) -> Option<T>;
}

#[cfg(feature = "write")]
mod write {
    use crate::input::IntoOption;

    impl<T> IntoOption<T> for Option<T> {
        fn into_option(self) -> Option<T> {
            self
        }
    }

    impl IntoOption<String> for &str {
        fn into_option(self) -> Option<String> {
            Some(self.to_owned())
        }
    }

    impl IntoOption<String> for &String {
        fn into_option(self) -> Option<String> {
            Some(self.to_owned())
        }
    }

    impl IntoOption<String> for String {
        fn into_option(self) -> Option<String> {
            Some(self)
        }
    }

    impl IntoOption<String> for std::borrow::Cow<'_, str> {
        fn into_option(self) -> Option<String> {
            Some(self.into_owned())
        }
    }

    impl IntoOption<String> for crate::ebook::toc::TocEntryKind<'_> {
        fn into_option(self) -> Option<String> {
            Some(self.as_str().to_owned())
        }
    }
}
