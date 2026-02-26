//! The Electronic Publication ([`Epub`]) module.
//!
//! `rbook` supports EPUB versions `2` and `3`.
//!
//! For more information regarding the EPUB spec, see:
//! <https://www.w3.org/TR/epub>
//!
//! # Reading
//! Opening an [`Epub`] provides access to its core components:
//! - [`EpubPackage`]: Package details (epub version, package location/directory, prefixes)
//! - [`EpubMetadata`]: Metadata details (title, language, identifiers, version)
//! - [`EpubManifest`]: Manifest resources (HTML, images, CSS, Media Overlays (SMIL))
//! - [`EpubSpine`]: Canonical reading order
//! - [`EpubToc`]: Table of Contents (e.g., guide, landmarks, page list)
//!
//! EPUBs can be opened using [`Epub::open`] or [`Epub::read`].
//! For finer-grain control over parser behavior, such as strictness and skipping
//! specific components for performance, see [`EpubOpenOptions`].
//!
//! ## Renditions
//! While rare, multi-rendition EPUBs are not fully supported currently.
//! The first `rootfile` declared in `META-INF/container.xml`
//! will always be selected as the active rendition.
//!
//! ## Examples
//! - Opening an [`Epub`] and inspecting the manifest:
//! ```
//! # use rbook::epub::Epub;
//! # use rbook::epub::metadata::EpubVersion;
//! # fn main() -> rbook::ebook::errors::EbookResult<()> {
//! let epub = Epub::options()
//!     .strict(true)
//!     .open("tests/ebooks/example_epub")?;
//!
//! assert!(epub.metadata().version().is_epub3());
//!
//! let manifest = epub.manifest();
//! assert_eq!(5, manifest.readable_content().count());
//! assert_eq!(3, manifest.images().count());
//! assert_eq!(0, manifest.audio().count());
//! assert_eq!(12, manifest.len());
//!
//! let cover = manifest.cover_image().unwrap();
//! assert_eq!("cover-image1", cover.id());
//! assert_eq!("/EPUB/img/cover.webm", cover.href());
//!
//! // Or `copy_bytes` to copy directly into any `Write` implementation
//! let cover_bytes = cover.read_bytes().unwrap();
//! # Ok(())
//! # }
//! ```
//!
//! # Writing
//! Modifying or creating an [`Epub`] is done through the write API,
//! available via the `write` crate feature (enabled by default).
//!
//! The API provides two levels of access:
//! - [`EpubEditor`]: A high-level abstraction for common tasks
//!   such as updating metadata, adding [authors](EpubEditor::author),
//!   and inserting [resources](EpubEditor::resource) or [chapters](EpubEditor::chapter).
//! ```
//! # #[cfg(feature = "write")]
//! # {
//! # use rbook::Epub;
//! Epub::builder()
//!     .author("John Doe")
//!     .resource(("stylesheet.css", "ol { list-style: none; }"));
//! # }
//! ```
//!
//! Lower-level write API:
//! - [`EpubPackageMut`](package::EpubPackageMut): Modify package details and prefixes.
//! - [`EpubMetadataMut`](metadata::EpubMetadataMut): Modify metadata entries and refinements.
//! - [`EpubManifestMut`](manifest::EpubManifestMut): Add, remove, or relocate resources.
//! - [`EpubSpineMut`](spine::EpubSpineMut): Reorder the reading sequence.
//! - [`EpubTocMut`](toc::EpubTocMut): Modify navigation hierarchy.
//! ```
//! # #[cfg(feature = "write")]
//! # {
//! # use rbook::Epub;
//! # use rbook::epub::manifest::DetachedEpubManifestEntry;
//! # use rbook::epub::metadata::DetachedEpubMetaEntry;
//! # fn main() {
//! # let mut epub = Epub::new();
//! epub.metadata_mut().push(
//!     // Create entries detached from the EPUB, then insert them.
//!     DetachedEpubMetaEntry::creator("Jane Doe")
//!         .file_as("Doe, Jane")
//!         .role("edt"),
//! );
//! epub.manifest_mut().push(
//!     DetachedEpubManifestEntry::new("stylesheet_1_id")
//!         .href("stylesheet.css")
//!         .content("ol { list-style: none; }"),
//! );
//! # }
//! # }
//! ```
//!
//! Output configuration, such as [compression](EpubWriteOptions::compression)
//! and [target compatibility](EpubWriteOptions::target) is available via [`EpubWriteOptions`].
//!
//! ## Cascading Updates
//! Modifying a manifest entry's `id` or `href` triggers a cascading update across an [`Epub`]:
//!
//! - Renaming a resource `id` updates all referencing `idref` values in the
//!   [spine](EpubSpine).
//! - Updating the `href` updates all associated navigation links in the
//!   [Table of Contents](EpubToc).
//!
//! See [`EpubManifestEntryMut::set_id`](manifest::EpubManifestEntryMut::set_id)
//! and [`EpubManifestEntryMut::set_href`](manifest::EpubManifestEntryMut::set_href)
//! for more details.
//!
//! ## Auto-Generation
//! Several structural requirements are handled automatically to produce a well-formed EPUB:
//!
//! ### Timestamps
//! If not set, the publication and modification dates
//! are generated automatically using the system clock in
//! [**ISO 8601-1**](https://www.iso.org/iso-8601-date-and-time-format.html) format.
//!
//! - For EPUB 2, the modification date is not generated.
//! - **WebAssembly**: On `wasm32-unknown-unknown`, the system clock is unavailable.
//!   Timestamps can be provided manually via [`EpubEditor::published_date`]
//!   and [`EpubEditor::modified_date`].
//!
//! ### IDs
//! To uphold structural integrity, unique IDs are generated automatically for:
//! - **Refined Entries**: Entries in the [spine](spine::EpubSpineEntry) or
//!   [metadata](metadata::EpubMetaEntry) that contain nested refinements but
//!   lack an explicit ID.
//! - **NCX Entries**: `navPoint` entries that lack an explicit ID.
//!
//! ### See Also
//! - [`EpubWriteOptions::target`] for additional EPUB 2 auto-generation details.
//! - [`EpubWriteOptions::generate_toc`] for ToC generation details.
//!
//! ## Cleaning
//! When performing heavy modifications (such as removing manifest resources),
//! it is recommended to call [`Epub::cleanup`],
//! which removes orphaned references in the spine and table of contents.
//!
//! ## XML Escaping
//! All metadata values, chapter titles, labels, and attribute values are stored as
//! unescaped plain text (e.g., `"1 < 2 & 3"`).
//!
//! The following characters are automatically XML-escaped
//! when [writing](Epub::write) to a destination:
//!
//! | Character | Entity Mapping |
//! |-----------|----------------|
//! | `<`       | `&lt;`         |
//! | `>`       | `&gt;`         |
//! | `&`       | `&amp;`        |
//! | `"`       | `&quot;`       |
//! | `'`       | `&apos;`       |
//!
//! To preserve author intent across reading systems,
//! certain whitespace characters are encoded as numeric character references:
//!
//! | Character  | Numeric Character Reference Mapping     |
//! |------------|-----------------------------------------|
//! | `\t`       | `&#9;`                                  |
//! | `\n`       | `&#10;`                                 |
//! | `\r`       | `&#13;`                                 |
//! | `\u{00A0}` | `&#160;` (Non-breaking space: `&nbsp;`) |

