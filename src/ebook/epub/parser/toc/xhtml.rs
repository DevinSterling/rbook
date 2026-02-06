use crate::ebook::epub::consts::{epub, xhtml, xhtml::bytes, xml};
use crate::ebook::epub::metadata::EpubVersion;
use crate::ebook::epub::parser::EpubParserValidator;
use crate::ebook::epub::toc::TocGroups;
use crate::epub::parser::toc::TocParser;
use crate::parser::ParserResult;
use crate::parser::xml::{XmlEvent, XmlStartElement};

impl<'a> TocParser<'a> {
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
        let mut attributes = el.attributes()?;
        let mut root = Self::new_toc_entry(&mut attributes)?;

        // Extract root kind
        let mut epub_type =
            self.require_attribute(attributes.remove(epub::TYPE)?, "nav[*epub:type]")?;
        // Although rare, `epub:type` allows several properties
        // separated by whitespace for toc elements. As a result,
        // get the first value as it is the most relevant and ignore the rest.
        epub_type.shrink_to(epub_type.find(' ').unwrap_or(epub_type.len()));

        // Extracts the title of the root toc entry.
        let (_, nav_title) = self
            .reader
            .get_text_till_either(el.name(), xhtml::ORDERED_LIST.as_bytes())?;

        root.kind = epub_type.into();
        root.attributes = attributes.try_into()?;
        root.label = nav_title;
        self.stack.push(root);

        Ok(())
    }

    fn push_nav_child(&mut self, el: &XmlStartElement<'_>) -> ParserResult<Option<XmlEvent<'a>>> {
        let mut attributes = el.attributes()?;
        let mut child = Self::new_toc_entry(&mut attributes)?;

        // For EPUB 3, <li> elements may act as a grouping header
        // if there's no direct <a> element containing an href & label.
        //
        // If the element does contain a direct <a> element,
        // the label retrieved here will be overridden.
        let (consumed_event, label) = self
            .reader
            .get_text_till_either(el.name(), xhtml::ANCHOR.as_bytes())?;

        child.label = label;
        child.attributes = attributes.try_into()?;
        self.stack.push(child);

        Ok(consumed_event)
    }

    fn handle_nav_anchor(&mut self, el: &XmlStartElement) -> ParserResult<()> {
        if let Some(nav_entry) = self.stack.last_mut() {
            let href_raw = self
                .ctx
                .require_attribute(el.get_attribute(xhtml::HREF)?, "a[*href]")?;

            nav_entry.id = el.get_attribute(xml::ID)?;
            nav_entry.href = Some(self.ctx.require_href(self.resolver.resolve(&href_raw))?);
            nav_entry.href_raw = Some(href_raw);
            nav_entry.label = self.reader.get_element_text(el)?;
            nav_entry.kind = el.get_attribute(epub::TYPE)?;
        }
        Ok(())
    }
}
