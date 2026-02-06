use crate::ebook::epub::consts::{opf, opf::bytes, xml};
use crate::ebook::epub::errors::EpubError;
use crate::ebook::epub::manifest::EpubManifestData;
use crate::ebook::epub::parser::package::PackageParser;
use crate::ebook::epub::parser::package::metadata::PendingRefinements;
use crate::ebook::epub::parser::{EpubParseConfig, EpubParserContext, EpubParserValidator};
use crate::ebook::epub::spine::{EpubSpineData, EpubSpineEntryData};
use crate::ebook::spine::PageDirection;
use crate::parser::ParserResult;
use crate::parser::xml::{XmlReader, XmlStartElement};

/// Stores additional information required during parsing-time.
pub(super) struct TempEpubSpine {
    pub(super) ncx_id: Option<String>,
    pub(super) data: EpubSpineData,
}

struct SpineParser<'package, 'a> {
    entries: Vec<EpubSpineEntryData>,
    ctx: &'package EpubParserContext<'a>,
    reader: &'package mut XmlReader<'a>,
    spine_el: &'package XmlStartElement<'a>,
    manifest: Option<&'package EpubManifestData>,
    refinements: &'package mut PendingRefinements,
}

impl<'package, 'a> SpineParser<'package, 'a> {
    fn new(
        ctx: &'package EpubParserContext<'a>,
        reader: &'package mut XmlReader<'a>,
        spine_el: &'package XmlStartElement<'a>,
        manifest: Option<&'package EpubManifestData>,
        refinements: &'package mut PendingRefinements,
    ) -> Self {
        Self {
            entries: Vec::new(),
            ctx,
            reader,
            spine_el,
            manifest,
            refinements,
        }
    }

    fn parse_spine(mut self) -> ParserResult<TempEpubSpine> {
        let page_direction = self
            .spine_el
            .get_attribute_raw(opf::PAGE_PROGRESSION_DIRECTION)?
            .map(PageDirection::from_bytes)
            .unwrap_or_default();

        // Attempt to get NCX id
        let ncx_id = self.require_toc(self.spine_el.get_attribute(opf::TOC)?)?;

        while let Some(itemref) = self.next_itemref()? {
            let entry = self.parse_itemref(&itemref)?;
            self.entries.push(entry);
        }

        Ok(TempEpubSpine {
            data: EpubSpineData::new(page_direction, self.entries),
            ncx_id,
        })
    }

    fn parse_itemref(&mut self, itemref: &XmlStartElement<'_>) -> ParserResult<EpubSpineEntryData> {
        let mut attributes = itemref.attributes()?;

        // Required fields
        let idref = self.require_idref(attributes.remove(opf::IDREF)?)?;

        // Optional fields
        let id = attributes.remove(xml::ID)?;
        let properties = attributes.remove(opf::PROPERTIES)?.into();
        let linear = attributes
            .remove(opf::LINEAR)?
            .is_none_or(|linear| linear == opf::YES);
        let refinements = id
            .as_deref()
            .and_then(|id| self.refinements.take_refinements(id))
            .unwrap_or_default();

        Ok(EpubSpineEntryData {
            attributes: attributes.try_into()?,
            id,
            idref,
            linear,
            properties,
            refinements,
        })
    }

    fn require_idref(&self, idref: Option<String>) -> ParserResult<String> {
        let idref = self.require_attribute(idref, "spine > itemref[*idref]")?;

        // Check the manifest if the given `idref` exists
        if let Some(manifest) = self.requires_manifest()?
            && !manifest.entries.contains_key(&idref)
        {
            return Err(EpubError::InvalidIdref(idref).into());
        }
        Ok(idref)
    }

    fn require_toc(&self, ncx_id: Option<String>) -> ParserResult<Option<String>> {
        // Required for EPUB 2
        if self.is_strict() && self.ctx.version.is_epub2() && ncx_id.is_none() {
            return Err(EpubError::NoNcxReference.into());
        }

        // NCX is optional for EPUB 3
        let Some(ncx_id) = ncx_id else {
            return Ok(None);
        };

        // If an NCX id is present, check if the resource exists
        if let Some(manifest) = self.requires_manifest()?
            && !manifest.entries.contains_key(&ncx_id)
        {
            return Err(EpubError::InvalidNcxReference(ncx_id).into());
        }
        Ok(Some(ncx_id))
    }

    fn requires_manifest(&self) -> ParserResult<Option<&EpubManifestData>> {
        // - If the manifest is explicitly skipped during parsing,
        //   it must not be checked as it will be `None`.
        if self.is_strict() && self.ctx.config().parse_manifest {
            // - At this point, the manifest must exist
            self.manifest
                .map(Some)
                .ok_or_else(|| EpubError::NoManifestFound.into())
        } else {
            Ok(None)
        }
    }

    fn next_itemref(&mut self) -> ParserResult<Option<XmlStartElement<'a>>> {
        PackageParser::simple_handler(self.reader, bytes::SPINE, bytes::ITEMREF)
    }
}

impl EpubParserValidator for SpineParser<'_, '_> {
    fn config(&self) -> &EpubParseConfig {
        self.ctx.config
    }
}

impl<'parser> PackageParser<'parser, '_> {
    pub(super) fn parse_spine(
        &mut self,
        spine: &XmlStartElement<'parser>,
    ) -> ParserResult<TempEpubSpine> {
        SpineParser::new(
            self.ctx,
            &mut self.reader,
            spine,
            self.manifest.as_ref(),
            &mut self.refinements,
        )
        .parse_spine()
    }
}
