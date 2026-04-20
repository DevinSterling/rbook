use crate::ebook::toc::TocEntryKind;
use crate::epub::consts::{opf::bytes, xml};
use crate::epub::metadata::EpubVersion;
use crate::epub::parser::package::PackageParser;
use crate::epub::parser::{EpubParseConfig, EpubParserContext, EpubParserValidator};
use crate::epub::toc::{EpubTocData, EpubTocEntryData, EpubTocKey};
use crate::parser::ParserResult;
use crate::parser::xml::{XmlReader, XmlStartElement, extract_attributes};
use crate::util::uri::UriResolver;

struct GuideParser<'package, 'a> {
    root: EpubTocEntryData,
    ctx: &'package EpubParserContext<'a>,
    reader: &'package mut XmlReader<'a>,
    resolver: UriResolver<'package>,
}

impl<'package, 'a> GuideParser<'package, 'a> {
    fn new(
        ctx: &'package EpubParserContext<'a>,
        reader: &'package mut XmlReader<'a>,
        resolver: UriResolver<'package>,
    ) -> Self {
        Self {
            root: EpubTocEntryData::default(),
            ctx,
            reader,
            resolver,
        }
    }

    fn parse_guide(mut self, guide: &XmlStartElement<'_>) -> ParserResult<EpubTocData> {
        extract_attributes! {
            guide.attributes(),
            // In nearly all cases, the `guide` element has no attributes
            ..attributes,
        }

        self.root.kind = Some(TocEntryKind::Landmarks.to_string());
        self.root.attributes = attributes.into();

        while let Some(reference) = self.next_reference()? {
            let entry = self.parse_reference(&reference)?;
            self.root.children.push(entry);
        }

        let tocs = indexmap::indexmap! {
            EpubTocKey::new(TocEntryKind::Landmarks.to_string(), EpubVersion::EPUB2) => self.root
        };
        Ok(EpubTocData::new(tocs))
    }

    fn parse_reference(&mut self, el: &XmlStartElement<'_>) -> ParserResult<EpubTocEntryData> {
        extract_attributes! {
            el.attributes(),
            bytes::HREF    => href_raw,
            bytes::TITLE   => label,
            bytes::TYPE    => kind,
            // Optional
            xml::bytes::ID => id,
            ..remaining,
        }
        // Validate
        self.ctx
            .check_attribute(&href_raw, "guide > reference[*href]")?;
        let label = self.require_attribute(label, "guide > reference[*title]")?;
        let kind = self.require_attribute(Some(kind), "guide > reference[*type]")?;
        let href = href_raw
            .as_deref()
            .map(|raw| self.ctx.require_href(self.resolver.resolve(raw)))
            .transpose()?;

        Ok(EpubTocEntryData {
            attributes: remaining.into(),
            id,
            label,
            kind,
            href,
            href_raw,
            ..EpubTocEntryData::default()
        })
    }

    fn next_reference(&mut self) -> ParserResult<Option<XmlStartElement<'a>>> {
        PackageParser::simple_handler(self.reader, bytes::GUIDE, bytes::REFERENCE)
    }
}

impl EpubParserValidator for GuideParser<'_, '_> {
    fn config(&self) -> &EpubParseConfig {
        self.ctx.config
    }
}

impl PackageParser<'_, '_> {
    pub(super) fn parse_guide(&mut self, guide: &XmlStartElement<'_>) -> ParserResult<EpubTocData> {
        GuideParser::new(self.ctx, &mut self.reader, self.resolver).parse_guide(guide)
    }
}
