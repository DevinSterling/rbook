mod guide;
mod manifest;
mod metadata;
mod spine;

use crate::ebook::element::TextDirection;
use crate::ebook::epub::consts::{opf, opf::bytes, xml};
use crate::ebook::epub::errors::EpubError;
use crate::ebook::epub::manifest::EpubManifestData;
use crate::ebook::epub::metadata::{EpubMetadataData, EpubVersion};
use crate::ebook::epub::package::{EpubPackageData, EpubVersionData, Prefix, Prefixes};
use crate::ebook::epub::parser::package::metadata::PendingRefinements;
use crate::ebook::epub::parser::package::spine::TempEpubSpine;
use crate::ebook::epub::parser::toc::TocLocation;
use crate::ebook::epub::parser::{
    EpubParseConfig, EpubParser, EpubParserContext, EpubParserValidator,
};
use crate::ebook::epub::spine::EpubSpineData;
use crate::ebook::epub::toc::EpubTocData;
use crate::ebook::metadata::Version;
use crate::parser::ParserResult;
use crate::parser::xml::{XmlEvent, XmlReader, XmlStartElement};
use crate::util::uri::UriResolver;

pub(super) struct ProcessedPackageData {
    pub(super) toc_locations: Vec<TocLocation>,
    pub(super) package: EpubPackageData,
    pub(super) metadata: EpubMetadataData,
    pub(super) manifest: EpubManifestData,
    pub(super) spine: EpubSpineData,
    pub(super) guide: EpubTocData,
}

struct PackageParser<'parser, 'a> {
    ctx: &'parser mut EpubParserContext<'a>,
    reader: XmlReader<'parser>,
    resolver: UriResolver<'parser>,
    package: Option<EpubPackageData>,
    metadata: Option<EpubMetadataData>,
    manifest: Option<EpubManifestData>,
    spine: Option<TempEpubSpine>,
    guide: Option<EpubTocData>,
    refinements: PendingRefinements,
}

impl<'parser, 'a> PackageParser<'parser, 'a> {
    fn new(
        ctx: &'parser mut EpubParserContext<'a>,
        data: &'parser [u8],
        location: &'parser str,
    ) -> Self {
        Self {
            reader: XmlReader::from_bytes(ctx.is_strict(), data),
            resolver: UriResolver::parent_of(location),
            refinements: PendingRefinements::new(),
            package: None,
            metadata: None,
            manifest: None,
            spine: None,
            guide: None,
            ctx,
        }
    }

    /// Parses the epub `.opf` file and returns all
    /// necessary data required for further processing.
    pub(super) fn parse_opf(mut self) -> ParserResult<ProcessedPackageData> {
        self.handle_opf()?;

        Ok(ProcessedPackageData {
            // NOTE: `get_toc_hrefs` must be called first as it requires a lookup
            //       into the `manifest` and `spine` fields of `PackageParser`.
            toc_locations: self.get_toc_hrefs()?,
            package: self.take_package()?,
            metadata: self.take_metadata()?,
            manifest: self.take_manifest()?,
            spine: self.take_spine()?,
            guide: self.guide.take().unwrap_or_else(EpubTocData::empty),
        })
    }

    fn take_package(&mut self) -> ParserResult<EpubPackageData> {
        let package = self.package.take();

        // Not possible to parse an EPUB without a package element
        self.mandatory(package, || EpubError::NoPackageFound)
    }

    fn take_metadata(&mut self) -> ParserResult<EpubMetadataData> {
        let metadata = self.metadata.take();

        self.resolve_section(
            self.config().parse_metadata,
            metadata,
            || EpubError::NoMetadataFound,
            EpubMetadataData::empty,
        )
    }

    fn take_manifest(&mut self) -> ParserResult<EpubManifestData> {
        let manifest = self.manifest.take();

        self.resolve_section(
            self.config().parse_manifest,
            manifest,
            || EpubError::NoManifestFound,
            EpubManifestData::empty,
        )
    }

    fn take_spine(&mut self) -> ParserResult<EpubSpineData> {
        let spine = self.spine.take();

        self.resolve_section(
            self.config().parse_spine,
            spine.map(|temp_spine| temp_spine.data),
            || EpubError::NoSpineFound,
            EpubSpineData::empty,
        )
    }

    fn resolve_section<T>(
        &self,
        should_parse: bool,
        option_data: Option<T>,
        error: fn() -> EpubError,
        default: fn() -> T,
    ) -> ParserResult<T> {
        if should_parse {
            // If parsing should happen, check if the parsed item exists
            if let Some(data) = option_data {
                return Ok(data);
            } else if self.is_strict() {
                return Err(error().into());
            }
        }
        // if not parsed (skipped) or not strict (lenient), use the default
        Ok(default())
    }

