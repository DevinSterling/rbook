mod editor;
mod writer;

use crate::ebook::element::Href;
use crate::ebook::errors::EbookResult;
use crate::ebook::resource::Resource;
use crate::epub::Epub;
use crate::epub::archive::EpubArchive;
use crate::epub::consts::opf;
use crate::epub::manifest::{EpubManifestData, EpubManifestMut};
use crate::epub::metadata::{
    DetachedEpubMetaEntry, EpubMetadataData, EpubMetadataMut, EpubVersion,
};
use crate::epub::package::{EpubPackageData, EpubPackageMut};
use crate::epub::spine::{EpubSpineData, EpubSpineMut};
use crate::epub::toc::{EpubTocData, EpubTocMut};
use crate::epub::write::writer::{EpubWriteConfig, EpubWriter};
use crate::input::Many;
use crate::util::borrow::MaybeOwned;
use crate::util::sync::SendAndSync;
use crate::util::uri;
use std::fmt::Debug;
use std::io::{Cursor, Write};
use std::path::Path;

pub use editor::{EpubChapter, EpubEditor};

impl Epub {
    /// Creates a new [`Epub`].
    ///
    /// The returned instance defaults to **EPUB 3** and contains an empty
    /// [`EpubManifest`](super::manifest::EpubManifest),
    /// [`EpubSpine`](super::spine::EpubSpine), and
    /// [`EpubToc`](super::toc::EpubToc).
    ///
    /// For maximum compatibility with reading systems,
    /// the package directory is initially set to `OEBPS/package.opf`.
    ///
    /// [`EpubMetadata`](super::metadata::EpubMetadata) is initialized with a single
    /// `generator` entry identifying `rbook`, aiding in debugging if the produced file
    /// has issues. This can be overridden using [`EpubEditor::generator`] or cleared
    /// via [`EpubMetadataMut::remove_by_property`].
    ///
    /// # See Also
    /// - [`Epub::builder`] to create an [`Epub`] from a builder.
    pub fn new() -> Self {
        const RBOOK: &str = concat!("rbook v", env!("CARGO_PKG_VERSION"));

        let mut epub = Self {
            archive: EpubArchive::empty(),
            package: EpubPackageData::new("/OEBPS/package.opf".to_owned(), EpubVersion::EPUB3),
            metadata: EpubMetadataData::empty(),
            manifest: EpubManifestData::empty(),
            spine: EpubSpineData::empty(),
            toc: EpubTocData::empty(),
        };

        // For debugging, a generator is helpful to determine the source
        // in the case of produced anomalies, especially if distributed.
        epub.metadata_mut()
            .push(DetachedEpubMetaEntry::meta_name(opf::GENERATOR).value(RBOOK));

        epub
    }

