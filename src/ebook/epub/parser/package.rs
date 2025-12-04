mod guide;
mod manifest;
mod metadata;
mod spine;

use crate::ebook::element::TextDirection;
use crate::ebook::epub::consts::{self, bytes};
use crate::ebook::epub::errors::EpubFormatError;
use crate::ebook::epub::manifest::EpubManifestData;
use crate::ebook::epub::metadata::{EpubMetadataData, EpubVersion, EpubVersionData};
use crate::ebook::epub::parser::EpubParser;
use crate::ebook::epub::spine::EpubSpineData;
use crate::ebook::epub::toc::EpubTocData;
use crate::ebook::metadata::Version;
use crate::epub::parser::UriResolver;
use crate::epub::parser::package::metadata::PendingRefinements;
use crate::parser::ParserResult;
use crate::parser::xml::{ByteReader, XmlElement, XmlReader};
use crate::util::sync::Shared;
use quick_xml::Reader;
use quick_xml::events::{BytesStart, Event};

pub(super) struct PackageData {
    version: EpubVersionData,
    unique_id: String,
    /// Default package document language
    xml_lang: Option<Shared<String>>,
    /// Default package document text directionality
    dir: TextDirection,
}

pub(super) struct TocLocation {
    /// Absolute toc href location
    pub(super) href: String,
    /// EPUB version associated with the location:
    /// - `.ncx` = EPUB 2
    /// - `.xhtml` = EPUB 3
    pub(super) version: EpubVersion,
}

type ProcessedPackageData = (
    Vec<TocLocation>,
    EpubMetadataData,
    EpubManifestData,
    EpubSpineData,
    EpubTocData,
);

type ProcessedOpfData = (
    Option<EpubMetadataData>,
    Option<EpubManifestData>,
    Option<EpubSpineData>,
    Option<EpubTocData>,
);

pub(super) struct PackageContext<'a> {
    pub(super) resolver: UriResolver<'a>,
    pub(super) reader: ByteReader<'a>,
    pub(super) refinements: PendingRefinements,
}

