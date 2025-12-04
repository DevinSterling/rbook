//! Error-related types for an [`Epub`](super::Epub).

/// Possible format errors from a [`Epub`](super::Epub).
///
/// Most of these errors are ignored when
/// [`EpubOpenOptions::strict`](super::EpubOpenOptions::strict)
/// is disabled.
///
/// # Variants
/// ## Container Errors (`META-INF/container.xml`)
/// Occurs within the `META-INF/container.xml` file:
/// - [`NoOpfReference`](EpubFormatError::NoOpfReference)
/// ## Package Errors (`.opf`)
/// Occurs within the package `.opf` file:
/// - [`UnknownVersion`](EpubFormatError::UnknownVersion)
/// - [`MissingMeta`](EpubFormatError::MissingMeta)
/// - [`CyclicMeta`](EpubFormatError::CyclicMeta)
/// - [`NoTocReference`](EpubFormatError::NoTocReference)
/// - [`NoPackageFound`](EpubFormatError::NoPackageFound)
/// - [`NoMetadataFound`](EpubFormatError::NoMetadataFound)
/// - [`NoManifestFound`](EpubFormatError::NoManifestFound)
/// - [`NoSpineFound`](EpubFormatError::NoSpineFound)
/// ## Toc Errors (`.ncx/.xhtml`)
/// Occurs within toc `.ncx` or `.xhtml` files:
/// - [`NoTocFound`](EpubFormatError::NoTocFound)
/// ## General Errors
/// Occurs in any file:
/// - [`MissingAttribute`](EpubFormatError::MissingAttribute)
/// - [`MissingValue`](EpubFormatError::MissingValue)
/// - [`InvalidHref`](EpubFormatError::InvalidHref)
#[non_exhaustive]
#[derive(thiserror::Error, Debug)]
pub enum EpubFormatError {
    ////////////////////////////////////////////////////////////////////////////////
    // General
    ////////////////////////////////////////////////////////////////////////////////
    /// A required attribute is missing from an element.
    #[error("Required attribute is missing: {0} <-- Missing")]
    MissingAttribute(String),

    /// The value of a specified element is missing.
    #[error("Required element value is missing: <{0}> <-- Missing value")]
    MissingValue(String),

    ////////////////////////////////////////////////////////////////////////////////
    // Within `META-INF/container.xml`
    ////////////////////////////////////////////////////////////////////////////////
    /// The container does not contain a reference
    /// pointing to an `.opf` file.
    ///
    /// Error Source: `META-INF/container.xml`
    #[error("Missing `<rootfile>` referencing an `.opf` file in `META-INF/container.xml`.")]
    NoOpfReference,

    /// An href pointing to a location is malformed.
    ///
    /// # Example
    /// Hrefs are expected be percent-encoded. Otherwise, it is malformed.
    /// - **Valid**:
    ///   `path/to/some%20cool%20file.xhtml`
    /// - **Malformed**:
    ///   `path/to/some cool file.xhtml`
    #[error("Invalid href: {0}")]
    InvalidHref(String),

    ////////////////////////////////////////////////////////////////////////////////
    // Within `package.opf`
    ////////////////////////////////////////////////////////////////////////////////
    /// Required EPUB version information is missing or invalid.
    ///
    /// Error Source: `.opf` file
    #[error("Missing or invalid epub version defined in the `.opf` file: {0}")]
    UnknownVersion(String),

    /// Required meta-information is missing
    ///
    /// For example, an epub requires at least *one* title.
    ///
    /// Error Source: `.opf` file
    #[error("Missing required meta in the `.opf` file: {0}")]
    MissingMeta(String),

    /// A cycle has been detected through a refinement `<meta>` chain.
    ///
    /// # Example
    /// ```xml
    /// <meta id="r1" refines="#r2" property="my-property-1">data1</meta>
    /// <meta id="r2" refines="#r1" property="my-property-2">data2</meta>
    /// ```
    ///
    /// Error Source: `.opf` file
    #[error("Cycle detected in refining meta in the `.opf` file: affected id={0}")]
    CyclicMeta(String),

    /// The manifest does not contain a reference
    /// pointing to a `table of contents` file.
    ///
    /// Error Source: `.opf` file
    #[error("Missing `<item>` referencing an `ncx` or `xhtml` toc in the `.opf` file.")]
    NoTocReference,

    /// The `package` element is not found.
    ///
    /// Error Source: `.opf` file
    #[error("Missing `<package>` in the `.opf` file.")]
    NoPackageFound,

    /// The `metadata` is not found.
    ///
    /// Compared to [`EpubFormatError::MissingMeta`], this indicates that
    /// the container of all metadata is unable to be found
    /// rather than the individually required meta-details.
    ///
    /// Error Source: `.opf` file
    #[error("Missing `<metadata>` in the `.opf` file.")]
    NoMetadataFound,

    /// The `Manifest` element is not found.
    ///
    /// Error Source: `.opf` file
    #[error("Missing `<manifest>` in the `.opf` file.")]
    NoManifestFound,

    /// The `spine` element is not found.
    ///
    /// Error Source: `.opf` file
    #[error("Missing `<spine>` in the `.opf` file.")]
    NoSpineFound,

    ////////////////////////////////////////////////////////////////////////////////
    // Within `toc.ncx/xhtml`
    ////////////////////////////////////////////////////////////////////////////////
    /// The `table of contents` file contains no `navMap` or `nav` element.
    ///
    /// Error Source: The table of contents (toc) `.ncx` or `.xhtml` file.
    #[error("Table of contents not found, it must be added within `toc.ncx/xhtml`.")]
    NoTocFound,
}