    /// Returns a builder to construct a new [`Epub`].
    ///
    /// Unlike [`Epub::edit`], the returned editor owns the underlying data,
    /// allowing methods to chain and retrieve the resulting [`Epub`] via [`EpubEditor::build`].
    ///
    /// For maximum compatibility with reading systems,
    /// the package directory is initially set to `OEBPS/package.opf`.
    ///
    /// # See Also
    /// - [`Epub::new`] for details about the initial state when creating an [`Epub`].
    /// - [`EpubWriteOptions`], which is chainable via [`EpubEditor::write`].
    ///
    /// # Examples
    /// Creating a minimal EPUB:
    /// ```
    /// # use rbook::Epub;
    /// # const INTRO: &[u8] = &[];
    /// use rbook::epub::EpubChapter;
    ///
    /// let epub = Epub::builder()
    ///     .identifier("urn:isbn:9780000000001")
    ///     .title("My Book")
    ///     .author("Jane Doe")
    ///     .language("en")
    ///     .chapter(EpubChapter::new("Introduction").xhtml(INTRO))
    ///     .build(); // Returns the Epub
    /// ```
    /// Creating and writing to disk:
    /// ```no_run
    /// # use rbook::Epub;
    /// use rbook::ebook::metadata::TitleKind;
    /// use rbook::epub::EpubChapter;
    /// use rbook::epub::metadata::DetachedEpubMetaEntry;
    ///
    /// let write_result = Epub::builder()
    ///     .identifier("urn:doi:10.1234/abc")
    ///     .title([
    ///         DetachedEpubMetaEntry::title("Example EPUB")
    ///             .alternate_script("ja", "サンプルEPUB")
    ///             .kind(TitleKind::Main),
    ///         DetachedEpubMetaEntry::title("Example")
    ///             .alternate_script("ja", "サンプル")
    ///             .kind(TitleKind::Short),
    ///     ])
    ///     .creator(
    ///         DetachedEpubMetaEntry::creator("John Doe")
    ///             .file_as("Doe, John")
    ///             .alternate_script("ja", "山田太郎")
    ///             // Explicitly specifying the role as `illustrator`
    ///             .role("ill"),
    ///     )
    ///     .language("en")
    ///     .chapter(
    ///         EpubChapter::new("Chapter 1")
    ///             .href("c1.xhtml")
    ///             .xhtml_body("<h1>The Introduction</h1>"),
    ///     )
    ///     .write()
    ///     .compression(9)
    ///     .save("my.epub"); // Returns the write result
    /// ```
    pub fn builder() -> EpubEditor<'static> {
        EpubEditor::new()
    }

    /// Returns an [`EpubEditor`] to edit an [`Epub`].
    ///
    /// # See Also
    /// For lower-level modifications, access the underlying components via:
    /// - [`Self::package_mut`]
    /// - [`Self::metadata_mut`]
    /// - [`Self::manifest_mut`]
    /// - [`Self::spine_mut`]
    /// - [`Self::toc_mut`]
    pub fn edit(&mut self) -> EpubEditor<'_> {
        EpubEditor {
            epub: MaybeOwned::Borrowed(self),
        }
    }

    /// Advanced [`EpubPackage`](super::EpubPackage) modification.
    ///
    /// # See Also
    /// - [`Self::edit`] for simple modification tasks.
    pub fn package_mut(&mut self) -> EpubPackageMut<'_> {
        EpubPackageMut::new(&mut self.archive, &mut self.package)
    }

    /// Advanced [`EpubMetadata`](super::EpubMetadata) modification.
    ///
    /// # See Also
    /// - [`Self::edit`] for simple modification tasks.
    pub fn metadata_mut(&mut self) -> EpubMetadataMut<'_> {
        EpubMetadataMut::new(&mut self.package, &mut self.metadata)
    }

    /// Advanced [`EpubManifest`](super::EpubManifest) modification.
    ///
    /// # See Also
    /// - [`Self::edit`] for simple modification tasks.
    pub fn manifest_mut(&mut self) -> EpubManifestMut<'_> {
        EpubManifestMut::new(self)
    }

    /// Advanced [`EpubSpine`](super::EpubSpine) modification.
    ///
    /// # See Also
    /// - [`Self::edit`] for simple modification tasks.
    pub fn spine_mut(&mut self) -> EpubSpineMut<'_> {
        EpubSpineMut::new(self)
    }

    /// Advanced [`EpubToc`](super::EpubToc) modification.
    ///
    /// # See Also
    /// - [`Self::edit`] for simple modification tasks.
    pub fn toc_mut(&mut self) -> EpubTocMut<'_> {
        EpubTocMut::new(self)
    }

    /// Cleans up the content of an [`Epub`], removing broken references.
    ///
    /// It is recommended to call this method after performing multiple removals from the
    /// [manifest](Self::manifest_mut) to retain ebook structural integrity.
    ///
    /// # Manifest
    /// All manifest entries that reference a non-existent fallback or media overlay
    /// are updated to [`None`].
    ///
    /// # Spine
    /// All spine entries that reference an ID not present in the manifest
    /// ([`idref`](super::spine::EpubSpineEntry::idref)) are removed.
    ///
    /// # Table of Contents
    /// All ToC entries that reference a path not present in the manifest
    /// ([`href`](super::toc::EpubTocEntry::href)) are removed.
    ///
    /// Hrefs that contain a scheme (e.g., `http:`, `https:`, `mailto:`) are
    /// treated as external resources and are always retained.
    ///
    /// # Examples
    /// - Removing broken references:
    /// ```
    /// # use rbook::Epub;
    /// # fn main() -> rbook::ebook::errors::EbookResult<()> {
    /// let mut epub = Epub::open("tests/ebooks/example_epub")?;
    ///
    /// // Removing a manifest entry and removing orphaned references:
    /// epub.manifest_mut().remove_by_id("c1");
    /// epub.cleanup();
    ///
    /// // Removing a manifest entry without cleaning up:
    /// epub.manifest_mut().remove_by_id("c2");
    ///
    /// // Checking the spine:
    /// let spine = epub.spine();
    /// // `c1` was removed because `cleanup` was called
    /// assert_eq!(0, spine.by_idref("c1").count());
    /// // `c2` remains in the spine (broken link) because `cleanup` was not called again
    /// assert_eq!(1, spine.by_idref("c2").count());
    /// # Ok(())
    /// # }
    /// ```
    pub fn cleanup(&mut self) {
        // Cleanup Manifest
        self.manifest.remove_non_existent_references();

        // Cleanup Spine
        let manifest_entries = &self.manifest.entries;
        self.spine
            .entries
            .retain(|entry| manifest_entries.contains_key(&entry.idref));

        // Cleanup ToC
        // Allocate a hashset for efficient path lookup.
        let hrefs = manifest_entries
            .iter()
            .map(|(_, entry)| entry.href.as_str())
            .collect::<std::collections::HashSet<_>>();

        // Remove ToC entries that reference a non-existent path
        self.toc.recursive_retain(|entry| {
            match &entry.href {
                Some(href) if !uri::has_scheme(href) => hrefs.contains(uri::path(href)),
                // External links (e.g., http://) are always preserved
                _ => true,
            }
        });
    }

    /// Returns configuration to write an [`Epub`] to a destination.
    #[must_use]
    pub fn write(&self) -> EpubWriteOptions<&Self> {
        EpubWriteOptions::<&Self>::new(self)
    }
}