    fn handle_opf(&mut self) -> ParserResult<()> {
        while let Some(event) = self.reader.next() {
            let XmlEvent::Start(el) = event? else {
                continue;
            };
            match el.local_name() {
                bytes::PACKAGE => {
                    let package = self.parse_package(&el)?;
                    self.package.replace(package);
                }
                bytes::METADATA if self.config().parse_metadata => {
                    let metadata = self.parse_metadata()?;
                    self.metadata.replace(metadata);
                }
                bytes::MANIFEST if self.config().parse_manifest => {
                    let manifest = self.parse_manifest()?;
                    self.manifest.replace(manifest);
                }
                bytes::SPINE if self.config().parse_spine => {
                    let spine = self.parse_spine(&el)?;
                    self.spine.replace(spine);
                }
                // "toc"-related due to its navigational aspect.
                bytes::GUIDE if self.config().parse_toc => {
                    let guide = self.parse_guide(&el)?;
                    self.guide.replace(guide);
                }
                _ => {}
            }
        }
        Ok(())
    }

    fn parse_package(&mut self, package: &XmlStartElement) -> ParserResult<EpubPackageData> {
        let mut attributes = package.attributes()?;

        // Required attributes
        let raw_version =
            self.require_attribute(attributes.remove(opf::VERSION)?, "package[*version]")?;
        let unique_identifier = self.require_attribute(
            attributes.remove(opf::UNIQUE_ID)?,
            "package[*unique-identifier]",
        )?;
        let version = self.handle_epub_version(raw_version)?;

        // Optional attributes
        let language = attributes.remove(xml::LANG)?;
        let prefix = attributes.remove(opf::PREFIX)?;
        let text_direction = attributes
            .remove(opf::TEXT_DIR)?
            .map_or(TextDirection::Auto, TextDirection::from);

        Ok(EpubPackageData {
            attributes: attributes.try_into()?,
            prefixes: self.parse_prefix(prefix)?,
            location: String::new(), // Temporary placeholder
            version,
            unique_identifier,
            language,
            text_direction,
        })
    }

    fn handle_epub_version(&mut self, raw: String) -> ParserResult<EpubVersionData> {
        let parsed = EpubVersion::from(match Version::from_str(&raw) {
            Some(version) => version,
            None if !self.is_strict() => Version(0, 0),
            _ => return Err(EpubError::InvalidVersion(raw).into()),
        });

        // Update context version
        self.ctx.version = parsed;

        Ok(EpubVersionData { raw, parsed })
    }

    fn parse_prefix(&mut self, raw: Option<String>) -> ParserResult<Prefixes> {
        let Some(raw) = raw else {
            return Ok(Prefixes::EMPTY);
        };

        let mut prefixes = Vec::new();
        let mut iter = raw.split_ascii_whitespace();

        // 1: A prefix must be immediately followed by a colon character (:)
        // 2: A prefix must be separated by its URI with a space.
        while let Some(prefix) = iter.next() {
            // Split the colon
            // - Also, an epub may not have a prefix spaced properly
            let (name, mut uri) = match prefix.split_once(':') {
                Some((name, uri)) => Ok((name, uri)),
                None if self.is_strict() => Err(EpubError::InvalidPrefix(prefix.to_owned())),
                None => continue,
            }?;

            // If the token ended with a colon, the URI must be the next whitespace-separated token
            if uri.is_empty() {
                uri = match iter.next() {
                    Some(uri) => Ok(uri),
                    None if self.is_strict() => Err(EpubError::InvalidPrefix(prefix.to_owned())),
                    None => continue,
                }?;
            }

            // A prefix should have a name
            if self.is_strict() && name.is_empty() {
                return Err(EpubError::InvalidPrefix(prefix.to_owned()).into());
            }

            prefixes.push(Prefix::create(name, uri));
        }

        Ok(Prefixes::new(prefixes))
    }

    fn simple_handler(
        reader: &mut XmlReader<'a>,
        parent: &[u8],
        child: &[u8],
    ) -> ParserResult<Option<XmlStartElement<'a>>> {
        while let Some(event) = reader.next() {
            return Ok(Some(match event? {
                XmlEvent::Start(el) if el.is_local_name(child) => el,
                XmlEvent::End(el) if el.local_name().as_ref() == parent => break,
                _ => continue,
            }));
        }
        Ok(None)
    }
}

impl EpubParserValidator for PackageParser<'_, '_> {
    fn config(&self) -> &EpubParseConfig {
        self.ctx.config
    }
}

impl EpubParser<'_> {
    pub(super) fn parse_package(
        &mut self,
        data: &[u8],
        location: String,
    ) -> ParserResult<ProcessedPackageData> {
        let mut data = PackageParser::new(&mut self.ctx, data, &location).parse_opf()?;
        // Finalize and set the package location
        data.package.location = location;

        Ok(data)
    }
}