impl EpubParser<'_> {
    /// Parses the epub `.opf` file and returns all
    /// necessary data required for further processing.
    pub(super) fn parse_opf(
        &mut self,
        resolver: UriResolver,
        data: &[u8],
    ) -> ParserResult<ProcessedPackageData> {
        let (metadata, manifest, spine, guide) = self.handle_opf(resolver, data)?;

        // Optional, if parsing is skipped
        let metadata = self.resolve_section(
            self.config.parse_metadata,
            metadata,
            || EpubFormatError::NoMetadataFound,
            || EpubMetadataData::from(self.version_hint),
        )?;
        let mut manifest = self.resolve_section(
            self.config.parse_manifest,
            manifest,
            || EpubFormatError::NoManifestFound,
            EpubManifestData::empty,
        )?;
        let spine = self.resolve_section(
            self.config.parse_spine,
            spine,
            || EpubFormatError::NoSpineFound,
            EpubSpineData::empty,
        )?;
        let toc_data = guide.unwrap_or_else(EpubTocData::empty);

        // Post-processing
        // Retrieving the toc hrefs depends on the manifest being parsed
        let toc_hrefs = if self.config.parse_manifest && self.config.parse_toc {
            self.get_toc_hrefs(&manifest)?
        } else {
            Vec::new()
        };
        if self.version_hint.is_epub2() && self.config.parse_manifest {
            Self::handle_epub2_cover_image(&metadata, &mut manifest);
        }

        Ok((toc_hrefs, metadata, manifest, spine, toc_data))
    }

    fn resolve_section<T>(
        &self,
        should_parse: bool,
        option_data: Option<T>,
        error: impl FnOnce() -> EpubFormatError,
        default: impl FnOnce() -> T,
    ) -> ParserResult<T> {
        if should_parse {
            // If parsing should happen, check if the parsed item exists
            if let Some(data) = option_data {
                return Ok(data);
            } else if self.config.strict {
                return Err(error().into());
            }
        }
        // if not parsed (skipped) or not strict (lenient), use the default
        Ok(default())
    }

    fn handle_opf(&mut self, resolver: UriResolver, data: &[u8]) -> ParserResult<ProcessedOpfData> {
        let mut package = None;
        let mut metadata = None;
        let mut manifest = None;
        let mut spine = None;
        let mut guide = None;
        let mut ctx = PackageContext {
            reader: Reader::from_reader(data),
            refinements: PendingRefinements::empty(),
            resolver,
        };

        while let Some(event) = ctx.reader.next() {
            let Event::Start(el) = event? else {
                continue;
            };
            match el.local_name().as_ref() {
                bytes::PACKAGE => {
                    package.replace(self.parse_package(&el)?);
                }
                bytes::METADATA if self.config.parse_metadata => {
                    metadata.replace(self.parse_metadata(
                        &mut ctx,
                        Self::assert_required(package.take(), || EpubFormatError::NoPackageFound)?,
                    )?);
                }
                bytes::MANIFEST if self.config.parse_manifest => {
                    manifest.replace(self.parse_manifest(&mut ctx)?);
                }
                bytes::SPINE if self.config.parse_spine => {
                    spine.replace(self.parse_spine(&mut ctx, &el)?);
                }
                // "toc"-related due to its navigational aspect.
                bytes::GUIDE if self.config.parse_toc => {
                    guide.replace(self.parse_guide(&mut ctx, &el)?);
                }
                _ => {}
            }
        }

        Ok((metadata, manifest, spine, guide))
    }

    fn parse_package(&mut self, package: &BytesStart) -> ParserResult<PackageData> {
        let mut attributes = package.bytes_attributes();

        // Required attributes
        let version = self.assert_option(
            attributes.take_attribute_value(consts::VERSION)?,
            "package[*version]",
        )?;
        let unique_id = self.assert_option(
            attributes.take_attribute_value(consts::UNIQUE_ID)?,
            "package[*unique-identifier]",
        )?;

        // Optional attributes
        let xml_lang = attributes
            .take_attribute_value(consts::LANG)?
            .map(Shared::new);
        let dir = attributes
            .take_attribute_value(consts::DIR)?
            .map_or(TextDirection::Auto, TextDirection::from);

        let version = self.handle_epub_version(version)?;

        Ok(PackageData {
            version,
            unique_id,
            xml_lang,
            dir,
        })
    }

    fn handle_epub_version(&mut self, raw: String) -> ParserResult<EpubVersionData> {
        let parsed = EpubVersion::from(match Version::from_str(&raw) {
            Some(version) => version,
            None if !self.config.strict => Version(0, 0),
            _ => return Err(EpubFormatError::UnknownVersion(raw).into()),
        });

        self.version_hint = match &parsed {
            EpubVersion::Epub2(_) | EpubVersion::Epub3(_) => parsed,
            // If not strict, treat unknown as epub3
            _ if !self.config.strict => EpubVersion::EPUB3,
            // Outside the valid range 2 <= version < 4
            _ => return Err(EpubFormatError::UnknownVersion(raw).into()),
        };

        Ok(EpubVersionData { raw, parsed })
    }

    /// If the `cover` meta exists, adds the `cover-image` property to the
    /// referenced manifest entry (if it doesn't contain it already).
    ///
    /// This is ignored for EPUB 3
    fn handle_epub2_cover_image(metadata: &EpubMetadataData, manifest: &mut EpubManifestData) {
        if let Some(properties) = metadata
            .by_group(consts::COVER)
            .and_then(|group| group.first())
            .and_then(|cover| manifest.by_id_mut(&cover.value))
            .map(|entry| &mut entry.properties)
        {
            properties.add_property("cover-image");
        }
    }

    fn simple_handler<'b>(
        reader: &'b mut ByteReader,
        parent: &[u8],
        child: &[u8],
    ) -> ParserResult<Option<BytesStart<'b>>> {
        while let Some(event) = reader.next() {
            return Ok(Some(match event? {
                Event::Start(el) | Event::Empty(el) if el.local_name().as_ref() == child => el,
                Event::End(el) if el.local_name().as_ref() == parent => break,
                _ => continue,
            }));
        }
        Ok(None)
    }
}
