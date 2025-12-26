//! The Electronic Publication ([`Epub`]) module.
//!
//! Supports EPUB versions `2` and `3`.
//!
//! For more information regarding the EPUB spec, see:
//! <https://www.w3.org/TR/epub>

mod consts;
pub mod errors;
pub mod manifest;
pub mod metadata;
mod parser;
pub mod reader;
pub mod spine;
pub mod toc;

use crate::ebook::Ebook;
use crate::ebook::archive::zip::ZipArchive;
use crate::ebook::archive::{self, Archive, ResourceArchive};
use crate::ebook::element::Href;
use crate::ebook::epub::manifest::{EpubManifest, EpubManifestData};
use crate::ebook::epub::metadata::{EpubMetadata, EpubMetadataData, EpubVersion};
use crate::ebook::epub::parser::EpubParser;
use crate::ebook::epub::reader::{EpubReader, EpubReaderBuilder, EpubReaderOptions};
use crate::ebook::epub::spine::{EpubSpine, EpubSpineData};
use crate::ebook::epub::toc::{EpubToc, EpubTocData};
use crate::ebook::errors::{EbookError, EbookResult};
use crate::ebook::resource::{Resource, ResourceKey};
use crate::util::uri;
use std::io::{Read, Seek};
use std::path::Path;