impl Default for Epub {
    fn default() -> Self {
        Self::new()
    }
}

/// Configuration to write an [`Epub`] to a destination.
///
/// `EpubWriteOptions` supports two usage patterns:
/// 1. **Attached**:
///    Created via [`Epub::write`] or [`EpubEditor::write`].
///    The options are bound to a specific [`Epub`].
/// 2. **Detached**:
///    Created via [`EpubWriteOptions::default`].
///    The options are standalone and terminal methods take a reference to an [`Epub`],
///    allowing configuration reuse.
///
/// # Renditions
/// Writing multi-rendition EPUBs is not currently supported.
/// This is a feature that will be introduced in the future.
///
/// If the source EPUB contains multiple renditions
/// (multiple `rootfile` entries in `META-INF/container.xml`),
/// **only the currently loaded rendition in [`Epub`] is preserved**.
///
/// The `container.xml` file is recreated to reference only the loaded rendition,
/// and resources specific to other renditions may be removed depending on [`Self::keep_orphans`].
///
/// # Options
/// ## Output format
/// - [`target`](Self::target) (Default: [`EpubVersion::Epub2`])
/// - [`compression`](Self::compression) (Default: `6`)
/// ## Cleanup
/// - [`keep_orphans`](Self::keep_orphans) (Default: Retain files in `META-INF`)
/// ## Table of Contents
/// - [`generate_toc`](Self::generate_toc) (Default: `true`)
/// - [`toc_stylesheet`](Self::toc_stylesheet) (Default: Preserve existing)
///
/// # Examples
/// - One-off write (Attached):
/// ```no_run
/// # use rbook::Epub;
/// # fn main() -> rbook::ebook::errors::EbookResult<()> {
/// Epub::builder()
///     .author("Jane Doe")
///     // ...Populate epub... //
///     .write()
///     .compression(9)
///     .save("path/to/destination.epub")?;
/// # Ok(())
/// # }
/// ```
/// - Batch writes (Detached):
/// ```no_run
/// # use rbook::Epub;
/// # use rbook::epub::EpubWriteOptions;
/// # fn main() -> rbook::ebook::errors::EbookResult<()> {
/// # let epubs = Vec::new();
/// let mut write_options = EpubWriteOptions::default();
///
/// write_options
///     .compression(0) // No compression for speed
///     .keep_orphans(true);
///
/// for (i, epub) in epubs.iter().enumerate() {
///     // Reuse options for every epub
///     write_options.save(epub, format!("book_{i}.epub"))?;
/// }
/// # Ok(())
/// # }
/// ```
#[derive(Clone, Debug)]
pub struct EpubWriteOptions<T = ()> {
    container: T,
    config: EpubWriteConfig,
}

