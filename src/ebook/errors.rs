//! Error-related types for an [`Ebook`](super::Ebook).

pub use crate::ebook::archive::errors::ArchiveError;
pub use crate::ebook::archive::errors::ArchiveResult;
use crate::ebook::epub::errors::EpubError;
use crate::reader::errors::ReaderError;
use std::char::DecodeUtf16Error;
use std::error::Error;
use std::string::FromUtf8Error;
use thiserror::Error;

/// Alias for `Result<T, EbookError>`.
pub type EbookResult<T> = Result<T, EbookError>;

/// Unified error type.
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

    /// A [`Reader`](crate::reader::Reader) encountered an error on
    /// [`ReaderContent`](crate::reader::ReaderContent) retrieval.
    #[error(transparent)]
    Reader(ReaderError),

    /// An IO exception occurred during writing.
    #[cfg(feature = "write")]
    #[error(transparent)]
    Io(#[from] std::io::Error),
}

impl From<ReaderError> for EbookError {
    fn from(reader_error: ReaderError) -> Self {
        match reader_error {
            ReaderError::Archive(error) => EbookError::Archive(error),
            ReaderError::Format(error) => EbookError::Format(error),
            error => EbookError::Reader(error),
        }
    }
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

    /// Format errors specific to an [`Epub`](crate::Epub).
    #[error(transparent)]
    Epub(#[from] EpubError),
}

/// Specific error details regarding `UTF`.
#[derive(Error, Debug)]
pub enum UtfError {
    /// Uneven byte count of UTF-16 data.
    #[error("UTF-16 data needs to contain an even amount of bytes")]
    UnevenByteCount(usize),

    /// Invalid UTF-8 data.
    #[error(transparent)]
    InvalidUtf8(#[from] FromUtf8Error),

    /// Invalid UTF-16 data.
    #[error(transparent)]
    UndecodableUtf16(#[from] DecodeUtf16Error),
}
