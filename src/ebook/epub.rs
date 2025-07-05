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
use crate::ebook::archive::{self, Archive};
use crate::ebook::element::Href;
use crate::ebook::epub::manifest::{EpubManifest, EpubManifestData};
use crate::ebook::epub::metadata::{EpubMetadata, EpubMetadataData, EpubVersion};
use crate::ebook::epub::parser::EpubParser;
use crate::ebook::epub::reader::{EpubReader, EpubReaderSettings};
use crate::ebook::epub::spine::{EpubSpine, EpubSpineData};
use crate::ebook::epub::toc::{EpubToc, EpubTocData};
use crate::ebook::errors::{EbookError, EbookResult};
use crate::ebook::resource::{Resource, ResourceKey};
use crate::util::uri;
use std::fmt::{Debug, Formatter};
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
/// Parsing can be configured using [`EpubSettings`].
///
/// Enabling `threadsafe` makes [`Epub`] implement `Send + Sync`:
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
pub struct Epub {
    archive: Box<dyn Archive>,
    package_file: String,
    metadata: EpubMetadataData,
    manifest: EpubManifestData,
    spine: EpubSpineData,
    toc: EpubTocData,
}

impl Epub {
    /// Opens an [`Epub`] from the given [`Path`] with default [`EpubSettings`].
    ///
    /// The provided path may be an EPUB **file** or **directory** containing the
    /// contents of an unzipped EPUB.
    ///
    /// # Errors
    /// - [`ArchiveError`](EbookError::Archive): Missing or invalid EPUB files.
    /// - [`FormatError`](EbookError::Format): Malformed EPUB content.
    ///
    /// # See Also
    /// - [`Self::open_with`] to specify settings.
    /// - [`Self::read`] to open from a byte buffer.
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
        Self::open_with(path, EpubSettings::default())
    }

    /// Opens an [`Epub`] from the given [`Path`] with the specified [`EpubSettings`].
    ///
    /// See [`Self::open`] for more details.
    ///
    /// # Examples
    /// - Opening an EPUB with settings:
    /// ```
    /// # use rbook::ebook::errors::EbookResult;
    /// # use rbook::epub::{Epub, EpubSettings};
    /// # use rbook::epub::metadata::EpubVersion;
    /// # fn main() -> EbookResult<()> {
    /// let epub = Epub::open_with(
    ///     "tests/ebooks/example_epub",
    ///     EpubSettings::builder()
    ///         .store_all(true)
    ///         .strict(false)
    ///         .preferred_page_list(EpubVersion::EPUB2)
    ///         .preferred_landmarks(EpubVersion::EPUB2)
    ///         .preferred_toc(EpubVersion::EPUB3)
    /// )?;
    /// # Ok(())
    /// # }
    /// ```
    pub fn open_with(
        path: impl AsRef<Path>,
        settings: impl Into<EpubSettings>,
    ) -> EbookResult<Self> {
        Self::new(
            settings.into(),
            archive::get_archive(path.as_ref()).map_err(EbookError::Archive)?,
        )
    }

    /// With the specified [`EpubSettings`],
    /// opens an EPUB from any implementation of [`Read`] + [`Seek`]
    /// (and [`Send`] + [`Sync`] if the `threadsafe` feature is enabled).
    ///
    /// # Errors
    /// - [`ArchiveError`](EbookError::Archive): Missing or invalid EPUB files.
    /// - [`FormatError`](EbookError::Format): Malformed EPUB content.
    ///
    /// # Examples
    /// - Opening from a [`Cursor`](std::io::Cursor) with an underlying [`Vec`] containing bytes:
    /// ```no_run
    /// # use rbook::ebook::errors::EbookResult;
    /// # use rbook::epub::{Epub, EpubSettings};
    /// # use std::error::Error;
    /// # fn main() -> EbookResult<()> {
    /// # let epub_bytes = b"";
    /// let bytes_vec: Vec<u8> = Vec::from(epub_bytes);
    /// let cursor = std::io::Cursor::new(bytes_vec);
    /// let epub = Epub::read(cursor, EpubSettings::default())?;
    /// # Ok(())
    /// # }
    /// ```
    /// - Opening from a [`File`](std::fs::File) directly:
    /// ```no_run
    /// # use rbook::epub::{Epub, EpubSettings};
    /// # use std::error::Error;
    /// # fn main() -> Result<(), Box<dyn Error>> {
    /// let epub_file = std::fs::File::open("tests/ebooks/example.epub")?;
    /// let epub = Epub::read(epub_file, EpubSettings::default())?;
    /// # Ok(())
    /// # }
    /// ```
    pub fn read<
        #[cfg(feature = "threadsafe")] R: 'static + Read + Seek + Send + Sync,
        #[cfg(not(feature = "threadsafe"))] R: 'static + Read + Seek,
    >(
        reader: R,
        settings: impl Into<EpubSettings>,
    ) -> EbookResult<Self> {
        Self::new(settings.into(), Box::new(ZipArchive::new(reader, None)?))
    }

    /// Returns a new [`EpubReader`] to sequentially read over the [`EpubSpine`]
    /// contents of an ebook with the specified [`EpubReaderSettings`].
    pub fn reader_with(&self, settings: impl Into<EpubReaderSettings>) -> EpubReader {
        EpubReader::new(self, settings.into())
    }

    /// The absolute percent-encoded location of the package `.opf` file.
    ///
    /// This is ***not*** a filesystem path.
    /// It always starts with `/` to indicate the EPUB container root,
    /// and ***is*** percent encoded (e.g., `/my%20dir/my%20pkg.opf`).
    ///
    /// # See Also
    /// - [`Href::decode`]
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
    pub fn package_file(&self) -> Href {
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
    /// - [`Href::decode`]
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
    pub fn package_directory(&self) -> Href {
        uri::parent(&self.package_file).into()
    }

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

    // For now, `EpubSettings` are not stored within the `Epub` struct.
    fn new(settings: EpubSettings, archive: Box<dyn Archive>) -> EbookResult<Self> {
        let mut parser = EpubParser::new(&settings, archive.as_ref());
        let data = parser.parse()?;

        Ok(Self {
            archive,
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
    fn reader(&self) -> EpubReader {
        EpubReader::new(self, EpubReaderSettings::default())
    }

    fn metadata(&self) -> EpubMetadata {
        EpubMetadata::new(&self.metadata)
    }

    fn manifest(&self) -> EpubManifest {
        EpubManifest::new(&self.manifest, EpubResourceProvider(self))
    }

    fn spine(&self) -> EpubSpine {
        EpubSpine::new(self.manifest().into(), &self.spine)
    }

    fn toc(&self) -> EpubToc {
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
    /// // Absolute container path (leading slash stripped):
    /// let c1 = epub.read_resource_bytes("/EPUB/c1.xhtml")?;
    /// // Resolves to existing `/EPUB/c1.xhtml`:
    /// assert_eq!(c1, epub.read_resource_bytes("c1.xhtml")?);
    /// // Resolves to non-existing `/c1.xhtml`:
    /// assert!(epub.read_resource_bytes("/c1.xhtml").is_err());
    /// // Resolves to non-existing `/EPUB/EPUB/c1.xhtml`:
    /// assert!(epub.read_resource_bytes("EPUB/c1.xhtml").is_err());
    ///
    /// // Navigation doc at the root:
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

impl Debug for Epub {
    fn fmt(&self, fmt: &mut Formatter<'_>) -> std::fmt::Result {
        fmt.debug_struct("Epub")
            .field("package_file", &self.package_file)
            .field("metadata", &self.metadata)
            .field("manifest", &self.manifest)
            .field("spine", &self.spine)
            .field("toc", &self.toc)
            .finish_non_exhaustive()
    }
}

impl PartialEq for Epub {
    fn eq(&self, other: &Self) -> bool {
        self.metadata() == other.metadata()
    }
}

/// EPUB-specific settings upon parsing an [`Epub`].
///
/// To create a mutable settings instance, see
/// [`EpubSettings::builder`] or [`EpubSettings::default`].
#[non_exhaustive]
#[derive(Clone, Debug)]
pub struct EpubSettings {
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
    pub preferred_toc: EpubVersion,
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
    pub preferred_landmarks: EpubVersion,
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
    pub preferred_page_list: EpubVersion,
    /// Store **both** EPUB 2 and 3-specific information.
    ///
    /// Currently, this only pertains to the table of contents.
    ///
    /// If the epub contains both an EPUB2 `ncx` toc
    /// and EPUB3 `xhtml` toc, they will both be parsed and
    /// added to [`EpubToc`].
    ///
    /// Default: `false`
    pub store_all: bool,
    /// When set to `true`, ensures an EPUB conforms to the following:
    /// - Has an **identifier**.
    /// - Has a **title**.
    /// - Has a primary **language**.
    /// - Has a **version** where `2.0 <= version < 4.0`.
    /// - Has a **table of contents**
    /// - Elements (i.e., `item`, `itemref`) have their required attributes present.
    ///
    /// If any of the conditions are not met,
    /// an error will be returned.
    ///
    /// **This setting does not validate that an EPUB conforms entirely to the spec.
    /// However, it will refuse further processing if malformations are found.**
    ///
    /// Default: `true`
    pub strict: bool,
}

impl EpubSettings {
    /// Returns a builder to create an [`EpubSettings`] instance.
    pub fn builder() -> EpubSettingsBuilder {
        EpubSettingsBuilder(Self::default())
    }
}

impl Default for EpubSettings {
    fn default() -> Self {
        Self {
            preferred_toc: EpubVersion::EPUB3,
            preferred_landmarks: EpubVersion::EPUB3,
            preferred_page_list: EpubVersion::EPUB3,
            store_all: false,
            strict: true,
        }
    }
}

impl From<EpubSettingsBuilder> for EpubSettings {
    fn from(value: EpubSettingsBuilder) -> Self {
        value.build()
    }
}

/// Builder to construct an [`EpubSettings`] instance.
///
/// # Examples
/// - Passing a builder to open an [`Epub`] with:
/// ```
/// # use rbook::ebook::errors::EbookResult;
/// # use rbook::epub::EpubSettings;
/// # use rbook::epub::metadata::EpubVersion;
/// # use rbook::{Ebook, Epub};
/// # fn main() -> EbookResult<()> {
/// let epub = Epub::open_with(
///     "tests/ebooks/example_epub",
///     EpubSettings::builder()
///         .store_all(true)
///         .strict(false)
///         .preferred_landmarks(EpubVersion::EPUB2)
/// )?;
/// # Ok(())
/// # }
/// ```
#[derive(Debug, Clone)]
pub struct EpubSettingsBuilder(EpubSettings);

impl EpubSettingsBuilder {
    /// Turn this builder into an [`EpubSettings`] instance.
    pub fn build(self) -> EpubSettings {
        self.0
    }

    /// See [`EpubSettings::preferred_toc`].
    pub fn preferred_toc(mut self, version: EpubVersion) -> Self {
        self.0.preferred_toc = version;
        self
    }

    /// See [`EpubSettings::preferred_landmarks`].
    pub fn preferred_landmarks(mut self, version: EpubVersion) -> Self {
        self.0.preferred_landmarks = version;
        self
    }

    /// See [`EpubSettings::preferred_page_list`].
    pub fn preferred_page_list(mut self, version: EpubVersion) -> Self {
        self.0.preferred_landmarks = version;
        self
    }

    /// See [`EpubSettings::store_all`].
    pub fn store_all(mut self, store_all: bool) -> Self {
        self.0.store_all = store_all;
        self
    }

    /// See [`EpubSettings::strict`].
    pub fn strict(mut self, strict: bool) -> Self {
        self.0.strict = strict;
        self
    }
}

#[derive(Copy, Clone)]
pub(super) struct EpubResourceProvider<'ebook>(&'ebook Epub);

impl EpubResourceProvider<'_> {
    pub(crate) fn read_str(&self, resource: Resource) -> EbookResult<String> {
        self.0.read_resource_str(resource)
    }

    pub(crate) fn read_bytes(&self, resource: Resource) -> EbookResult<Vec<u8>> {
        self.0.read_resource_bytes(resource)
    }
}
