//! Error-related types for an [`Ebook`](super::Ebook).

pub use crate::ebook::archive::errors::ArchiveError;
use crate::ebook::epub::errors::EpubFormatError;
use std::error::Error;
use std::string::FromUtf8Error;

/// Alias for `Result<T, EbookError>`.
pub type EbookResult<T> = Result<T, EbookError>;

/// Possible errors for an [`Ebook`](crate::Ebook).
#[non_exhaustive]
#[derive(thiserror::Error, Debug)]
pub enum EbookError {
    /// File access within an ebook archive has failed.
    #[error(transparent)]
    Archive(#[from] ArchiveError),

    /// Essential files are missing, such as the manifest or malformed file contents.
    #[error(transparent)]
    Format(#[from] FormatError),
}

/// Possible format errors for an [`Ebook`](crate::Ebook).
#[non_exhaustive]
#[derive(thiserror::Error, Debug)]
pub enum FormatError {
    /// Ebook file content unexpectedly causes an internal parser error.
    ///
    /// This may originate from malformed content within a file, such as improper XML.
    #[error(transparent)]
    Unparsable(#[from] Box<dyn Error + Send + Sync + 'static>),

    /// Ebook file contents do not conform to valid UTF-8.
    #[error(transparent)]
    InvalidUtf8(#[from] FromUtf8Error),

    /// Format errors specific to an [`Epub`](crate::Epub).
    #[error(transparent)]
    Epub(#[from] EpubFormatError),
}
