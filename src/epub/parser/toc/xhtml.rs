use crate::epub::consts::{epub, xhtml, xhtml::bytes, xml};
use crate::epub::metadata::EpubVersion;
use crate::epub::parser::EpubParserValidator;
use crate::epub::parser::toc::TocParser;
use crate::epub::toc::{EpubTocEntryData, TocGroups};
use crate::parser::ParserResult;
use crate::parser::xml::{XmlEvent, XmlStartElement, extract_attributes};

impl<'a> TocParser<'_, 'a> {
    pub(super) fn parse_epub3_nav(mut self) -> ParserResult<TocGroups> {
        // Reading text may consume an important event, so
        // temporarily store consumed events to continue from.
        let mut next_event = None;

        while let Some(event) = self.reader.take_or_next(&mut next_event) {
            match event? {
                XmlEvent::Start(el) => match el.local_name() {
                    // Root Entry
                    bytes::NAV => self.push_nav_root(&el)?,
                    // Nested Entry
                    bytes::LIST_ITEM => next_event = self.push_nav_child(&el)?,
                    bytes::ANCHOR => self.handle_nav_anchor(&el)?,
                    _ => {}
                },
                XmlEvent::End(el) => match el.local_name().as_ref() {
                    bytes::NAV | bytes::LIST_ITEM => {
                        self.handle_pop(EpubVersion::EPUB3);
                    }
                    _ => {}
                },
                _ => {}
            }
        }

        Ok(self.groups)
    }

    fn push_nav_root(&mut self, el: &XmlStartElement<'_>) -> ParserResult<()> {
        extract_attributes! {
            el.attributes(),
            // Extract root kind
            epub::bytes::TYPE => epub_type,
            // Optional
            xml::bytes::ID    => id,
            ..remaining,
        }
        // Validate
        let mut epub_type = self.require_attribute(epub_type, "nav[*epub:type]")?;
        // Although rare, `epub:type` allows several properties
        // separated by whitespace for toc elements. As a result,
        // get the first value as it is the most relevant and ignore the rest.
        epub_type.shrink_to(epub_type.find(' ').unwrap_or(epub_type.len()));

        // Extracts the title of the root toc entry.
        let (_, label) = self
            .reader
            .get_text_till_either(el.name(), xhtml::ORDERED_LIST.as_bytes())?;

        self.stack.push(EpubTocEntryData {
            attributes: remaining.into(),
            kind: Some(epub_type),
            id,
            label,
            ..EpubTocEntryData::default()
        });
        Ok(())
    }

    fn push_nav_child(&mut self, el: &XmlStartElement<'_>) -> ParserResult<Option<XmlEvent<'a>>> {
        extract_attributes! {
            el.attributes(),
            xml::bytes::ID => id,
            ..remaining,
        }
        // For EPUB 3, <li> elements may act as a grouping header
        // if there's no direct <a> element containing an href & label.
        //
        // If the element does contain a direct <a> element,
        // the label retrieved here will be overridden.
        let (consumed_event, label) = self
            .reader
            .get_text_till_either(el.name(), xhtml::ANCHOR.as_bytes())?;

        self.stack.push(EpubTocEntryData {
            attributes: remaining.into(),
            id,
            label,
            ..EpubTocEntryData::default()
        });
        Ok(consumed_event)
    }

    fn handle_nav_anchor(&mut self, el: &XmlStartElement) -> ParserResult<()> {
        if let Some(nav_entry) = self.stack.last_mut() {
            extract_attributes! {
                el.attributes(),
                bytes::HREF       => href_raw,
                xml::bytes::ID    => id,
                epub::bytes::TYPE => epub_type,
            }
            // Validate
            self.ctx.check_attribute(&href_raw, "a[*href]")?;
            let href = href_raw
                .as_deref()
                .map(|raw| self.ctx.require_href(self.resolver.resolve(raw)))
                .transpose()?;

            nav_entry.id = id;
            nav_entry.href = href;
            nav_entry.href_raw = href_raw;
            nav_entry.kind = epub_type;
            nav_entry.label = self.reader.get_element_text(el)?;
        }
        Ok(())
    }
}
