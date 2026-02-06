//! Error-related types for an [`Epub`](super::Epub).

/// Possible format errors from an [`Epub`](super::Epub).
///
/// # Variants
/// When parsing, error variants flagged with `*` are ignored when
/// [`EpubOpenOptions::strict`](super::EpubOpenOptions::strict)
/// is disabled.
///
/// ## Container Errors (`container.xml`)
/// Occurs within `/META-INF/container.xml`:
/// - [`NoOpfReference`](EpubError::NoOpfReference)
/// ## OPF (`.opf`) Package Errors
/// Occurs within the package `.opf` file:
/// ### Package Errors
/// - [`NoPackageFound`](EpubError::NoPackageFound)
/// - [`InvalidVersion`](EpubError::InvalidVersion)*
/// - [`InvalidPrefix`](EpubError::InvalidPrefix)*
/// - [`InvalidUniqueIdentifier`](EpubError::InvalidUniqueIdentifier)*
/// ### Metadata Errors
/// - [`NoMetadataFound`](EpubError::NoMetadataFound)*
/// - [`InvalidRefines`](EpubError::InvalidRefines)*
/// - [`MissingTitle`](EpubError::MissingTitle)*
/// - [`MissingLanguage`](EpubError::MissingLanguage)*
/// - [`CyclicMeta`](EpubError::CyclicMeta)
/// ### Manifest Errors
/// - [`NoManifestFound`](EpubError::NoManifestFound)*
/// - [`DuplicateItemId`](EpubError::DuplicateItemId)*
/// - [`NoXhtmlTocReference`](EpubError::NoXhtmlTocReference)*
/// ### Spine Errors
/// - [`NoSpineFound`](EpubError::NoSpineFound)*
/// - [`InvalidIdref`](EpubError::InvalidIdref)*
/// - [`InvalidNcxReference`](EpubError::InvalidNcxReference)*
/// - [`NoNcxReference`](EpubError::NoNcxReference)*
/// ## Toc Errors (`.ncx/.xhtml`)
/// Occurs within ToC `.ncx` or `.xhtml` files:
/// - [`NoTocFound`](EpubError::NoTocFound)*
/// ## General Errors
/// Occurs in any file:
/// - [`MissingAttribute`](EpubError::MissingAttribute)*
/// - [`MissingValue`](EpubError::MissingValue)*
/// - [`UnencodedHref`](EpubError::UnencodedHref)*
#[non_exhaustive]
#[derive(thiserror::Error, Debug)]
pub enum EpubError {
    ////////////////////////////////////////////////////////////////////////////////
    // General
    ////////////////////////////////////////////////////////////////////////////////
    /// A required attribute is missing from an element.
    #[error("Required attribute missing: {0}")]
    MissingAttribute(String),

    /// The value of a specified element is missing.
    #[error("Required text value is missing for element <{0}>")]
    MissingValue(String),

    /// An href pointing to a location is unencoded.
    ///
    /// # Example
    /// Hrefs are expected be percent-encoded. Otherwise, it is malformed.
    /// - **Valid**:
    ///   `path/to/some%20cool%20file.xhtml`
    /// - **Malformed**:
    ///   `path/to/some cool file.xhtml`
    #[error("Unencoded href (percent-encoding required): {0}")]
    UnencodedHref(String),

    ////////////////////////////////////////////////////////////////////////////////
    // Within `META-INF/container.xml`
    ////////////////////////////////////////////////////////////////////////////////
    /// The container does not contain a reference
    /// pointing to an `.opf` file.
    ///
    /// Error Source: `META-INF/container.xml`
    #[error("Missing `rootfile` element referencing an `.opf` file in `META-INF/container.xml`")]
    NoOpfReference,

    ////////////////////////////////////////////////////////////////////////////////
    // Package-specific errors
    ////////////////////////////////////////////////////////////////////////////////
    /// The `package` element is not found.
    ///
    /// Error Source: `.opf` file
    #[error("Missing `package` element")]
    NoPackageFound,

    /// The EPUB `version` field on the `package` element is invalid.
    ///
    /// Expected Range: `2 <= version < 4`
    ///
    /// Error Source: `.opf` file
    #[error("Invalid package epub `version`: {0}")]
    InvalidVersion(String),

