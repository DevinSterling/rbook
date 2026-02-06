//! Error-related types for a [`Reader`](super::Reader).

use crate::ebook::errors::{ArchiveError, FormatError};
use thiserror::Error;

/// Alias for `Result<T, ReaderError>`.
pub type ReaderResult<T> = Result<T, ReaderError>;

/// Possible errors from a [`Reader`](super::Reader).
///
/// # Variants
/// ## Input Errors
/// Indicates the caller provided invalid arguments which can be corrected.
/// - [`OutOfBounds`](ReaderError::OutOfBounds)
/// - [`NoMapping`](ReaderError::NoMapping)
/// ## Output Errors
/// Not directly caused by input, indicating a malformed ebook.
/// - [`Archive`](ReaderError::Archive)
/// - [`Format`](ReaderError::Format)
#[non_exhaustive]
#[derive(Error, Debug)]
pub enum ReaderError {
    /// A [`ReaderKey::Position`](super::ReaderKey::Position) is beyond the allocated
    /// length of a reader ([`Reader::len`](super::Reader::len)); out-of-bounds.
    #[error("[OutOfBounds Error]: The position `{position}` must be less than the length `{len}`")]
    OutOfBounds {
        /// The requested out-of-bounds position.
        position: usize,
        /// The maximum length that `position` must be less than.
        len: usize,
    },

    /// A [`ReaderKey::Value`](super::ReaderKey::Value) has no corresponding mapping
    /// within a [`Reader`](super::Reader).
    #[error("[NoMapping Error]: The provided `{0}` has no corresponding mapping")]
    NoMapping(
        /// The value that has no associated mapping.
        String,
    ),

    /// Retrieval of reader content within an ebook archive has failed.
    ///
    /// When converting a [`ReaderError`] into [`EbookError`](crate::ebook::errors::EbookError),
    /// implicitly using the try-operator (`?`) or explicitly using
    /// [`EbookError::from`](crate::ebook::errors::EbookError::from),
    /// this variant is retrievable from
    /// [`EbookError::Archive`](crate::ebook::errors::EbookError::Archive) ***instead***.
    #[error(transparent)]
    Archive(#[from] ArchiveError),

    /// Malformed file contents, such as essential fields missing.
    ///
    /// When converting a [`ReaderError`] into [`EbookError`](crate::ebook::errors::EbookError),
    /// implicitly using the try-operator (`?`) or explicitly using
    /// [`EbookError::from`](crate::ebook::errors::EbookError::from),
    /// this variant is retrievable from
    /// [`EbookError::Format`](crate::ebook::errors::EbookError::Format) ***instead***.
    #[error(transparent)]
    Format(#[from] FormatError),
}
