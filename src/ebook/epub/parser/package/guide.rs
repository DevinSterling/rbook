use crate::ebook::epub::consts;
use crate::ebook::epub::consts::bytes;
use crate::ebook::epub::parser::EpubParser;
use crate::ebook::epub::toc::{InternalEpubToc, InternalEpubTocEntry};
use crate::ebook::toc::TocEntryKind;
use crate::epub::parser::package::PackageContext;
use crate::parser::ParserResult;
use crate::parser::xml::XmlElement;
use quick_xml::events::BytesStart;

impl EpubParser<'_> {
    pub(super) fn parse_guide(
        &self,
        ctx: &mut PackageContext,
        guide: &BytesStart,
    ) -> ParserResult<InternalEpubToc> {
        let mut root = InternalEpubTocEntry {
            kind: TocEntryKind::Landmarks,
            attributes: guide.bytes_attributes().try_into()?,
            ..InternalEpubTocEntry::default()
        };

        while let Some(el) = Self::simple_handler(&mut ctx.reader, bytes::GUIDE, bytes::REFERENCE)?
        {
            let mut attributes = el.bytes_attributes();

            // Required fields
            let (href, href_raw) = self.assert_option(
                attributes
                    .take_attribute_value(consts::HREF)?
                    .map(|href_raw| (ctx.resolver.resolve(&href_raw), href_raw)),
                "guide > reference[*href]",
            )?;
            let label = self.assert_option(
                attributes.take_attribute_value(consts::GUIDE_TITLE)?,
                "guide > reference[*title]",
            )?;
            let kind = self
                .assert_option(
                    attributes.take_attribute_value(consts::GUIDE_TYPE)?,
                    "guide > reference[*type]",
                )?
                .into();

            // Optional fields
            let id = attributes.take_attribute_value(consts::ID)?;

            root.children.push(InternalEpubTocEntry {
                order: root.children.len() + 1,
                href: Some(href),
                href_raw: Some(href_raw),
                attributes: attributes.try_into()?,
                depth: 1, // The parent `root` has a depth of `0`
                id,
                label,
                kind,
                ..Default::default()
            });
        }
        Ok(InternalEpubToc::from_guide(root))
    }
}