impl<T> EpubWriteOptions<T> {
    fn save_epub(epub: &Epub, config: &EpubWriteConfig, path: &Path) -> EbookResult<()> {
        const TEMP: &str = "rbook.tmp";

        let temp = path.with_extension(TEMP);

        let write_result = (|| {
            let file = std::fs::File::create(&temp)?;
            let mut buf = std::io::BufWriter::new(file);

            Self::write_epub(epub, config, &mut buf)?;

            // Write the remaining bytes to file
            buf.flush()?;
            std::fs::rename(&temp, path)?;
            Ok(())
        })();

        if let Err(error) = write_result {
            // Attempt to remove the temp file
            let _ = std::fs::remove_file(&temp);
            // Original error takes precedence
            return Err(error);
        }
        Ok(())
    }

    fn write_epub<W: Write>(epub: &Epub, config: &EpubWriteConfig, write_to: W) -> EbookResult<W> {
        EpubWriter::new(config, epub, write_to).write()
    }

    fn vec_epub(epub: &Epub, config: &EpubWriteConfig) -> EbookResult<Vec<u8>> {
        let cursor = Cursor::new(Vec::new());

        Self::write_epub(epub, config, cursor).map(Cursor::into_inner)
    }

    // NOTE: strict mode for writing will become a feature in the future.
    // fn strict(&mut self, strict: bool) -> &mut Self {
    //     self.config.strict = strict;
    //     self
    // }

    /// Sets the compatibility target versions,
    /// controlling which version-specific data is generated during writing.
    ///
    /// **This is primarily useful for EPUB 2 backwards-compatibility.**
    ///
    /// If the target is [`None`] or an empty collection (`[]`),
    /// no specialized post-processing shown here is performed when writing.
    ///
    /// # Targeting EPUB 2
    /// When EPUB 2 is targeted for backwards-compatibility,
    /// legacy data (e.g., NCX, Guide, Metadata entries)
    /// are generated to make an EPUB 3 file readable on older devices.
    ///
    /// **Note**: If the [`Epub`] version is set to [`EpubVersion::Epub2`],
    /// EPUB 2 is always targeted, regardless of this option.
    ///
    /// ## 1. Metadata
    /// - The `xmlns:opf` namespace declaration is added to the `<metadata>` element.
    /// - If absent, a legacy `cover` metadata entry is generated if an EPUB 3 cover image
    ///   entry exists in the manifest (e.g., `<meta name="cover" content="image-id">`).
    ///
    /// ## 2. EPUB 3 Refinements
    /// EPUB 3 refinements are downgraded into their legacy
    /// [EPUB 2 metadata attribute](https://idpf.org/epub/20/spec/OPF_2.0_final_spec.html#AppendixA)
    /// counterparts if not already present.
    ///
    /// **Note**: This downgrade is only performed if the [`Epub`] version is set to
    /// [`EpubVersion::Epub2`].
    /// Generating legacy attributes for EPUB 3 results in EpubCheck validation errors.
    ///
    /// | Metadata Entry Property  | Refinement Property | Mapped to legacy attribute |
    /// |--------------------------|---------------------|----------------------------|
    /// | `dc:identifier`          | `identifier-type`   | `opf:scheme`               |
    /// | `dc:creator/contributor` | `role`              | `opf:role`                 |
    /// | `dc:creator/contributor` | `file-as`           | `opf:file-as`              |
    ///
    /// ## 3. Spine
    /// The `toc` attribute is added to the `<spine>` element, linking it to the NCX file.
    ///
    /// ## 4. Guide & Navigation
    /// If [`Self::generate_toc`] is `true`:
    /// - **Guide**: A legacy `<guide>` element is generated.
    /// - **NCX**: A legacy `toc.ncx` file is generated.
    ///
    /// # Examples
    /// - Specifying the target:
    /// ```
    /// # use rbook::epub::EpubWriteOptions;
    /// use rbook::epub::metadata::EpubVersion;
    ///
    /// EpubWriteOptions::default()
    ///     // Target nothing
    ///     // - Useful for generating non-backwards-compatible EPUB 3 ebooks
    ///     .target(None)
    ///     // Target EPUB 2
    ///     // - Default; generates backwards-compatible EPUB 3 ebooks
    ///     .target(EpubVersion::EPUB2)
    ///     // Target both EPUB 2 and 3
    ///     .target([EpubVersion::EPUB2, EpubVersion::EPUB3]);
    /// ```
    ///
    /// Default: [`EpubVersion::Epub2`]
    pub fn target(&mut self, target: impl Many<EpubVersion>) -> &mut Self {
        // Clear all previous targets
        self.config.targets.clear();

        for version in target.iter_many() {
            self.config.targets.add(version);
        }

        self
    }