mod archive;
mod consts;
pub mod errors;
pub mod manifest;
pub mod metadata;
pub mod package;
mod parser;
pub mod reader;
pub mod spine;
pub mod toc;
#[cfg(feature = "write")]
mod write;

use crate::ebook::Ebook;
use crate::ebook::archive::zip::ZipArchive;
use crate::ebook::archive::{Archive, ResourceProvider, get_archive};
use crate::ebook::epub::archive::EpubArchive;
use crate::ebook::epub::manifest::{EpubManifest, EpubManifestData};
use crate::ebook::epub::metadata::{EpubMetadata, EpubMetadataData, EpubVersion};
use crate::ebook::epub::package::{EpubPackage, EpubPackageData};
use crate::ebook::epub::parser::{EpubParseConfig, EpubParser};
use crate::ebook::epub::reader::{EpubReader, EpubReaderOptions};
use crate::ebook::epub::spine::{EpubSpine, EpubSpineData};
use crate::ebook::epub::toc::{EpubToc, EpubTocData};
use crate::ebook::errors::{ArchiveResult, EbookError, EbookResult};
use crate::ebook::resource::{Resource, ResourceKey};
use crate::util::{self, Sealed, uri};
use std::borrow::Cow;
use std::io::{Read, Seek, Write};
use std::path::Path;

