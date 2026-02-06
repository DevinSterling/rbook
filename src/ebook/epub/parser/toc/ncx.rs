use crate::ebook::epub::consts::{ncx, ncx::bytes};
use crate::ebook::epub::metadata::EpubVersion;
use crate::ebook::epub::parser::EpubParserValidator;
use crate::ebook::epub::toc::TocGroups;
use crate::ebook::toc::TocEntryKind;
use crate::epub::parser::toc::TocParser;
use crate::parser::ParserResult;
use crate::parser::xml::{XmlEvent, XmlStartElement};

impl TocParser<'_> {
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
        let mut attributes = el.attributes()?;
        let mut root = Self::new_toc_entry(&mut attributes)?;

        // For NCX, rbook supports `navMap` and `pageList`.
        // If the current element is not `navMap`, then it is `pageList`
        root.kind = Some(
            match el.local_name() {
                bytes::NAV_MAP => TocEntryKind::Toc,
                _ => TocEntryKind::PageList,
            }
            .to_string(),
        );
        root.attributes = attributes.try_into()?;
        self.stack.push(root);
        Ok(())
    }

    fn push_ncx_child(&mut self, el: &XmlStartElement<'_>) -> ParserResult<()> {
        let mut attributes = el.attributes()?;
        let mut child = Self::new_toc_entry(&mut attributes)?;

        // PageTarget elements require a `type` attribute.
        // - Kinds: "front" | "normal" | "special"
        if el.is_local_name(bytes::PAGE_TARGET) {
            let kind = attributes.remove(ncx::TYPE)?;
            self.require_attribute(kind.as_deref(), "pageTarget[*type]")?;
            child.kind = kind;
        }

        child.attributes = attributes.try_into()?;
        self.stack.push(child);
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
            // NCX documents require content elements to have the src attribute
            let href_raw = self
                .ctx
                .require_attribute(el.get_attribute(ncx::SRC)?, "content[*src]")?;

            nav_entry.href = Some(self.ctx.require_href(self.resolver.resolve(&href_raw))?);
            nav_entry.href_raw = Some(href_raw);
        }
        Ok(())
    }
}