/// [`Ebook`]: Electronic Publication (EPUB)
///
/// Provides access to the following contents of an epub:
/// - [`EpubMetadata`]: Metadata details (epub version, title, language, identifiers)
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
/// rbook = { version = "...", features = ["threadsafe"] }
/// ```
/// # Renditions
/// Multi-rendition EPUBs are not fully supported,
/// and the first OPF `rootfile` will always be selected.  
///
/// # Examples
/// - Reading the contents of an epub:
/// ```
/// # use rbook::ebook::errors::EbookResult;
/// # use rbook::ebook::manifest::{Manifest, ManifestEntry};
/// # use rbook::ebook::metadata::{MetaEntry, Metadata};
/// # use rbook::reader::ReaderContent;
/// # use rbook::{Ebook, Epub};
/// # fn main() -> EbookResult<()> {
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
    archive: ResourceArchive,
    package_file: String,
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
    /// # use rbook::ebook::errors::EbookResult;
    /// # use rbook::epub::Epub;
    /// # use rbook::epub::metadata::EpubVersion;
    /// # fn main() -> EbookResult<()> {
    /// let epub = Epub::options()
    ///     .strict(false)
    ///     .store_all(true)
    ///     .skip_metadata(true)
    ///     .skip_spine(true)
    ///     .preferred_page_list(EpubVersion::EPUB2)
    ///     .preferred_landmarks(EpubVersion::EPUB2)
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
    /// [`EpubOpenOptions::strict`] is set to `true` by default.
    ///
    /// # Errors
    /// - [`ArchiveError`](EbookError::Archive): Missing or invalid EPUB files.
    /// - [`FormatError`](EbookError::Format): Malformed EPUB content.
    ///
    /// # See Also
    /// - [`Self::options`] to specify options.
    /// - [`EpubOpenOptions::read`] to read from a byte buffer.
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

    /// Returns a builder to create an [`EpubReader`] with specific options.
    ///
    /// # Examples
    /// - Retrieving a new EPUB reader instance with configuration:
    /// ```
    /// # use rbook::{Ebook, Epub};
    /// # use rbook::ebook::errors::EbookResult;
    /// # use rbook::epub::reader::LinearBehavior;
    /// # use rbook::reader::{Reader, ReaderContent};
    /// # fn main() -> EbookResult<()> {
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
    pub fn reader_builder(&self) -> EpubReaderBuilder<'_> {
        EpubReaderBuilder::new(self)
    }

    /// The absolute percent-encoded location of the package `.opf` file.
    ///
    /// This is ***not*** a filesystem path.
    /// It always starts with `/` to indicate the EPUB container root,
    /// and ***is*** percent encoded (e.g., `/my%20dir/my%20pkg.opf`).
    ///
    /// # See Also
    /// - [`Href::decode`] to retrieve the percent-decoded form.
    /// - [`Href::name`] to retrieve the filename.
    ///
    /// # Examples
    /// - Retrieving the package file:
    /// ```
    /// # use rbook::{Ebook, Epub};
    /// # use rbook::ebook::errors::EbookResult;
    /// # fn main() -> EbookResult<()> {
    /// let epub = Epub::open("tests/ebooks/example_epub")?;
    ///
    /// assert_eq!("/EPUB/example.opf", epub.package_file().as_str());
    /// # Ok(())
    /// # }
    /// ```
    pub fn package_file(&self) -> Href<'_> {
        self.package_file.as_str().into()
    }

    /// The absolute percent-encoded directory [`Self::package_file`] resides in.
    ///
    /// This is ***not*** a filesystem path.
    /// It always starts with `/` to indicate the EPUB container root,
    /// and ***is*** percent encoded (e.g., `/my%20dir`).
    ///
    /// [`Resources`](Resource) referenced in the package file are resolved relative to the
    /// package directory.
    ///
    /// # See Also
    /// - [`Href::decode`] to retrieve the percent-decoded form.
    ///
    /// # Examples
    /// - Retrieving the package file and directory:
    /// ```
    /// # use rbook::{Ebook, Epub};
    /// # use rbook::ebook::errors::EbookResult;
    /// # fn main() -> EbookResult<()> {
    /// let epub = Epub::open("tests/ebooks/example_epub")?;
    /// let package_dir = epub.package_directory().as_str();
    /// let package_file = epub.package_file().as_str();
    ///
    /// assert_eq!("/EPUB", package_dir);
    /// assert_eq!(format!("{package_dir}/example.opf"), package_file);
    /// # Ok(())
    /// # }
    /// ```
    pub fn package_directory(&self) -> Href<'_> {
        uri::parent(&self.package_file).into()
    }

    /// Deprecated in favor of [`EpubOpenOptions::read`], accessible via [`Epub::open`].
    #[deprecated(
        since = "0.6.8",
        note = "Use `Epub::options`, then call `EpubOpenOptions::open` instead."
    )]
    pub fn open_with(
        path: impl AsRef<Path>,
        options: impl Into<EpubOpenOptions>,
    ) -> EbookResult<Self> {
        options.into().open(path)
    }

    /// Deprecated in favor of [`EpubOpenOptions::read`], accessible via [`Epub::options`].
    #[deprecated(
        since = "0.6.8",
        note = "Use `Epub::options`, then call `EpubOpenOptions::read` instead."
    )]
    pub fn read<
        #[cfg(feature = "threadsafe")] R: 'static + Read + Seek + Send + Sync,
        #[cfg(not(feature = "threadsafe"))] R: 'static + Read + Seek,
    >(
        reader: R,
        options: impl Into<EpubOpenOptions>,
    ) -> EbookResult<Self> {
        options.into().read(reader)
    }

    /// Deprecated in favor of [`Epub::reader_builder`] / [`EpubReaderOptions::create`].
    #[deprecated(
        since = "0.6.8",
        note = "Use `Epub::reader_builder` or `EpubReaderOptions::create` instead."
    )]
    pub fn reader_with(&self, options: impl Into<EpubReaderOptions>) -> EpubReader<'_> {
        options.into().create(self)
    }

    //////////////////////////////////
    // PRIVATE API
    //////////////////////////////////

    fn transform_resource<'b>(&self, resource: Resource<'b>) -> Resource<'b> {
        let href = match resource.key() {
            ResourceKey::Value(value) => uri::decode(value.as_ref()),
            ResourceKey::Position(_) => return resource,
        };
        let package_dir = self.package_directory().decode();

        let modified_href = if href.starts_with('/') {
            uri::normalize(href.as_ref())
        } else {
            uri::resolve(package_dir.as_ref(), href.as_ref()).into_owned()
        };

        resource.swap_value(modified_href)
    }

    // For now, the preferences from `config` are not stored within the `Epub` struct.
    fn parse(config: EpubConfig, archive: Box<dyn Archive>) -> EbookResult<Self> {
        let mut parser = EpubParser::new(&config, archive.as_ref());
        let data = parser.parse()?;

        Ok(Self {
            archive: ResourceArchive::new(archive),
            package_file: data.package_file,
            metadata: data.metadata,
            manifest: data.manifest,
            spine: data.spine,
            toc: data.toc,
        })
    }
}