#[cfg(feature = "write")]
pub use write::{EpubChapter, EpubEditor, EpubWriteOptions, OrphanFilter};

/// [`Ebook`]: Electronic Publication (EPUB)
///
/// Provides access to the following contents of an epub:
/// - [`EpubPackage`]: Package details (epub version, package location/directory, prefixes)
/// - [`EpubMetadata`]: Metadata details (title, language, identifiers, version)
/// - [`EpubManifest`]: Manifest resources (HTML, images, CSS, Media Overlays (SMIL))
/// - [`EpubSpine`]: Canonical reading order
/// - [`EpubToc`]: Table of contents, Guide and Landmarks
///
/// # Configuration
/// Parsing can be configured using [`EpubOpenOptions`].
///
/// Toggling the crate feature `threadsafe` (enabled by default)
/// makes [`Epub`] implement `Send + Sync`:
/// ```toml
/// [dependencies]
/// rbook = { version = "…", features = ["threadsafe"] }
/// ```
/// # Renditions
/// Multi-rendition EPUBs are not fully supported,
/// and the first OPF `rootfile` will always be selected.  
///
/// # See Also
/// - [`EpubEditor`] to conveniently [create](Epub::builder) or [modify](Epub::edit) an [`Epub`].
/// - [`epub`](self) trait-level documentation for more information.
///
/// # Examples
/// - Reading the contents of an epub:
/// ```
/// # use rbook::Epub;
/// # fn main() -> rbook::ebook::errors::EbookResult<()> {
/// let epub = Epub::open("tests/ebooks/example_epub")?;
///
/// // Retrieving the main title
/// assert_eq!("Example EPUB", epub.metadata().title().unwrap().value());
///
/// // Printing the contents of each page
/// for result in epub.reader() {
///     let details = result.unwrap();
///     let media_type = details.manifest_entry().media_type();
///     let xhtml = details.content();
///
///     println!("{media_type}: {xhtml}");
/// }
/// # Ok(())
/// # }
/// ```
#[derive(Debug)]
pub struct Epub {
    archive: EpubArchive,
    package: EpubPackageData,
    metadata: EpubMetadataData,
    manifest: EpubManifestData,
    spine: EpubSpineData,
    toc: EpubTocData,
}

impl Epub {
    /// Returns a builder to open an [`Epub`] with specific options.
    ///
    /// # Examples
    /// - Opening an EPUB with specific options:
    /// ```
    /// # use rbook::epub::Epub;
    /// # use rbook::epub::metadata::EpubVersion;
    /// # fn main() -> rbook::ebook::errors::EbookResult<()> {
    /// let epub = Epub::options()
    ///     .strict(true)
    ///     .retain_variants(true)
    ///     .skip_metadata(true)
    ///     .skip_spine(true)
    ///     .preferred_toc(EpubVersion::EPUB2)
    ///     .open("tests/ebooks/example_epub")?;
    /// # Ok(())
    /// # }
    /// ```
    pub fn options() -> EpubOpenOptions {
        EpubOpenOptions::new()
    }

    /// Opens an [`Epub`] from the given [`Path`] with default [`EpubOpenOptions`].
    ///
    /// The given path may be an EPUB **file** or **directory** containing the
    /// contents of an unzipped EPUB.
    ///
    /// # Note
    /// [`EpubOpenOptions::strict`] is set to `false` by default.
    ///
    /// # Errors
    /// - [`ArchiveError`](EbookError::Archive): Missing or invalid EPUB files.
    /// - [`FormatError`](EbookError::Format): Malformed EPUB content.
    ///
    /// # See Also
    /// - [`Self::options`] to specify options.
    /// - [`Self::read`] to read from a byte buffer.
    /// - [`EpubOpenOptions::read`] to read from a byte buffer with specific options.
    ///
    /// # Examples
    /// - Opening from an EPUB file:
    ///   ```no_run
    ///   # use rbook::Epub;
    ///   let epub = Epub::open("/ebooks/zipped.epub");
    ///   ```
    /// - Opening from a directory containing the contents of an unzipped EPUB:
    ///   ```no_run
    ///   # use rbook::Epub;
    ///   let epub = Epub::open("/ebooks/unzipped_epub_dir");
    ///   ```
    pub fn open(path: impl AsRef<Path>) -> EbookResult<Self> {
        Self::options().open(path)
    }

