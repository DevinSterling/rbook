pub mod epub;
// pub mod mobi;
// pub mod cbz;
pub mod xml;

use std::io::{Read, Seek};
use std::path::Path;
use thiserror::Error;

use crate::archive::ArchiveError;

/// Result type with [EbookError] as the error.
pub type EbookResult<T> = Result<T, EbookError>;

/// Trait that represents an ebook object supported by rbook.
///
/// Provides an associated function, `new(path: &str)` that returns
/// the result of an ebook object with its associated contents.
///
/// # Supported Formats
/// Current supported formats are:
/// - [epub](epub::Epub)
pub trait Ebook {
    type Format;

    /// Creates a new ebook object with its associated content.
    ///
    /// The function accepts a string that leads to a directory or file.
    ///
    /// # Errors
    /// If the given path does not support the ebook format, an
    /// [EbookError] from a result will be returned.
    ///
    /// # Examples
    /// Basic usage:
    /// ```
    /// use rbook::Ebook;
    ///
    /// // Providing a file in epub format
    /// let epub = rbook::Epub::new("tests/ebooks/childrens-literature.epub").unwrap();
    ///
    /// // View contents
    /// println!("{epub:?}");
    /// ```
    fn new<P: AsRef<Path>>(path: P) -> EbookResult<Self::Format>;

    /// Creates a new ebook object with its associated content using
    /// a reader instance.
    ///
    /// # Errors
    /// If the given instance does not support the ebook format, an
    /// [EbookError] from a result will be returned.
    ///
    /// # Examples
    /// Basic usage:
    /// ```
    /// use rbook::Ebook;
    ///
    /// let file = std::fs::File::open("tests/ebooks/childrens-literature.epub").unwrap();
    /// let epub = rbook::Epub::read_from(file);
    ///
    /// // View contents
    /// println!("{epub:?}");
    /// ```
    fn read_from<
        #[cfg(feature = "multi-thread")] R: Seek + Read + Send + Sync + 'static,
        #[cfg(not(feature = "multi-thread"))] R: Seek + Read + 'static,
    >(
        reader: R,
    ) -> EbookResult<Self::Format>;
}

/// Possible errors for [Ebook]
/// - [IO](Self::IO)
/// - [Parse](Self::Parse)
/// - [Archive](Self::Archive)
#[derive(Error, Debug)]
pub enum EbookError {
    /// When a given ebook path is not valid.
    #[error("[IO Error][{cause}]: {description}")]
    IO { cause: String, description: String },
    /// When parsing, essential files are missing, e.g., the
    /// manifest. In addition, malformed file contents can
    /// cause a parse error.
    #[error("[Parse Error][{cause}]: {description}")]
    Parse { cause: String, description: String },
    /// When access to files in an ebook archive fails.
    #[error("[Archive Error]{0}")]
    Archive(ArchiveError),
}