#[allow(refining_impl_trait)]
impl Ebook for Epub {
    /// Returns a new [`EpubReader`] to sequentially read over the [`EpubSpine`]
    /// contents of an ebook.
    ///
    /// # See Also
    /// - [`Self::reader_builder`] to alter the behavior of an [`EpubReader`].
    fn reader(&self) -> EpubReader<'_> {
        self.reader_builder().create()
    }

    fn metadata(&self) -> EpubMetadata<'_> {
        EpubMetadata::new(&self.metadata)
    }

    fn manifest(&self) -> EpubManifest<'_> {
        EpubManifest::new(EpubResourceProvider(&self.archive), &self.manifest)
    }

    fn spine(&self) -> EpubSpine<'_> {
        EpubSpine::new(self.manifest().into(), &self.spine)
    }

    fn toc(&self) -> EpubToc<'_> {
        EpubToc::new(self.manifest().into(), &self.toc)
    }

    /// See [`Self::read_resource_bytes`] for EPUB-specific information regarding
    /// normalization.
    fn read_resource_str<'a>(&self, resource: impl Into<Resource<'a>>) -> EbookResult<String> {
        self.archive
            .read_resource_str(&self.transform_resource(resource.into()))
            .map_err(Into::into)
    }

    /// Returns the specified [`Resource`] in the form of bytes.
    ///
    /// If the given [`Resource::key`] is a relative path,
    /// it is appended to [`Self::package_directory`].
    /// Such behavior may be circumvented by making the path absolute by adding a forward
    /// slash (`/`) before ***any*** components.
    ///
    /// Paths are percent-decoded ***then*** normalized before resource retrieval.
    ///
    /// # See Also
    /// - [`Ebook::read_resource_bytes`]
    ///
    /// # Examples:
    /// - Retrieving file content:
    /// ```
    /// # use rbook::{Ebook, Epub};
    /// # use rbook::ebook::errors::EbookResult;
    /// # fn main() -> EbookResult<()> {
    /// let epub = Epub::open("tests/ebooks/example_epub")?;
    ///
    /// assert_eq!("/EPUB", epub.package_directory().as_str());
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
    fn read_resource_bytes<'a>(&self, resource: impl Into<Resource<'a>>) -> EbookResult<Vec<u8>> {
        self.archive
            .read_resource_bytes(&self.transform_resource(resource.into()))
            .map_err(Into::into)
    }
}

impl PartialEq for Epub {
    fn eq(&self, other: &Self) -> bool {
        self.package_file() == other.package_file()
            && self.metadata() == other.metadata()
            && self.manifest() == other.manifest()
            && self.spine() == other.spine()
            && self.toc() == other.toc()
    }
}

#[derive(Clone, Debug)]
pub(crate) struct EpubConfig {
    /// See [`EpubOpenOptions::preferred_toc`].
    pub(crate) preferred_toc: EpubVersion,
    /// See [`EpubOpenOptions::preferred_landmarks`].
    pub(crate) preferred_landmarks: EpubVersion,
    /// See [`EpubOpenOptions::preferred_page_list`].
    pub(crate) preferred_page_list: EpubVersion,
    /// See [`EpubOpenOptions::store_all`].
    pub(crate) store_all: bool,
    /// See [`EpubOpenOptions::strict`].
    pub(crate) strict: bool,
    /// See [`EpubOpenOptions::skip_metadata`]; inverted.
    pub(crate) parse_metadata: bool,
    /// See [`EpubOpenOptions::skip_manifest`]; inverted.
    pub(crate) parse_manifest: bool,
    /// See [`EpubOpenOptions::skip_spine`]; inverted.
    pub(crate) parse_spine: bool,
    /// See [`EpubOpenOptions::skip_toc`]; inverted.
    pub(crate) parse_toc: bool,
}

// Temporary placeholder for now until 0.7.0
#[allow(deprecated)]
impl From<EpubOpenOptions> for EpubConfig {
    fn from(options: EpubOpenOptions) -> Self {
        Self {
            preferred_toc: options.preferred_toc,
            preferred_landmarks: options.preferred_landmarks,
            preferred_page_list: options.preferred_page_list,
            store_all: options.store_all,
            strict: options.strict,
            parse_metadata: !options.skip_metadata,
            parse_manifest: !options.skip_manifest,
            parse_spine: !options.skip_spine,
            parse_toc: !options.skip_toc,
        }
    }
}

// BACKWARD COMPATIBILITY (Renamed)
/// Deprecated; prefer [`EpubOpenOptions`] instead.
#[deprecated(since = "0.6.8", note = "Use `EpubOpenOptions` instead.")]
pub type EpubSettingsBuilder = EpubOpenOptions;
/// Deprecated; prefer [`EpubOpenOptions`] instead.
#[deprecated(since = "0.6.8", note = "Use `EpubOpenOptions` instead.")]
pub type EpubSettings = EpubOpenOptions;