    /// Opens an EPUB from any implementation of [`Read`] + [`Seek`]
    /// with default [`EpubOpenOptions`].
    ///
    /// # Thread-safety
    /// [`Send`] + [`Sync`] are required constraints if the
    /// `threadsafe` feature is enabled (**enabled by default**).
    ///
    /// Thread-safety can be disabled in a project's `Cargo.toml` file.
    /// See the [base documentation](crate)
    /// for a list of the default crate features and an example.
    ///
    /// # Errors
    /// - [`ArchiveError`](EbookError::Archive): Missing or invalid EPUB files.
    /// - [`FormatError`](EbookError::Format): Malformed EPUB content.
    ///
    /// # See Also
    /// - [`Self::options`] to specify options.
    /// - [`Self::open`] to open from a path (file or directory).
    /// - [`EpubOpenOptions::read`] to open an [`Epub`] with specific options applied.
    pub fn read<
        #[cfg(feature = "threadsafe")] R: 'static + Read + Seek + Send + Sync,
        #[cfg(not(feature = "threadsafe"))] R: 'static + Read + Seek,
    >(
        source: R,
    ) -> EbookResult<Self> {
        Self::options().read(source)
    }

    /// Returns a builder to create an [`EpubReader`] with specific options.
    ///
    /// # Examples
    /// - Retrieving a new EPUB reader instance with configuration:
    /// ```
    /// # use rbook::Epub;
    /// # use rbook::epub::reader::LinearBehavior;
    /// # fn main() -> rbook::ebook::errors::EbookResult<()> {
    /// let epub = Epub::open("tests/ebooks/epub2")?;
    /// let mut epub_reader = epub.reader_builder()
    ///     // Omit linear readable entries
    ///     .linear_behavior(LinearBehavior::NonLinearOnly)
    ///     .create();
    ///
    /// for entry in epub_reader {
    ///     // handle non-linear entry
    /// }
    /// # Ok(())
    /// # }
    /// ```
    pub fn reader_builder(&self) -> EpubReaderOptions<&Self> {
        EpubReaderOptions::<&Self>::new(self)
    }

