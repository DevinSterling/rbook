mod container;
mod package;
mod toc;

use crate::ebook::EbookResult;
use crate::ebook::archive::Archive;
use crate::ebook::epub::EpubConfig;
use crate::ebook::epub::consts;
use crate::ebook::epub::errors::EpubFormatError;
use crate::ebook::epub::manifest::EpubManifestData;
use crate::ebook::epub::metadata::{EpubMetadataData, EpubVersion};
use crate::ebook::epub::parser::package::TocLocation;
use crate::ebook::epub::spine::EpubSpineData;
use crate::ebook::epub::toc::EpubTocData;
use crate::ebook::errors::ArchiveError;
use crate::parser::ParserResult;
use crate::util::uri;

pub(super) struct ParsedContent {
    pub(super) package_file: String,
    pub(super) metadata: EpubMetadataData,
    pub(super) manifest: EpubManifestData,
    pub(super) spine: EpubSpineData,
    /// Encompasses landmarks & guide as well.
    pub(super) toc: EpubTocData,
}

/// Resolver to turn relative uris into absolute.
pub(super) struct UriResolver<'a>(
    /// The absolute path where relative paths are made absolute from.
    &'a str,
);

impl UriResolver<'_> {
    pub(super) fn resolve(&self, href: &str) -> String {
        uri::resolve(self.0, href).into_owned()
    }
}

pub(super) struct EpubParser<'a> {
    config: &'a EpubConfig,
    archive: &'a dyn Archive,
    version_hint: EpubVersion,
}

impl<'a> EpubParser<'a> {
    pub(super) fn new(settings: &'a EpubConfig, archive: &'a dyn Archive) -> Self {
        Self {
            config: settings,
            archive,
            // Irrelevant default: Overridden as soon as the package start element is parsed.
            version_hint: EpubVersion::EPUB3,
        }
    }

    pub(super) fn parse(&mut self) -> EbookResult<ParsedContent> {
        // Parse "META-INF/container.xml"
        let content_meta_inf = self.read_resource(consts::CONTAINER)?;

        let package_file = Self::parse_container(&content_meta_inf)?;
        // A resolver to turn uris within the <package> from relative to absolute
        let package_resolver = UriResolver(uri::parent(&package_file));

        // Parse "package.opf"
        let package_content = self.read_resource(package_file.as_str())?;
        let (toc_hrefs, metadata, manifest, spine, mut toc) =
            self.parse_opf(package_resolver, &package_content)?;

        // Parse "toc.xhtml/ncx"
        for TocLocation { href, version } in toc_hrefs {
            self.version_hint = version;
            // A resolver to turn uris within the toc file from relative to absolute
            let toc_resolver = UriResolver(uri::parent(&href));
            let content_toc = self.read_resource(href.as_str())?;
            toc.extend(self.parse_toc(&toc_resolver, &content_toc)?);
        }

        toc.set_preferences(self.config);

        Ok(ParsedContent {
            package_file,
            metadata,
            manifest,
            spine,
            toc,
        })
    }

    fn read_resource(&self, file: &str) -> Result<Vec<u8>, ArchiveError> {
        self.archive.read_resource_bytes_utf8(&file.into())
    }

    // Helper methods
    fn assert_required<T>(
        parent: Option<T>,
        if_missing: impl FnOnce() -> EpubFormatError,
    ) -> ParserResult<T> {
        parent.ok_or_else(|| if_missing().into())
    }

    fn assert_option<T: Default>(
        &self,
        option: Option<T>,
        error_message: &'static str,
    ) -> ParserResult<T> {
        if self.config.strict && option.is_none() {
            Err(EpubFormatError::MissingAttribute(String::from(error_message)).into())
        } else {
            Ok(option.unwrap_or_default())
        }
    }
}