    /// The `unique-identifier` field on the `package` element is invalid,
    /// referencing a non-existent `dc:identifier` entry by ID.
    ///
    /// Error Source: `.opf` file
    #[error(
        "Invalid package `unique-identifier` referencing a non-existent `dc:identifier` entry by ID: {0}"
    )]
    InvalidUniqueIdentifier(String),

    /// The `prefix` field on the `package` element is malformed.
    ///
    /// A well-formed value consists of space-separated `prefix: uri` pairs.
    ///
    /// See <https://www.w3.org/TR/epub/#sec-prefix-attr> for details.
    ///
    /// Error Source: `.opf` file
    #[error("Invalid package `prefix`: {0}")]
    InvalidPrefix(String),

    ////////////////////////////////////////////////////////////////////////////////
    // Metadata-specific errors
    ////////////////////////////////////////////////////////////////////////////////
    /// The `metadata` element is not found.
    ///
    /// This indicates that the parent container of all metadata is unable to be found
    /// rather than individual metadata entries.
    ///
    /// Error Source: `.opf` file
    #[error("Missing `metadata` element")]
    NoMetadataFound,

    /// The `refines` field of a refinement is invalid,
    /// referencing a non-existent metadata entry.
    ///
    /// Error Source: `.opf` file
    #[error("Invalid `refines` field referencing a non-existent id: {0}")]
    InvalidRefines(String),

    /// The `dc:title` metadata entry is missing.
    /// At least one is required.
    ///
    /// Error Source: `.opf` file
    #[error("Missing `dc:title` metadata entry")]
    MissingTitle,

    /// The required `dc:language` metadata entry is missing.
    /// At least one is required.
    ///
    /// Error Source: `.opf` file
    #[error("Missing `dc:language` metadata entry")]
    MissingLanguage,

    /// A cycle has been detected through a refinement `<meta>` chain.
    ///
    /// # Example
    /// - Each `<meta>` element depends on each other:
    /// ```xml
    /// <meta id="r1" refines="#r2" property="my-property-1">data1</meta>
    /// <meta id="r2" refines="#r1" property="my-property-2">data2</meta>
    /// ```
    ///
    /// Error Source: `.opf` file
    #[error("Cycle detected in metadata refinements; affected ID: {0}")]
    CyclicMeta(String),

    ////////////////////////////////////////////////////////////////////////////////
    // Manifest-specific errors
    ////////////////////////////////////////////////////////////////////////////////
    /// The `manifest` element is not found.
    ///
    /// Error Source: `.opf` file
    #[error("Missing `manifest` element")]
    NoManifestFound,

    /// An `item` element within the manifest contains a duplicate `id`.
    ///
    /// Each item in the manifest must have a unique ID.
    ///
    /// Error Source: `.opf` file
    #[error("Duplicate manifest `item` ID found: {0}")]
    DuplicateItemId(String),

    /// The manifest does not contain a reference pointing to a EPUB 3 ToC xhtml file.
    ///
    /// This occurs when no manifest entry contains the `nav` property.
    ///
    /// Error Source: `.opf` file
    #[error("Manifest missing an `item` with the 'nav' property (Required for EPUB 3)")]
    NoXhtmlTocReference,

    ////////////////////////////////////////////////////////////////////////////////
    // Spine-specific errors
    ////////////////////////////////////////////////////////////////////////////////
    /// The `spine` element is not found.
    ///
    /// Error Source: `.opf` file
    #[error("Missing `spine` element")]
    NoSpineFound,

    /// The `idref` field of a spine entry (`itemref`) points to
    /// a non-existent manifest entry (`item`).
    ///
    /// Error Source: `.opf` file
    #[error(
        "Invalid spine entry `idref` field that references a non-existent manifest entry by ID: {0}"
    )]
    InvalidIdref(String),

    /// The `toc` field on the spine `spine` element points to
    /// a non-existent manifest entry (`item`).
    ///
    /// Error Source: `.opf` file
    #[error("Invalid spine `toc` field that references a non-existent manifest entry by ID: {0}")]
    InvalidNcxReference(String),

    /// The `toc` field (referencing the NCX file by ID) on the `spine` is missing.
    ///
    /// Error Source: `.opf` file
    #[error("Missing spine `toc` field (Required for EPUB 2)")]
    NoNcxReference,

    ////////////////////////////////////////////////////////////////////////////////
    // Within `toc.ncx/xhtml`
    ////////////////////////////////////////////////////////////////////////////////
    /// The `table of contents` file contains no `navMap` or `nav` element.
    ///
    /// Error Source: The table of contents (ToC) `.ncx` or `.xhtml` file.
    #[error("No navigation structure (nav or navMap) found in the ToC resource")]
    NoTocFound,
}