    /// Returns a new [`EpubReader`] to sequentially read over the [`EpubSpine`]
    /// contents of an ebook.
    #[doc = util::inherent_doc!(Ebook, metadata)]
    /// # See Also
    /// - [`Self::reader_builder`] to alter the behavior of an [`EpubReader`].
    pub fn reader(&self) -> EpubReader<'_> {
        self.reader_builder().create()
    }

    /// The package, encompassing `<package>` element data.
    pub fn package(&self) -> EpubPackage<'_> {
        EpubPackage::new(&self.package)
    }

    /// Data associated with an [`Epub`], such as
    /// [title](EpubMetadata::title) and [author](EpubMetadata::creators) information.
    #[doc = util::inherent_doc!(Ebook, metadata)]
    pub fn metadata(&self) -> EpubMetadata<'_> {
        EpubMetadata::new(&self.package, &self.metadata)
    }

    /// The [`EpubManifest`], encompassing the publication [`resources`](Resource)
    /// contained within an [`Epub`].
    #[doc = util::inherent_doc!(Ebook, manifest)]
    pub fn manifest(&self) -> EpubManifest<'_> {
        EpubManifest::new(
            ResourceProvider::Archive(&self.archive),
            (&self.package).into(),
            &self.manifest,
            &self.metadata,
        )
    }

    /// The [`EpubSpine`], encompassing the canonical reading-order sequence.
    #[doc = util::inherent_doc!(Ebook, spine)]
    pub fn spine(&self) -> EpubSpine<'_> {
        EpubSpine::new(self.manifest().into(), (&self.package).into(), &self.spine)
    }

    /// The table of contents ([`EpubToc`]), encompassing navigation points
    /// (e.g., landmarks, guide, page list).
    #[doc = util::inherent_doc!(Ebook, toc)]
    pub fn toc(&self) -> EpubToc<'_> {
        EpubToc::new(self.manifest().into(), &self.toc)
    }

    /// Copies the content of a [`Resource`] into the given `writer`,
    /// returning the total number of bytes written on success.
    ///
    /// # Normalization
    /// If the given [`Resource::key`] is a relative path,
    /// it is appended to [`EpubPackage::directory`].
    /// Such behavior may be circumvented by making the path absolute by adding a forward
    /// slash (`/`) before ***any*** components.
    ///
    /// Paths are percent-decoded ***then*** normalized before resource retrieval.
    #[doc = util::inherent_doc!(Ebook, copy_resource)]
    /// # See Also
    /// - [`EpubManifestEntry::copy_bytes`](manifest::EpubManifestEntry::copy_bytes)
    ///   to copy the content directly from a manifest entry.
    /// - [`Self::read_resource_str`] to retrieve the content as a [`String`].
    /// - [`Self::read_resource_bytes`] to retrieve the content as bytes.
    ///
    /// # Examples:
    /// - Retrieving file content:
    /// ```
    /// # use rbook::Epub;
    /// # fn main() -> rbook::ebook::errors::EbookResult<()> {
    /// let epub = Epub::open("tests/ebooks/example_epub")?;
    ///
    /// assert_eq!("/EPUB", epub.package().directory().as_str());
    ///
    /// // Absolute chapter path:
    /// let c1 = epub.read_resource_bytes("/EPUB/c1.xhtml")?;
    /// // Resolves to existing `/EPUB/c1.xhtml`:
    /// assert_eq!(c1, epub.read_resource_bytes("c1.xhtml")?);
    /// // Resolves to non-existing `/c1.xhtml`:
    /// assert!(epub.read_resource_bytes("/c1.xhtml").is_err());
    /// // Resolves to non-existing `/EPUB/EPUB/c1.xhtml`:
    /// assert!(epub.read_resource_bytes("EPUB/c1.xhtml").is_err());
    ///
    /// // Absolute container path:
    /// let toc = epub.read_resource_bytes("/META-INF/container.xml")?;
    /// // Resolves to existing `/META-INF/container.xml`:
    /// assert_eq!(toc, epub.read_resource_bytes("../META-INF/container.xml")?);
    /// // Resolves to existing `/META-INF/container.xml`:
    /// assert_eq!(toc, epub.read_resource_bytes("/EPUB/../META-INF/container.xml")?);
    /// // Resolves to non-existing `/EPUB/META-INF/container.xml`:
    /// assert!(epub.read_resource_bytes("META-INF/container.xml").is_err());
    /// // Resolves to non-existing `/EPUB/META-INF/container.xml`:
    /// assert!(epub.read_resource_bytes("EPUB/../META-INF/container.xml").is_err());
    /// # Ok(())
    /// # }
    /// ```
    pub fn copy_resource<'a>(
        &self,
        resource: impl Into<Resource<'a>>,
        mut writer: &mut impl Write,
    ) -> ArchiveResult<u64> {
        self.archive
            .copy_resource_decoded(&self.transform_resource(resource.into()), &mut writer)
    }

    /// Returns the content of a [`Resource`] as a string.
    #[doc = util::inherent_doc!(Ebook, read_resource_str)]
    /// # See Also
    /// - [`Self::copy_resource`] for path normalization details.
    /// - [`EpubManifestEntry::read_str`](manifest::EpubManifestEntry::read_str)
    ///   to retrieve the content directly from a manifest entry.
    pub fn read_resource_str<'a>(
        &self,
        resource: impl Into<Resource<'a>>,
    ) -> ArchiveResult<String> {
        Ebook::read_resource_str(self, resource)
    }

    /// Returns the content of a [`Resource`] as bytes.
    #[doc = util::inherent_doc!(Ebook, read_resource_bytes)]
    /// # See Also
    /// - [`Self::copy_resource`] for path normalization details.
    /// - [`EpubManifestEntry::read_bytes`](manifest::EpubManifestEntry::read_bytes)
    ///   to retrieve the content directly from a manifest entry.
    pub fn read_resource_bytes<'a>(
        &self,
        resource: impl Into<Resource<'a>>,
    ) -> ArchiveResult<Vec<u8>> {
        Ebook::read_resource_bytes(self, resource)
    }

    //////////////////////////////////
    // PRIVATE API
    //////////////////////////////////

    fn transform_resource<'a>(&self, resource: Resource<'a>) -> Resource<'a> {
        // Decoding must happen first before path normalization
        let decoded_href = match resource.key() {
            ResourceKey::Value(value) => uri::decode(value),
            ResourceKey::Position(_) => return resource,
        };

        let normalized = if decoded_href.starts_with(uri::SEPARATOR) {
            uri::normalize(&decoded_href)
        } else {
            uri::resolve(&self.package().directory().decode(), &decoded_href)
        };

        if let Cow::Owned(normalized) = normalized {
            // Input is not normalized
            resource.swap_value(normalized)
        } else if let Cow::Owned(decoded) = decoded_href {
            // Input is normalized, not decoded
            resource.swap_value(decoded)
        } else {
            // Input is already decoded and normalized
            resource
        }
    }

    fn parse(config: &EpubParseConfig, archive: Box<dyn Archive>) -> EbookResult<Self> {
        let archive = EpubArchive::new(archive);
        let data = EpubParser::new(config, &archive).parse()?;

        Ok(Self {
            archive,
            package: data.package,
            metadata: data.metadata,
            manifest: data.manifest,
            spine: data.spine,
            toc: data.toc,
        })
    }
}

