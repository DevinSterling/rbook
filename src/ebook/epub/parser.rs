mod container;
mod package;
mod toc;

use crate::ebook::EbookResult;
use crate::ebook::archive::Archive;
use crate::ebook::archive::errors::ArchiveResult;
use crate::ebook::epub::consts::ocf;
use crate::ebook::epub::errors::EpubError;
use crate::ebook::epub::manifest::EpubManifestData;
use crate::ebook::epub::metadata::{EpubMetadataData, EpubVersion};
use crate::ebook::epub::package::EpubPackageData;
use crate::ebook::epub::spine::EpubSpineData;
use crate::ebook::epub::toc::EpubTocData;
use crate::parser::ParserResult;
use crate::util::borrow::CowExt;
use crate::util::uri;
use std::borrow::Cow;

#[derive(Clone, Debug)]
pub(crate) struct EpubParseConfig {
    /// See [`super::EpubOpenOptions::preferred_toc`].
    pub(crate) preferred_toc: EpubVersion,
    /// See [`super::EpubOpenOptions::retain_variants`].
    pub(crate) retain_variants: bool,
    /// See [`super::EpubOpenOptions::strict`].
    pub(crate) strict: bool,
    /// See [`super::EpubOpenOptions::skip_metadata`]; inverted.
    pub(crate) parse_metadata: bool,
    /// See [`super::EpubOpenOptions::skip_manifest`]; inverted.
    pub(crate) parse_manifest: bool,
    /// See [`super::EpubOpenOptions::skip_spine`]; inverted.
    pub(crate) parse_spine: bool,
    /// See [`super::EpubOpenOptions::skip_toc`]; inverted.
    pub(crate) parse_toc: bool,
}

impl Default for EpubParseConfig {
    fn default() -> Self {
        Self {
            preferred_toc: EpubVersion::EPUB3,
            retain_variants: false,
            strict: false,
            parse_metadata: true,
            parse_manifest: true,
            parse_spine: true,
            parse_toc: true,
        }
    }
}

/// Utility trait to perform validation on sub-parsers.
trait EpubParserValidator {
    fn config(&self) -> &EpubParseConfig;

    fn is_strict(&self) -> bool {
        self.config().strict
    }

    fn mandatory<T>(
        &self,
        parent: Option<T>,
        if_missing: impl FnOnce() -> EpubError,
    ) -> ParserResult<T> {
        parent.ok_or_else(|| if_missing().into())
    }

    /// Required attribute value.
    ///
    /// If `value` is [`None`],
    /// its [`Default`] is returned if `strict` mode is disabled.
    /// Otherwise, an error is returned.
    fn require_attribute<T: Default>(
        &self,
        value: Option<T>,
        error_message: &'static str,
    ) -> ParserResult<T> {
        if self.is_strict() && value.is_none() {
            Err(EpubError::MissingAttribute(error_message.to_owned()).into())
        } else {
            Ok(value.unwrap_or_default())
        }
    }

    fn require_href(&self, href: String) -> ParserResult<String> {
        let encoded = uri::encode(&href);
        if self.is_strict() && matches!(encoded, Cow::Owned(_)) {
            Err(EpubError::UnencodedHref(href).into())
        } else {
            Ok(encoded.take_owned().unwrap_or(href))
        }
    }
}

pub(super) struct ParsedComponents {
    pub(super) package: EpubPackageData,
    pub(super) metadata: EpubMetadataData,
    pub(super) manifest: EpubManifestData,
    pub(super) spine: EpubSpineData,
    /// Encompasses landmarks & guide as well.
    pub(super) toc: EpubTocData,
}

/// The context shared among all EPUB-related parsers.
///
/// Currently consists of a single [`EpubParseConfig`] field.
#[derive(Copy, Clone)]
pub(super) struct EpubParserContext<'a> {
    config: &'a EpubParseConfig,
    version: EpubVersion,
}

impl EpubParserValidator for EpubParserContext<'_> {
    fn config(&self) -> &EpubParseConfig {
        self.config
    }
}

pub(super) struct EpubParser<'a> {
    ctx: EpubParserContext<'a>,
    archive: &'a dyn Archive,
}

impl<'a> EpubParser<'a> {
    pub(super) fn new(config: &'a EpubParseConfig, archive: &'a dyn Archive) -> Self {
        Self {
            ctx: EpubParserContext {
                config,
                // Default: Overridden as soon as the package start element is parsed.
                version: EpubVersion::EPUB3,
            },
            archive,
        }
    }

    pub(super) fn parse(mut self) -> EbookResult<ParsedComponents> {
        // Parse "META-INF/container.xml"
        let content_meta_inf = self.read_resource(ocf::CONTAINER_PATH)?;
        let package_file = self.parse_container(&content_meta_inf)?;

        // Parse "package.opf"
        let package_content = self.read_resource(&package_file)?;
        let opf = self.parse_package(&package_content, package_file)?;
        let mut toc = opf.guide;

        // Parse "toc.xhtml/ncx"
        self.parse_tocs(opf.toc_locations, &mut toc)?;

        Ok(ParsedComponents {
            package: opf.package,
            metadata: opf.metadata,
            manifest: opf.manifest,
            spine: opf.spine,
            toc,
        })
    }

    fn read_resource(&self, file: &str) -> ArchiveResult<Vec<u8>> {
        self.archive.read_resource_as_utf8_bytes(&file.into())
    }
}

impl EpubParserValidator for EpubParser<'_> {
    fn config(&self) -> &EpubParseConfig {
        self.ctx.config
    }
}