/// Builder to open an [`Epub`].
///
/// # Options
/// ## Parsing Behavior
/// - [`strict`](EpubOpenOptions::strict)
/// - [`skip_metadata`](EpubOpenOptions::skip_metadata)
/// - [`skip_manifest`](EpubOpenOptions::skip_manifest)
/// - [`skip_spine`](EpubOpenOptions::skip_spine)
/// - [`skip_toc`](EpubOpenOptions::skip_toc)
/// ## Table of Contents
/// - [`preferred_toc`](EpubOpenOptions::preferred_toc)
/// - [`preferred_landmarks`](EpubOpenOptions::preferred_landmarks)
/// - [`preferred_page_list`](EpubOpenOptions::preferred_page_list)
/// - [`store_all`](EpubOpenOptions::store_all)
///
/// # Examples
/// - Supplying specific options to open an [`Epub`] with:
/// ```
/// # use rbook::ebook::errors::EbookResult;
/// # use rbook::epub::metadata::EpubVersion;
/// # use rbook::{Ebook, Epub};
/// # fn main() -> EbookResult<()> {
/// let epub = Epub::options() // returns `EpubOpenOptions`
///     .store_all(true)
///     .strict(false)
///     .skip_toc(true)
///     .preferred_landmarks(EpubVersion::EPUB2)
///     .open("tests/ebooks/example_epub")?;
/// # Ok(())
/// # }
/// ```
#[non_exhaustive]
#[derive(Clone, Debug)]
pub struct EpubOpenOptions /*(EpubConfig)*/ {
    // In 0.7.0, all these fields will be replaced with `EpubConfig`.
    /// See [`EpubOpenOptions::preferred_toc`].
    #[deprecated(since = "0.6.8", note = "Use `preferred_toc` method instead.")]
    pub preferred_toc: EpubVersion,
    /// See [`EpubOpenOptions::preferred_landmarks`].
    #[deprecated(since = "0.6.8", note = "Use `preferred_landmarks` method instead.")]
    pub preferred_landmarks: EpubVersion,
    /// See [`EpubOpenOptions::preferred_page_list`].
    #[deprecated(since = "0.6.8", note = "Use `preferred_page_list` method instead.")]
    pub preferred_page_list: EpubVersion,
    /// See [`EpubOpenOptions::store_all`].
    #[deprecated(since = "0.6.8", note = "Use `store_all` method instead.")]
    pub store_all: bool,
    /// See [`EpubOpenOptions::strict`].
    #[deprecated(since = "0.6.8", note = "Use `strict` method instead.")]
    pub strict: bool,
    skip_metadata: bool,
    skip_manifest: bool,
    skip_spine: bool,
    skip_toc: bool,
}

