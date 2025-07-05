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

        // Assert existence
        let mut metadata = Self::assert_required(EpubFormatError::NoMetadataFound, metadata)?;
        let mut manifest = Self::assert_required(EpubFormatError::NoManifestFound, manifest)?;
        let spine = Self::assert_required(EpubFormatError::NoSpineFound, spine)?;
        let toc_data = guide.unwrap_or_else(EpubTocData::empty);

        // Post-process
        if self.version_hint.is_epub2() {
            Self::handle_epub2_cover_image(&mut metadata, &mut manifest);
        }
        let toc_hrefs = self.get_toc_hrefs(&manifest)?;

        Ok((toc_hrefs, metadata, manifest, spine, toc_data))
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
            if let Event::Start(el) = event? {
                match el.local_name().as_ref() {
                    bytes::PACKAGE => {
                        package.replace(self.parse_package(&el)?);
                    }
                    bytes::METADATA => {
                        metadata.replace(self.parse_metadata(
                            &mut ctx,
                            Self::assert_required(EpubFormatError::NoPackageFound, package.take())?,
                        )?);
                    }
                    bytes::MANIFEST => {
                        manifest.replace(self.parse_manifest(&mut ctx)?);
                    }
                    bytes::SPINE => {
                        spine.replace(self.parse_spine(&mut ctx, &el)?);
                    }
                    bytes::GUIDE => {
                        guide.replace(self.parse_guide(&mut ctx, &el)?);
                    }
                    _ => {}
                }
            }
        }

        Ok((metadata, manifest, spine, guide))
    }

    fn parse_package(&mut self, package: &BytesStart) -> ParserResult<PackageData> {
        let mut attributes = package.bytes_attributes();

        // Required attributes
        let version = self.assert_optional(
            attributes.take_attribute_value(consts::VERSION)?,
            "package[*version]",
        )?;
        let unique_id = self.assert_optional(
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
            None if !self.settings.strict => Version(0, 0),
            _ => return Err(EpubFormatError::UnknownVersion(raw).into()),
        });

        self.version_hint = match &parsed {
            EpubVersion::Epub2(_) => EpubVersion::EPUB2,
            // If not strict, treat unknown as epub3
            _ => EpubVersion::EPUB3,
        };

        Ok(EpubVersionData { raw, parsed })
    }

    /// If the `cover` meta exists, adds the `cover-image` property to the
    /// referenced manifest entry (if it doesn't contain it already).
    ///
    /// This is ignored for EPUB 3
    fn handle_epub2_cover_image(metadata: &mut EpubMetadataData, manifest: &mut EpubManifestData) {
        if let Some(properties) = metadata
            .by_group_mut(consts::COVER)
            .and_then(|group| group.first())
            .and_then(|cover| cover.id.as_deref())
            .and_then(|cover_id| manifest.by_id_mut(cover_id))
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
            match event? {
                Event::Start(el) | Event::Empty(el) if el.local_name().as_ref() == child => {
                    return Ok(Some(el));
                }
                Event::End(el) if el.local_name().as_ref() == parent => {
                    break;
                }
                _ => {}
            }
        }
        Ok(None)
    }
}