    /// When set to `true`, regenerates the table of contents.
    ///
    /// When generating the table of contents files (`*.xhtml` and `*.ncx`),
    /// if either the EPUB 2 or EPUB 3 variant is absent,
    /// it is automatically generated using the data from the other.
    ///
    /// For example, when generating a backwards-compatible EPUB 3 ebook,
    /// if the EPUB 3 `nav` page list is absent, it is derived automatically
    /// from the EPUB 2 `ncx` page list if present.
    /// (This behavior is *similar* to [`EpubTocMut::by_kind_mut`])
    ///
    /// If set to `false`, existing ToC files are preserved exactly as they are
    /// in the source archive, which may result in broken links if other
    /// resources were moved or renamed.
    ///
    /// Default: `true`
    pub fn generate_toc(&mut self, generate: bool) -> &mut Self {
        self.config.generate_toc = generate;
        self
    }

    /// Sets the stylesheets for the generated Table of Contents via the [`Many`] trait.
    ///
    /// This method replaces any previously set ToC stylesheets. The provided
    /// locations must match the `href` of CSS resources added to the manifest
    /// (e.g., [`EpubEditor::resource`]).
    /// Unknown locations are ignored.
    ///
    /// If this method is not called, any existing stylesheet links
    /// found in the original `xhtml` ToC file will be preserved.
    ///
    /// # Note
    /// - For EPUB 3, links are added to the generated `xhtml` ToC file.
    /// - For EPUB 2, this has no effect as NCX does not support CSS.
    /// - Stylesheets are only linked if [`Self::generate_toc`] is `true`.
    ///
    /// # See Also
    /// - [`EpubEditor::resource`] for path details.
    ///   The same path resolution rules apply to this method for the given stylesheet locations.
    ///
    /// # Examples
    /// - Setting the toc stylesheet:
    /// ```
    /// # use rbook::Epub;
    /// # use std::path::PathBuf;
    /// # const TOC_CSS: &[u8] = &[];
    /// # const XYZ_CSS: &[u8] = &[];
    /// Epub::builder()
    ///     // ...Add Metadata
    ///     // ...Add Content
    ///     // ...Add resources
    ///     .resource(("toc.css", PathBuf::from("local/file/on/disk/toc.css")))
    ///     .resource(("clear.css", "ol { list-style: none; }"))
    ///     .write()
    ///     // Sets the generated ToC to contain no linked stylesheets
    ///     .toc_stylesheet(None)
    ///     .toc_stylesheet("stylesheet.css")
    ///     // Only the last call takes effect
    ///     .toc_stylesheet(["toc.css", "clear.css"]);
    /// ```
    /// - Using every stylesheet present in the manifest of an [`Epub`]:
    /// ```no_run
    /// # use rbook::Epub;
    /// use rbook::input::Batch;
    ///
    /// # fn main() -> rbook::ebook::errors::EbookResult<()> {
    /// let mut epub = Epub::new();
    /// // ...Add Metadata
    /// // ...Add Content
    /// // ...Add resources
    /// let styles = epub.manifest().styles();
    ///
    /// epub.write()
    ///     .toc_stylesheet(Batch(styles))
    ///     .save("new.epub")
    /// # }
    /// ```
    pub fn toc_stylesheet<'a>(&mut self, location: impl Many<Resource<'a>>) -> &mut Self {
        self.config.generated_toc_stylesheets = Some(
            location
                .iter_many()
                .filter_map(|resource| resource.key().value().map(str::to_owned))
                .collect(),
        );
        self
    }

    /// Sets the *deflate* compression level of the generated epub file.
    ///
    /// The given compression level must be within the range `[0, 9]`.
    /// If the level is greater than the maximum bound, it is set to `9`.
    ///
    /// - **Lower value**: Faster compression → Larger file size
    /// - **Higher value**: Slower compression → Smaller file size
    ///
    /// A compression level of `0` equates to no compression.
    ///
    /// Default: `6` (Balance of speed + compression)
    pub fn compression(&mut self, level: u8) -> &mut Self {
        self.config.compression = level.min(9);
        self
    }

    /// When set to `true`, retains all files not referenced in the
    /// [`EpubManifest`](super::manifest::EpubManifest), including discarded resources.
    /// If set to `false`, only files specified in the manifest are retained.
    ///
    /// # Note
    /// Resources added via [`EpubEditor::container_resource`] are always retained
    /// regardless of this option.
    ///
    /// # Examples
    /// ```
    /// # use rbook::epub::EpubWriteOptions;
    /// use rbook::ebook::element::Href;
    ///
    /// EpubWriteOptions::default()
    ///     // Retain all orphaned resources
    ///     .keep_orphans(true)
    ///     // Discard all orphaned resources
    ///     .keep_orphans(false)
    ///     // Retain resources within META-INF (Default)
    ///     .keep_orphans(|file: Href| file.as_str().starts_with("/META-INF/"))
    ///     // Retain orphaned JSON resources
    ///     .keep_orphans(|file: Href| file.extension() == Some("json"));
    /// ```
    ///
    /// Default: Retain files within `/META-INF/`.
    pub fn keep_orphans(&mut self, keep: impl OrphanFilter + 'static) -> &mut Self {
        self.config.keep_orphans = Some(std::sync::Arc::new(keep));
        self
    }
}

