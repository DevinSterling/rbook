use crate::ebook::toc::TocEntryKind;
use crate::epub::consts::{ncx, ncx::bytes, xml};
use crate::epub::metadata::EpubVersion;
use crate::epub::parser::EpubParserValidator;
use crate::epub::parser::toc::TocParser;
use crate::epub::toc::{EpubTocEntryData, TocGroups};
use crate::parser::ParserResult;
use crate::parser::xml::{XmlEvent, XmlStartElement, extract_attributes};

impl TocParser<'_, '_> {
    pub(super) fn parse_epub2_ncx(mut self) -> ParserResult<TocGroups> {
        let mut doc_title = String::new();

        while let Some(event) = self.reader.next() {
            match event? {
                XmlEvent::Start(el) => match el.local_name() {
                    bytes::DOC_TITLE => doc_title = self.reader.get_element_text(&el)?,
                    // Root Entry
                    bytes::NAV_MAP | bytes::PAGE_LIST => self.push_ncx_root(&el)?,
                    // Nested Entry
                    bytes::NAV_POINT | bytes::PAGE_TARGET => self.push_ncx_child(&el)?,
                    bytes::NAV_LABEL => self.handle_ncx_label(&el)?,
                    bytes::CONTENT => self.handle_ncx_src(&el)?,
                    _ => {}
                },
                XmlEvent::End(el) => match el.local_name().as_ref() {
                    bytes::NAV_MAP | bytes::PAGE_LIST | bytes::NAV_POINT | bytes::PAGE_TARGET => {
                        self.handle_pop(EpubVersion::EPUB2);
                    }
                    _ => {}
                },
                _ => {}
            }
        }

        // Apply the ncx `docTitle` to the main table of contents
        if let Some(root) = self
            .groups
            .get_mut(&(TocEntryKind::Toc.as_str(), EpubVersion::EPUB2))
        {
            root.label = doc_title;
        }

        Ok(self.groups)
    }

    fn push_ncx_root(&mut self, el: &XmlStartElement<'_>) -> ParserResult<()> {
        extract_attributes! {
            el.attributes(),
            xml::bytes::ID => id,
            ..remaining,
        }

        // For NCX, rbook supports `navMap` and `pageList`.
        // If the current element is not `navMap`, then it is `pageList`
        let kind = Some(
            match el.local_name() {
                bytes::NAV_MAP => TocEntryKind::Toc,
                _ => TocEntryKind::PageList,
            }
            .to_string(),
        );

        self.stack.push(EpubTocEntryData {
            attributes: remaining.into(),
            id,
            kind,
            ..EpubTocEntryData::default()
        });
        Ok(())
    }

    fn push_ncx_child(&mut self, el: &XmlStartElement<'_>) -> ParserResult<()> {
        let is_page_target = el.is_local_name(bytes::PAGE_TARGET);

        extract_attributes! {
            el.attributes(),
            xml::bytes::ID => id,
            // PageTarget elements require a `type` attribute.
            // - Kinds: "front" | "normal" | "special"
            bytes::TYPE where is_page_target => kind,
            ..remaining,
        }

        if is_page_target {
            kind = Some(self.require_attribute(kind, "pageTarget[*type]")?);
        }

        self.stack.push(EpubTocEntryData {
            attributes: remaining.into(),
            id,
            kind,
            ..EpubTocEntryData::default()
        });
        Ok(())
    }

    fn handle_ncx_label(&mut self, el: &XmlStartElement<'_>) -> ParserResult<()> {
        if let Some(nav_entry) = self.stack.last_mut() {
            // Extract text content
            nav_entry.label = self.reader.get_element_text(el)?;
        }
        Ok(())
    }

    fn handle_ncx_src(&mut self, el: &XmlStartElement<'_>) -> ParserResult<()> {
        if let Some(nav_entry) = self.stack.last_mut() {
            let href_raw = el.get_attribute(ncx::SRC)?;

            // NCX documents require content elements to have the src attribute
            self.ctx.check_attribute(&href_raw, "content[*src]")?;
            let href = href_raw
                .as_deref()
                .map(|raw| self.ctx.require_href(self.resolver.resolve(raw)))
                .transpose()?;

            nav_entry.href = href;
            nav_entry.href_raw = href_raw;
        }
        Ok(())
    }
}
