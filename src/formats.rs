pub mod epub;
// pub mod mobi;
// pub mod cbz;
pub mod xml;

use thiserror::Error;
use std::io::{Read, Seek};
use std::path::Path;

use crate::archive::ArchiveError;

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
    /// [EbookError](EbookError) from a result will be returned.
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
    fn new<P: AsRef<Path>>(path: P) -> Result<Self::Format, EbookError>;

    /// Creates a new ebook object with its associated content using
    /// a reader instance.
    ///
    /// # Errors
    /// If the given instance does not support the ebook format, an
    /// [EbookError](EbookError) from a result will be returned.
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
    fn read_from<R: Seek + Read + 'static>(reader: R) -> Result<Self::Format, EbookError>;
}

/// Possible errors for [Ebook](Ebook)
/// - **IO**: When a given ebook path is not valid
/// - **Parse**: When parsing, essential files are missing, e.g., the
/// manifest. In addition, malformed file contents can cause a parse error.
/// - **Archive**: When access to files in an ebook archive fails
#[derive(Error, Debug)]
pub enum EbookError {
    #[error("[IO Error][{cause}]: {description}")]
    IO { cause: String, description: String },
    #[error("[Parse Error][{cause}]: {description}")]
    Parse { cause: String, description: String },
    #[error("[Archive Error]{0}")]
    Archive(ArchiveError)
}