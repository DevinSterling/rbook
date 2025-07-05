use crate::ebook::epub::consts::{self, bytes};
use crate::ebook::epub::parser::EpubParser;
use crate::ebook::epub::spine::{EpubSpineData, EpubSpineEntryData};
use crate::ebook::spine::PageDirection;
use crate::epub::parser::package::PackageContext;
use crate::parser::ParserResult;
use crate::parser::xml::XmlElement;
use quick_xml::events::BytesStart;

impl EpubParser<'_> {
    pub(super) fn parse_spine(
        &self,
        ctx: &mut PackageContext,
        spine: &BytesStart,
    ) -> ParserResult<EpubSpineData> {
        let mut entries = Vec::new();
        let page_direction = spine
            .get_attribute(consts::PAGE_PROGRESSION_DIRECTION)
            .map(PageDirection::from_bytes)
            .unwrap_or_default();

        while let Some(el) = Self::simple_handler(&mut ctx.reader, bytes::SPINE, bytes::ITEMREF)? {
            let mut attributes = el.bytes_attributes();

            // Required fields
            let idref = self.assert_optional(
                attributes.take_attribute_value(consts::IDREF)?,
                "spine > itemref[*idref]",
            )?;

            // Optional fields
            let id = attributes.take_attribute_value(consts::ID)?;
            let properties = attributes.take_attribute_value(consts::PROPERTIES)?.into();
            let linear = attributes
                .take_attribute_value(consts::LINEAR)?
                .is_none_or(|linear| linear == "yes");
            let refinements = id
                .as_deref()
                .and_then(|id| ctx.refinements.take_refinements(id))
                .unwrap_or_default();

            entries.push(EpubSpineEntryData {
                order: entries.len(),
                attributes: attributes.try_into()?,
                id,
                idref,
                linear,
                properties,
                refinements,
            });
        }
        Ok(EpubSpineData::new(page_direction, entries))
    }
}