impl Sealed for Epub {}

#[allow(refining_impl_trait)]
impl Ebook for Epub {
    fn reader(&self) -> EpubReader<'_> {
        self.reader()
    }

    fn metadata(&self) -> EpubMetadata<'_> {
        self.metadata()
    }

    fn manifest(&self) -> EpubManifest<'_> {
        self.manifest()
    }

    fn spine(&self) -> EpubSpine<'_> {
        self.spine()
    }

    fn toc(&self) -> EpubToc<'_> {
        self.toc()
    }

    fn copy_resource<'a>(
        &self,
        resource: impl Into<Resource<'a>>,
        mut writer: &mut impl Write,
    ) -> ArchiveResult<u64> {
        self.copy_resource(resource, &mut writer)
    }
}

impl PartialEq for Epub {
    fn eq(&self, other: &Self) -> bool {
        self.package == other.package
            && self.metadata() == other.metadata()
            && self.manifest() == other.manifest()
            && self.spine() == other.spine()
            && self.toc() == other.toc()
    }
}

/// Configuration to open an [`Epub`] from a source.
///
/// # Options
/// ## Parsing Behavior
/// - [`strict`](EpubOpenOptions::strict) (Default: `false`)
/// - [`skip_metadata`](EpubOpenOptions::skip_metadata) (Default: `false`)
/// - [`skip_manifest`](EpubOpenOptions::skip_manifest) (Default: `false`)
/// - [`skip_spine`](EpubOpenOptions::skip_spine) (Default: `false`)
/// - [`skip_toc`](EpubOpenOptions::skip_toc) (Default: `false`)
/// ## Table of Contents
/// - [`preferred_toc`](EpubOpenOptions::preferred_toc) (Default: [`EpubVersion::Epub3`])
/// - [`retain_variants`](EpubOpenOptions::retain_variants) (Default: `false`)
///
/// # Examples
/// - Supplying specific options to open an [`Epub`] with:
/// ```no_run
/// # use rbook::Epub;
/// # use rbook::epub::metadata::EpubVersion;
/// # fn main() -> rbook::ebook::errors::EbookResult<()> {
/// let mut options = Epub::options(); // returns `EpubOpenOptions`
///
/// options.retain_variants(true)
///        .strict(true)
///        .skip_toc(true)
///        .preferred_toc(EpubVersion::EPUB2);
///
/// // Opening multiple EPUBs with the same options
/// let epub1 = options.open("example.epub")?;
/// let epub2 = options.open("lotr.epub")?;
/// let epub3 = options.open("sao.epub")?;
/// # Ok(())
/// # }
/// ```
#[derive(Clone, Debug, Default)]
pub struct EpubOpenOptions(EpubParseConfig);

impl EpubOpenOptions {
    /// Creates a new builder with default values.
    ///
    /// # See Also
    /// - [`Epub::options`]
    pub fn new() -> Self {
        Self(EpubParseConfig::default())
    }

    /// Opens an [`Epub`] from the given [`Path`] with the specified [options](EpubOpenOptions).
    ///
    /// The given path may be an EPUB **file** or **directory** containing the
    /// contents of an unzipped EPUB.
    ///
    /// # Errors
    /// - [`ArchiveError`](EbookError::Archive): Missing or invalid EPUB files.
    /// - [`FormatError`](EbookError::Format): Malformed EPUB content.
    ///
    /// # See Also
    /// - [`Self::read`] to open from a byte buffer.
    /// - [`Epub::open`] to open an [`Epub`] with default options applied.
    pub fn open(&self, path: impl AsRef<Path>) -> EbookResult<Epub> {
        Epub::parse(
            &self.0,
            get_archive(path.as_ref()).map_err(EbookError::Archive)?,
        )
    }