impl<'ebook> EpubWriteOptions<&'ebook Epub> {
    fn new(epub: &'ebook Epub) -> Self {
        Self {
            container: epub,
            config: EpubWriteConfig::default(),
        }
    }

    /// Saves an [`Epub`] to disk using the given `path`.
    pub fn save(&self, path: impl AsRef<Path>) -> EbookResult<()> {
        Self::save_epub(self.container, &self.config, path.as_ref())
    }

    /// Writes an [`Epub`] to the given `writer`.
    pub fn write<W: Write>(&self, writer: W) -> EbookResult<W> {
        Self::write_epub(self.container, &self.config, writer)
    }

    /// Generates an [`Epub`] as a byte [`Vec`].
    pub fn to_vec(&self) -> EbookResult<Vec<u8>> {
        Self::vec_epub(self.container, &self.config)
    }
}

impl EpubWriteOptions {
    /// Saves the provided [`Epub`] to disk using the given `path`.
    ///
    /// If options are one-off, prefer [`EpubWriteOptions::<&Epub>::save`].
    pub fn save(&self, epub: &Epub, path: impl AsRef<Path>) -> EbookResult<()> {
        Self::save_epub(epub, &self.config, path.as_ref())
    }

    /// Writes the provided [`Epub`] to the given `writer`.
    ///
    /// If options are one-off, prefer [`EpubWriteOptions::<&Epub>::write`].
    pub fn write<W: Write>(&self, epub: &Epub, write_to: W) -> EbookResult<W> {
        Self::write_epub(epub, &self.config, write_to)
    }

    /// Generates a byte [`Vec`] for the provided [`Epub`].
    ///
    /// If options are one-off, prefer [`EpubWriteOptions::<&Epub>::to_vec`].
    pub fn to_vec(&self, epub: &Epub) -> EbookResult<Vec<u8>> {
        Self::vec_epub(epub, &self.config)
    }
}

impl Default for EpubWriteOptions {
    fn default() -> Self {
        Self {
            container: (),
            config: EpubWriteConfig::default(),
        }
    }
}

/// A filter to determine if a file not present in the
/// [`EpubManifest`](super::manifest::EpubManifest) should be kept.
///
/// See [`EpubWriteOptions::keep_orphans`] for more details.
pub trait OrphanFilter: SendAndSync {
    /// Returns `true` if the orphaned file should be kept in the archive.
    ///
    /// The given `path` is the percent-decoded absolute location of the file within the EPUB
    /// container (e.g., `/EPUB/unused image.jpg`).
    fn filter(&self, path: Href<'_>) -> bool;
}

impl OrphanFilter for bool {
    fn filter(&self, _href: Href<'_>) -> bool {
        *self
    }
}

impl<F: Fn(Href<'_>) -> bool + SendAndSync + 'static> OrphanFilter for F {
    fn filter(&self, href: Href<'_>) -> bool {
        self(href)
    }
}