#[allow(deprecated)]
impl EpubOpenOptions {
    /// Creates a new builder with default values.
    ///
    /// # See Also
    /// - [`Epub::options`]
    pub fn new() -> Self {
        Self {
            preferred_toc: EpubVersion::EPUB3,
            preferred_landmarks: EpubVersion::EPUB3,
            preferred_page_list: EpubVersion::EPUB3,
            store_all: false,
            strict: true,
            skip_metadata: false,
            skip_manifest: false,
            skip_spine: false,
            skip_toc: false,
        }
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
    pub fn open(self, path: impl AsRef<Path>) -> EbookResult<Epub> {
        Epub::parse(
            self.into(),
            archive::get_archive(path.as_ref()).map_err(EbookError::Archive)?,
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
    ///
    /// # Examples
    /// - Opening from a [`Cursor`](std::io::Cursor) with an underlying [`Vec`] containing bytes:
    /// ```no_run
    /// # use rbook::ebook::errors::EbookResult;
    /// # use rbook::epub::Epub;
    /// # fn main() -> EbookResult<()> {
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
        self,
        reader: R,
    ) -> EbookResult<Epub> {
        Epub::parse(self.into(), Box::new(ZipArchive::new(reader, None)?))
    }

    //////////////////////////////////
    // BUILDING
    //////////////////////////////////

    /// Prefer a table of contents (toc) format over another as the default
    /// from [`Toc::contents`](super::Toc::contents).
    ///
    /// **Formats**:
    /// - Epub2: `navMap (ncx)`
    /// - Epub3: `toc (xhtml)`
    ///
    /// If the preferred format is not available,
    /// the other format is used instead.
    ///
    /// **This primarily affects EPUB3 ebooks that
    /// are backwards-compatible with EPUB2.**
    ///
    /// Default: [`EpubVersion::Epub3`]
    pub fn preferred_toc(mut self, version: EpubVersion) -> Self {
        self.preferred_toc = version;
        self
    }

    /// Prefer a landmark format over another as the default
    /// from [`EpubToc::landmarks`].
    ///
    /// **Formats**:
    /// - Epub2: `guide (opf)`
    /// - Epub3: `landmarks (xhtml)`
    ///
    /// If the preferred format is not available,
    /// the other format is used instead.
    ///
    /// **This primarily affects EPUB3 ebooks that
    /// are backwards-compatible with EPUB2.**
    ///
    /// Default: [`EpubVersion::Epub3`]
    pub fn preferred_landmarks(mut self, version: EpubVersion) -> Self {
        self.preferred_landmarks = version;
        self
    }

    /// Prefer a page list format over another as the default
    /// from [`EpubToc::page_list`].
    ///
    /// **Formats**:
    /// - Epub2: `pageList (ncx)`
    /// - Epub3: `page-list (xhtml)`
    ///
    /// If the preferred format is not available,
    /// the other format is used instead.
    ///
    /// **This primarily affects EPUB3 ebooks that
    /// are backwards-compatible with EPUB2.**
    ///
    /// Default: [`EpubVersion::Epub3`]
    pub fn preferred_page_list(mut self, version: EpubVersion) -> Self {
        self.preferred_page_list = version;
        self
    }

    /// Store **both** EPUB 2 and 3-specific information.
    ///
    /// **Currently, this only pertains to the table of contents.**
    ///
    /// If the epub contains both an EPUB2 `ncx` toc
    /// and EPUB3 `xhtml` toc, they will both be parsed and
    /// added to [`EpubToc`].
    ///
    /// Default: `false`
    pub fn store_all(mut self, store_all: bool) -> Self {
        self.store_all = store_all;
        self
    }

    /// When set to `true`, ensures an EPUB conforms to the following:
    /// - Has an [**identifier**](super::Metadata::identifier).
    /// - Has a [**title**](super::Metadata::title).
    /// - Has a primary [**language**](super::Metadata::language).
    /// - Has a [**version**](EpubMetadata::version) where `2.0 <= version < 4.0`.
    /// - Has a [**table of contents**](super::Toc::contents).
    /// - Elements (e.g., `item`, `itemref`) have their required attributes present.
    /// - [Hrefs](Href) are **percent-encoded**.
    ///
    /// If any of the conditions are not met,
    /// an error will be returned.
    ///
    /// **This setting does not validate that an EPUB conforms entirely to the spec.
    /// However, it will refuse further processing if malformations are found.**
    ///
    /// # See Also
    /// - [`EpubFormatError`](errors::EpubFormatError) to see which
    ///   format errors are ignored when strict mode disabled.
    ///
    /// Default: `true`
    pub fn strict(mut self, strict: bool) -> Self {
        self.strict = strict;
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
    pub fn skip_metadata(mut self, skip: bool) -> Self {
        self.skip_metadata = skip;
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
    pub fn skip_manifest(mut self, skip: bool) -> Self {
        self.skip_manifest = skip;
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
    pub fn skip_spine(mut self, skip: bool) -> Self {
        self.skip_spine = skip;
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
    pub fn skip_toc(mut self, skip: bool) -> Self {
        self.skip_toc = skip;
        self
    }

    /// Turn this instance into an [`EpubOpenOptions`] instance.
    #[deprecated(
        since = "0.6.8",
        note = "Use `EpubOpenOptions::open` or `EpubOpenOptions::read` instead."
    )]
    pub fn build(self) -> EpubOpenOptions {
        self
    }

    /// Deprecated; prefer [`Epub::options`] instead.
    #[deprecated(
        since = "0.6.8",
        note = "Use `Epub::options` or `EpubOpenOptions::new` instead."
    )]
    pub fn builder() -> Self {
        Self::new()
    }
}

impl Default for EpubOpenOptions {
    fn default() -> Self {
        Self::new()
    }
}

#[derive(Copy, Clone)]
pub(super) struct EpubResourceProvider<'ebook>(&'ebook ResourceArchive);

/// These methods don't delegate to [`Epub::transform_resource`]
/// as the input (`resource`) is **trusted**.
impl EpubResourceProvider<'_> {
    pub(super) fn read_str(&self, resource: Resource) -> EbookResult<String> {
        self.0.read_resource_str(&resource).map_err(Into::into)
    }

    pub(super) fn read_bytes(&self, resource: Resource) -> EbookResult<Vec<u8>> {
        self.0.read_resource_bytes(&resource).map_err(Into::into)
    }
}