    /// Opens an EPUB from any implementation of [`Read`] + [`Seek`]
    /// with the specified [options](EpubOpenOptions).
    ///
    /// # Thread-safety
    /// [`Send`] + [`Sync`] are required constraints if the
    /// `threadsafe` feature is enabled (**enabled by default**).
    ///
    /// Thread-safety can be disabled in a project's `Cargo.toml` file.
    /// See the [base documentation](crate)
    /// for a list of the default crate features and an example.
    ///
    /// # Errors
    /// - [`ArchiveError`](EbookError::Archive): Missing or invalid EPUB files.
    /// - [`FormatError`](EbookError::Format): Malformed EPUB content.
    ///
    /// # See Also
    /// - [`Self::open`] to open from a path (file or directory).
    /// - [`Epub::read`] to open an [`Epub`] with default options applied.
    ///
    /// # Examples
    /// - Opening from a [`Cursor`](std::io::Cursor) with an underlying [`Vec`] containing bytes:
    /// ```no_run
    /// # use rbook::epub::Epub;
    /// # fn main() -> rbook::ebook::errors::EbookResult<()> {
    /// # let epub_bytes = b"";
    /// let bytes_vec: Vec<u8> = Vec::from(epub_bytes);
    /// let cursor = std::io::Cursor::new(bytes_vec);
    /// let epub = Epub::options().read(cursor)?;
    /// # Ok(())
    /// # }
    /// ```
    /// - Opening from a [`File`](std::fs::File) directly:
    /// ```no_run
    /// # use rbook::epub::Epub;
    /// # use std::error::Error;
    /// # fn main() -> Result<(), Box<dyn Error>> {
    /// let epub_file = std::fs::File::open("tests/ebooks/example.epub")?;
    /// let epub = Epub::options().read(epub_file)?;
    /// # Ok(())
    /// # }
    /// ```
    pub fn read<
        #[cfg(feature = "threadsafe")] R: 'static + Read + Seek + Send + Sync,
        #[cfg(not(feature = "threadsafe"))] R: 'static + Read + Seek,
    >(
        &self,
        source: R,
    ) -> EbookResult<Epub> {
        Epub::parse(&self.0, Box::new(ZipArchive::new(source, None)?))
    }

    //////////////////////////////////
    // BUILDING
    //////////////////////////////////

    /// Prefer a specific Table of Contents variant (e.g., EPUB 2 NCX).
    ///
    /// **This affects EPUB 3 ebooks that are backwards-compatible with EPUB 2.**
    ///
    /// This option dictates which variant is parsed if [`Self::retain_variants`]
    /// is set to `false`.
    ///
    /// **Variants**:
    /// - EPUB 2: `navMap (ncx)`
    /// - EPUB 3: `toc (xhtml)`
    ///
    /// If the preferred variant is not available,
    /// the other variant is used instead.
    ///
    /// Default: [`EpubVersion::Epub3`]
    pub fn preferred_toc(&mut self, version: EpubVersion) -> &mut Self {
        self.0.preferred_toc = version.as_major();
        self
    }

    /// Retain **both** EPUB 2 and 3-specific information.
    ///
    /// **Currently, this only pertains to the table of contents.**
    ///
    /// When set to `true`, if the EPUB contains both an EPUB 2 **NCX** ToC
    /// and EPUB 3 **XHTML** ToC, they will both be parsed and
    /// added to [`EpubToc`].
    /// **Otherwise, only the format specified by
    /// [`Self::preferred_toc`] will be retained.**
    ///
    /// # Modification Behavior
    /// When enabled, all variants are treated independently.
    /// Modifications made to one (e.g., inserting a ToC entry pointing to a chapter)
    /// will **not** automatically sync to the other.
    /// Changes must be applied to both variants manually.
    ///
    /// Only one variant is required when writing.
    /// See [`EpubWriteOptions::generate_toc`] for more details.
    ///
    /// Default: `false`
    pub fn retain_variants(&mut self, retain: bool) -> &mut Self {
        self.0.retain_variants = retain;
        self
    }

