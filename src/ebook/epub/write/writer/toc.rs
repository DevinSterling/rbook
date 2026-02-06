mod ncx;
mod xhtml;

use crate::ebook::epub::Epub;
use crate::ebook::epub::consts::opf;
use crate::ebook::epub::manifest::EpubManifestEntryData;
use crate::ebook::epub::metadata::EpubVersion;
use crate::ebook::epub::toc::EpubTocEntryData;
use crate::ebook::epub::write::writer::toc::ncx::NcxTocWriter;
use crate::ebook::epub::write::writer::toc::xhtml::XhtmlTocWriter;
use crate::ebook::epub::write::writer::{EpubWriteConfig, EpubWriter, EpubWriterContext};
use crate::ebook::resource::consts::mime;
use crate::ebook::toc::TocEntryKind;
use crate::util::uri;
use crate::writer::WriterResult;
use std::borrow::Cow;
use std::io::Write;

/// If EPUB 2 NCX-specific ToC roots are not found,
/// fallback to EPUB 3-specific ToC root entries when generating the NCX document.
pub(super) const EPUB2_TOC_FALLBACKS: [EpubVersion; 2] = [EpubVersion::EPUB2, EpubVersion::EPUB3];

/// If EPUB 3-specific ToC roots are not found,
/// fallback to EPUB 2 NCX-specific ToC root entries when generating the nav document.
const EPUB3_TOC_FALLBACKS: [EpubVersion; 2] = [EpubVersion::EPUB3, EpubVersion::EPUB2];

/// Possess the ToC resource location and manifest entry ID.
pub(super) struct TocData<'a> {
    /// Indicates whether this data (`id` and `location`)
    /// was absent, then generated during writing.
    ///
    /// - `true`: Generated within [`TocContext::resolve_toc_data`].
    /// - `false`: `id` and `location` is specified prior to writing
    ///   (e.g., provided explicitly from a user).
    ///
    /// Note that `is_generated` has no effect on [`EpubWriteConfig::generate_toc`];
    /// they are separate.
    pub(super) is_generated: bool,
    /// Manifest item `id`.
    pub(super) id: Cow<'a, str>,
    /// The ***absolute percent-encoded*** location of the ToC resource.
    pub(super) location: Cow<'a, str>,
}

/// Holds ToC-related data important during writing, indicating
/// if the associated content must be generated dynamically:
/// - EPUB 2 `ncx` document
/// - EPUB 3 `xhtml` ("nav") document
pub(super) struct TocContext<'a> {
    pub(super) epub2_ncx: Option<TocData<'a>>,
    pub(super) epub3_nav: Option<TocData<'a>>,
}

impl<'a> TocContext<'a> {
    pub(super) fn generate(config: &EpubWriteConfig, epub: &'a Epub) -> Self {
        const NCX_ID: &str = "ncx";
        const NCX_FILE: &str = "toc.ncx";
        const XHTML_ID: &str = "nav";
        const XHTML_FILE: &str = "toc.xhtml";

        let version = epub.package.version.parsed;
        let mut epub2_ncx = None;
        let mut epub3_nav = None;

        // Retrieve ncx toc manifest id/href
        if version.is_epub2() || config.targets.epub2 {
            epub2_ncx = Self::resolve_toc_data(config, epub, NCX_ID, NCX_FILE, |entry| {
                entry.media_type == mime::NCX
            });
        }
        // Retrieve xhtml toc manifest id/href
        if version.is_epub3() || config.targets.epub3 {
            epub3_nav = Self::resolve_toc_data(config, epub, XHTML_ID, XHTML_FILE, |entry| {
                entry.properties.has_property(opf::NAV_PROPERTY)
            });
        }

        TocContext {
            epub2_ncx,
            epub3_nav,
        }
    }

    fn resolve_toc_data(
        config: &EpubWriteConfig,
        epub: &'a Epub,
        default_toc_id: &'static str,
        default_toc_href: &'static str,
        predicate: fn(&EpubManifestEntryData) -> bool,
    ) -> Option<TocData<'a>> {
        let manifest = &epub.manifest;
        let data = manifest
            .entries
            .iter()
            .find(|(_, entry)| predicate(entry))
            .map(|(id, entry)| TocData {
                is_generated: false,
                id: Cow::Borrowed(id.as_str()),
                location: Cow::Borrowed(entry.href.as_str()),
            });

        // If the toc entry exists already,
        // there's no need to generate a new manifest entry
        if let Some(data) = data {
            return Some(data);
        }
        // If toc manifest entry generation is disabled, return None
        else if !config.generate_toc {
            return None;
        }

        // Generate new toc manifest entries
        // - Avoid conflicting id/href
        let id = manifest.generate_unique_id(default_toc_id.to_owned());
        let href = manifest.generate_unique_href(uri::join(
            epub.package().directory().as_str(),
            default_toc_href,
        ));

        Some(TocData {
            is_generated: true,
            id: Cow::Owned(id),
            location: Cow::Owned(href),
        })
    }
}

impl<W: Write> EpubWriter<'_, W> {
    pub(super) fn write_toc(&mut self) -> WriterResult<()> {
        // No need to write/generate the ToC if explicitly disabled
        if !self.ctx.config.generate_toc {
            return Ok(());
        }
        if let Some(ncx) = &self.ctx.toc.epub2_ncx {
            // The `location` must be decoded as it's percent-encoded
            self.zip.start_file(&uri::decode(&ncx.location))?;
            NcxTocWriter::new(&self.ctx, ncx, &mut self.zip).write_ncx()?;
        }
        if let Some(nav) = &self.ctx.toc.epub3_nav {
            self.zip.start_file(&uri::decode(&nav.location))?;
            XhtmlTocWriter::new(&self.ctx, nav, &mut self.zip).write_xhtml()?;
        }
        Ok(())
    }
}

pub(super) fn get_toc_root<'ebook>(
    ctx: &'ebook EpubWriterContext,
    kind: TocEntryKind<'_>,
    versions: impl IntoIterator<Item = EpubVersion>,
) -> Option<&'ebook EpubTocEntryData> {
    let toc = &ctx.epub.toc.entries;
    let kind = kind.as_str();

    for version in versions.into_iter() {
        if let Some(root) = toc.get(&(kind, version)) {
            return Some(root);
        }
    }
    None
}
