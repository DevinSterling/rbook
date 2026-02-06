use crate::ebook::errors::UtfError;
use crate::ebook::resource::Resource;
use std::io;
use std::path::PathBuf;

/// Alias for `Result<T, ArchiveError>`.
pub type ArchiveResult<T> = Result<T, ArchiveError>;

/// Possible errors from an archive contained within an [`Ebook`](crate::Ebook).
#[non_exhaustive]
#[derive(thiserror::Error, Debug)]
pub enum ArchiveError {
    /// The content exists (such as a resource contained within an archive),
    /// although is unable to be read due to invalid utf8.
    ///
    /// This can occur when requesting to read a resource by string.
    #[error("[InvalidUtf8Resource - `{resource}`]: Resource value cannot be read as UTF-8")]
    InvalidUtf8Resource {
        /// The root cause of the error.
        source: UtfError,
        /// The [`Resource`] responsible for triggering the error.
        resource: Resource<'static>,
    },

    /// A given resource does not point to a valid location.
    #[error("[InvalidResource - `{resource}`]: {source}")]
    InvalidResource {
        /// The root cause of the error.
        source: io::Error,
        /// The [`Resource`] responsible for triggering the error.
        resource: Resource<'static>,
    },

    /// The content exists (such as a resource contained within an archive),
    /// although is unable to be read, typically I/O.
    #[error("[CannotRead - `{resource:?}`]: {source}")]
    CannotRead {
        /// The root cause of the error.
        source: io::Error,
        /// The [`Resource`] responsible for triggering the error.
        resource: Resource<'static>,
    },

    /// The archive itself is unreadable due to not existing,
    /// unsupported format, or malformed state.
    ///
    /// This error is *generally* thrown **before** an archive is instantiated.
    ///
    /// Path *is* [`None`] when an improper reader `R: Read + Seek`,
    /// is supplied during ebook instantiation from a method such as
    /// [`EpubOpenOptions::read`](crate::epub::EpubOpenOptions::read).
    #[error("[UnreadableArchive - `{path:?}`]: {source}")]
    UnreadableArchive {
        /// The root cause of this error.
        source: io::Error,
        /// The path responsible for triggering the error, if applicable.
        path: Option<PathBuf>,
    },
}