    /// When set to `true`, ensures an EPUB conforms to the following:
    /// - Has an [**identifier**](super::Metadata::identifier).
    /// - Has a [**title**](super::Metadata::title).
    /// - Has a primary [**language**](super::Metadata::language).
    /// - Has a [**version**](EpubMetadata::version) where `2.0 <= version < 4.0`.
    /// - Has a [**table of contents**](super::Toc::contents).
    /// - Elements (e.g., `item`, `itemref`) have their required attributes present.
    ///
    /// If any of the conditions are not met,
    /// an error will be returned.
    ///
    /// **This setting does not validate that an EPUB conforms entirely to the spec.
    /// However, it will refuse further processing if malformations are found.**
    ///
    /// # See Also
    /// - [`EpubError`](errors::EpubError) to see which
    ///   format errors are ignored when strict mode disabled.
    ///
    /// Default: `false`
    pub fn strict(&mut self, strict: bool) -> &mut Self {
        self.0.strict = strict;
        self
    }

    /// When set to `true`, all parsing for [`EpubMetadata`] is skipped.
    /// This is useful as a *speed and space optimization*
    /// when all metadata-related info (e.g., title, author, etc.) is not required.
    ///
    /// If `true`, [`Epub::metadata`] will return an empty [`EpubMetadata`] instance.
    /// However, version-associated methods (e.g., [`EpubMetadata::version`]) will work,
    /// as version information is stored independently of internal EPUB metadata entries.
    ///
    /// # Side Effects
    /// - **Refinements**:
    ///   Methods that return refinements **will** return an empty container (e.g.,
    ///   [`EpubManifestEntry::refinements`](manifest::EpubManifestEntry::refinements),
    ///   [`EpubSpineEntry::refinements`](spine::EpubSpineEntry::refinements)).
    ///
    /// Default: `false`
    pub fn skip_metadata(&mut self, skip: bool) -> &mut Self {
        self.0.parse_metadata = !skip;
        self
    }

    /// When set to `true`, all parsing for [`EpubManifest`] is skipped.
    /// This is useful as a *speed and space optimization*
    /// when all manifest-related info is not required.
    ///
    /// If `true`, [`Epub::manifest`] will return an empty [`EpubManifest`] instance.
    ///
    /// # Side Effects
    /// - **Table of Contents**:
    ///   As [`EpubToc`] mainly relies on the manifest to resolve ToC-related resources,
    ///   setting this to `true` **prevents detecting ToC files** (e.g. `toc.ncx`),
    ///   even if [`Self::skip_toc`] is set to `false`.
    ///
    ///   **Exception**: The EPUB 2 `guide` (which resides within the package file)
    ///   will still be parsed if present, as it does not depend on the manifest.
    ///   To disable this, set [`Self::skip_toc`] to `true`.
    ///
    /// - **Manifest Entries & Resources**:
    ///   Methods that resolve to a manifest entry or [`Resource`] **will** return [`None`] (e.g.,
    ///   [`EpubSpineEntry::manifest_entry`](super::spine::SpineEntry::manifest_entry),
    ///   [`EpubTocEntry::resource`](super::toc::TocEntry::resource)).
    ///
    /// - **Reader Content**:
    ///   Accessing content from an [`EpubReader`]
    ///   **will** result in a [`ReaderError`](crate::reader::errors::ReaderError),
    ///   as it requires a lookup to the manifest.
    ///
    /// Default: `false`
    pub fn skip_manifest(&mut self, skip: bool) -> &mut Self {
        self.0.parse_manifest = !skip;
        self
    }

    /// When set to `true`, all parsing for [`EpubSpine`] is skipped.
    /// This is useful as a *speed and space optimization*
    /// when all spine-related info is not required.
    ///
    /// If `true`, [`Epub::spine`] will return an empty [`EpubSpine`] instance.
    ///
    /// # Side Effects
    /// - **Readers**:
    ///   Each created [`EpubReader`] **will** be untraversable
    ///   as readers requires a lookup to the spine.
    ///
    /// Default: `false`
    pub fn skip_spine(&mut self, skip: bool) -> &mut Self {
        self.0.parse_spine = !skip;
        self
    }

    /// When set to `true`, all parsing for [`EpubToc`] is skipped
    /// (e.g., toc, guide, landmarks, etc.).
    /// This is useful as a *speed and space optimization*
    /// when all ToC-related info is not required.
    ///
    /// If `true`, [`Epub::toc`] will return an empty [`EpubToc`] instance.
    ///
    /// Default: `false`
    pub fn skip_toc(&mut self, skip: bool) -> &mut Self {
        self.0.parse_toc = !skip;
        self
    }
}
