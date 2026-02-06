use crate::ebook::epub::EpubVersion;
use crate::ebook::epub::consts::{opf, opf::bytes, xml};
use crate::ebook::epub::errors::EpubError;
use crate::ebook::epub::manifest::{EpubManifestData, EpubManifestEntryData};
use crate::ebook::epub::parser::EpubParseConfig;
use crate::ebook::epub::parser::package::metadata::PendingRefinements;
use crate::ebook::epub::parser::package::{
    EpubParserContext, EpubParserValidator, PackageParser, TocLocation,
};
use crate::parser::ParserResult;
use crate::parser::xml::{XmlReader, XmlStartElement};
use crate::util::uri::UriResolver;
use indexmap::IndexMap;

struct ManifestParser<'package, 'a> {
    entries: IndexMap<String, EpubManifestEntryData>,
    ctx: &'package EpubParserContext<'a>,
    reader: &'package mut XmlReader<'a>,
    refinements: &'package mut PendingRefinements,
    resolver: UriResolver<'package>,
}

impl<'package, 'a> ManifestParser<'package, 'a> {
    fn new(
        ctx: &'package EpubParserContext<'a>,
        reader: &'package mut XmlReader<'a>,
        refinements: &'package mut PendingRefinements,
        resolver: UriResolver<'package>,
    ) -> Self {
        Self {
            entries: IndexMap::new(),
            ctx,
            reader,
            refinements,
            resolver,
        }
    }

    fn parse_manifest(mut self) -> ParserResult<EpubManifestData> {
        while let Some(item) = self.next_item()? {
            let (id, entry) = self.parse_item(&item)?;
            self.entries.insert(id, entry);
        }
        Ok(EpubManifestData::new(self.entries))
    }

    fn parse_item(
        &mut self,
        item: &XmlStartElement<'_>,
    ) -> ParserResult<(String, EpubManifestEntryData)> {
        let mut attributes = item.attributes()?;

        // Required fields
        let id = self.require_id(attributes.remove(xml::ID)?)?;
        let href_raw =
            self.require_attribute(attributes.remove(opf::HREF)?, "manifest > item[*href]")?;
        let href = self.require_href(self.resolver.resolve(&href_raw))?;
        let mut media_type = self.require_attribute(
            attributes.remove(opf::MEDIA_TYPE)?,
            "manifest > item[*media_type]",
        )?;

        // Optional fields
        let media_overlay = attributes.remove(opf::MEDIA_OVERLAY)?;
        let fallback = attributes.remove(opf::FALLBACK)?;
        let properties = attributes.remove(opf::PROPERTIES)?.into();
        let refinements = self.refinements.take_refinements(&id).unwrap_or_default();

        // Set media_type to lowercase to enforce uniformity.
        media_type.make_ascii_lowercase();

        let entry = EpubManifestEntryData {
            attributes: attributes.try_into()?,
            refinements,
            href,
            href_raw,
            media_type,
            fallback,
            media_overlay,
            properties,
        };
        Ok((id, entry))
    }

    fn require_id(&self, id: Option<String>) -> ParserResult<String> {
        let id = self.require_attribute(id, "manifest > item[*id]")?;

        // Check the manifest if the given `id` already exists
        if self.is_strict() && self.entries.contains_key(&id) {
            return Err(EpubError::DuplicateItemId(id).into());
        }
        Ok(id)
    }

    fn next_item(&mut self) -> ParserResult<Option<XmlStartElement<'a>>> {
        PackageParser::simple_handler(self.reader, bytes::MANIFEST, bytes::ITEM)
    }
}

impl EpubParserValidator for ManifestParser<'_, '_> {
    fn config(&self) -> &EpubParseConfig {
        self.ctx.config
    }
}

impl PackageParser<'_, '_> {
    pub(super) fn parse_manifest(&mut self) -> ParserResult<EpubManifestData> {
        ManifestParser::new(
            self.ctx,
            &mut self.reader,
            &mut self.refinements,
            self.resolver,
        )
        .parse_manifest()
    }

    /// Retrieve the toc hrefs from the manifest.
    pub(super) fn get_toc_hrefs(&self) -> ParserResult<Vec<TocLocation>> {
        // The manifest is `None` when parsing the manifest is disabled.
        let Some(manifest) = &self.manifest else {
            return Ok(Vec::new());
        };
        // If parsing the toc is explicitly disabled, return early.
        if !self.ctx.config.parse_toc {
            return Ok(Vec::new());
        }

        let version = self.ctx.version.as_major();
        let config = self.ctx.config;

        let preferred_toc = if version.is_epub2() {
            // EPUB 2 only supports NCX.
            version
        } else {
            config.preferred_toc
        };

        let mut ncx = None;
        let mut nav = None;

        // Retrieve EPUB 2 NCX
        if let Some(spine) = &self.spine
            && let Some(ncx_id) = &spine.ncx_id
        {
            ncx = manifest
                .entries
                .get(ncx_id)
                .map(|entry| TocLocation::new(entry.href.to_owned(), EpubVersion::EPUB2));
        }
        // Retrieve EPUB 3 XHTML nav
        if config.retain_variants || preferred_toc.is_epub3() || ncx.is_none() {
            nav = manifest
                .entries
                .values()
                .find(|e| e.properties.has_property(opf::NAV_PROPERTY))
                .map(|entry| TocLocation::new(entry.href.to_owned(), EpubVersion::EPUB3));
        }

        let locations: Vec<_> = if config.retain_variants {
            ncx.into_iter().chain(nav).collect()
        } else if preferred_toc.is_epub3() {
            nav.or(ncx).into_iter().collect()
        } else {
            ncx.or(nav).into_iter().collect()
        };

        if self.is_strict() && version.is_epub3() && locations.is_empty() {
            // No need to check for EPUB 2 here as it's checked
            // in the spine via the `toc` attribute already.
            Err(EpubError::NoXhtmlTocReference.into())
        } else {
            Ok(locations)
        }
    }
}
