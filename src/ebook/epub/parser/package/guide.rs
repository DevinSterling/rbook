use crate::ebook::epub::consts::{opf, opf::bytes, xml};
use crate::ebook::epub::metadata::EpubVersion;
use crate::ebook::epub::parser::package::PackageParser;
use crate::ebook::epub::parser::{EpubParseConfig, EpubParserContext, EpubParserValidator};
use crate::ebook::epub::toc::{EpubTocData, EpubTocEntryData, EpubTocKey};
use crate::ebook::toc::TocEntryKind;
use crate::parser::ParserResult;
use crate::parser::xml::{XmlReader, XmlStartElement};
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
        self.root.kind = Some(TocEntryKind::Landmarks.to_string());
        self.root.attributes = guide.attributes()?.try_into()?;

        while let Some(reference) = self.next_reference()? {
            let entry = self.parse_reference(&reference)?;
            self.root.children.push(entry);
        }

        let tocs = indexmap::indexmap! {
            EpubTocKey::new(TocEntryKind::Landmarks.to_string(), EpubVersion::EPUB2) => self.root
        };
        Ok(EpubTocData::new(tocs))
    }

    fn parse_reference(
        &mut self,
        reference: &XmlStartElement<'_>,
    ) -> ParserResult<EpubTocEntryData> {
        let mut attributes = reference.attributes()?;

        // Required fields
        let (href, href_raw) = self.require_attribute(
            attributes
                .remove(opf::HREF)?
                .map(|href_raw| (self.resolver.resolve(&href_raw), href_raw)),
            "guide > reference[*href]",
        )?;
        let label =
            self.require_attribute(attributes.remove(opf::TITLE)?, "guide > reference[*title]")?;
        let kind = self
            .require_attribute(attributes.remove(opf::TYPE)?, "guide > reference[*type]")?
            .into();

        // Optional fields
        let id = attributes.remove(xml::ID)?;

        Ok(EpubTocEntryData {
            href: Some(href),
            href_raw: Some(href_raw),
            attributes: attributes.try_into()?,
            id,
            label,
            kind,
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
