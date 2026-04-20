use crate::ebook::spine::PageDirection;
use crate::epub::consts::{opf::bytes, xml};
use crate::epub::errors::EpubError;
use crate::epub::manifest::EpubManifestData;
use crate::epub::parser::package::PackageParser;
use crate::epub::parser::package::metadata::PendingRefinements;
use crate::epub::parser::{EpubParseConfig, EpubParserContext, EpubParserValidator};
use crate::epub::spine::{EpubSpineData, EpubSpineEntryData};
use crate::parser::ParserResult;
use crate::parser::xml::{XmlReader, XmlStartElement, extract_attributes};

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
        extract_attributes! {
            self.spine_el.attributes(),
            bytes::PAGE_DIRECTION => direction as |attr| PageDirection::from_bytes(attr.value()),
            bytes::TOC            => ncx_id,
        }
        // Validate
        let page_direction = direction.unwrap_or_default();
        let ncx_id = self.require_toc(ncx_id)?;

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
        extract_attributes! {
            itemref.attributes(),
            bytes::IDREF      => idref,
            // Optional
            xml::bytes::ID    => id,
            bytes::PROPERTIES => properties,
            bytes::LINEAR     => linear as |attr| attr.value() == bytes::YES,
            ..remaining,
        }
        // Validate
        let idref = self.require_idref(idref)?;

        let refinements = id
            .as_deref()
            .and_then(|id| self.refinements.take_refinements(id))
            .unwrap_or_default();

        Ok(EpubSpineEntryData {
            linear: linear.unwrap_or(true),
            properties: properties.into(),
            attributes: remaining.into(),
            id,
            idref,
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
