//! Error-related types for a [`Reader`](super::Reader).

use crate::ebook::errors::{EbookError, FormatError};
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
/// Not directly caused by input; indicates a deeper problem in an ebook’s contents.
/// - [`MalformedEbook`](ReaderError::MalformedEbook)
/// - [`InvalidEbookContent`](ReaderError::InvalidEbookContent)
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

    /// Content retrieval failed due to a malformed ebook
    /// (e.g., malformed or missing required data).
    #[error(transparent)]
    MalformedEbook(#[from] FormatError),

    /// An unexpected error propagated from the underlying [`Ebook`](crate::Ebook)
    /// of a [`Reader`](super::Reader).
    #[error(transparent)]
    InvalidEbookContent(#[from] EbookError),
}
