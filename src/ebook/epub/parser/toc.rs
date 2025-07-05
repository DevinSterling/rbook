use crate::ebook::epub::consts::{self, bytes};
use crate::ebook::epub::errors::EpubFormatError;
use crate::ebook::epub::parser::EpubParser;
use crate::ebook::epub::toc::{EpubTocData, EpubTocEntryData, EpubTocKey};
use crate::ebook::toc::TocEntryKind;
use crate::epub::parser::UriResolver;
use crate::epub::toc::TocGroups;
use crate::parser::ParserResult;
use crate::parser::xml::{ByteReader, BytesAttributes, XmlElement, XmlReader};
use quick_xml::Reader;
use quick_xml::events::{BytesStart, Event};
use quick_xml::name::LocalName;
use std::borrow::Cow;
use std::collections::HashMap;
use std::default::Default;

impl EpubParser<'_> {
    pub(super) fn parse_toc(
        &mut self,
        resolver: &UriResolver,
        data: &[u8],
    ) -> ParserResult<EpubTocData> {
        let toc_groups = self.handle_toc(resolver, data)?;

        // Perform assertions
        if self.settings.strict {
            self.assert_toc(&toc_groups)?;
        }

        Ok(EpubTocData::new(toc_groups))
    }

    pub(super) fn handle_toc(
        &self,
        resolver: &UriResolver,
        data: &[u8],
    ) -> ParserResult<TocGroups> {
        // Keep track of latest nav element entry
        let mut entry_stack = Vec::new();
        let mut toc_groups = HashMap::new();
        let mut natural_order = 0;

        let mut reader = Reader::from_reader(data);
        // Reading text may consume an important event, so
        // temporarily store consumed events to continue from.
        let mut next_event = None;

        while let Some(event) = reader.take_or_next(&mut next_event) {
            match event? {
                // Encountered a `<nav epub:type=...>/<navMap>` tag
                Event::Start(el) if Self::is_nav_root(el.local_name()) => {
                    natural_order = 0;
                    self.handle_push_root(&el, natural_order, &mut entry_stack)?;
                }
                // Encountered a child `<li>/<navPoint>` tag
                Event::Start(el) if Self::is_nav_child(el.local_name()) => {
                    natural_order += 1;
                    next_event =
                        self.handle_push_child(&el, natural_order, &mut entry_stack, &mut reader)?;
                }
                // Encountered a child details `<a>/<content>` tag
                ref event @ (Event::Start(ref el) | Event::Empty(ref el))
                    if Self::is_nav_details(el.local_name()) =>
                {
                    let is_start = matches!(event, Event::Start(_));
                    Self::handle_details(resolver, el, is_start, &mut entry_stack, &mut reader)?;
                }
                // Pop from `entry_stack`; backtrack
                Event::End(el)
                    if Self::is_nav_root(el.local_name())
                        || Self::is_nav_child(el.local_name()) =>
                {
                    self.handle_pop(&mut toc_groups, &mut entry_stack);
                }
                _ => {}
            }
        }

        Ok(toc_groups)
    }

    fn is_nav_root(name: LocalName) -> bool {
        matches!(
            name.as_ref(),
            bytes::NAV | bytes::NAV_MAP | bytes::PAGE_LIST
        )
    }

    fn is_nav_child(name: LocalName) -> bool {
        matches!(
            name.as_ref(),
            bytes::LIST_ITEM | bytes::NAV_POINT | bytes::PAGE_TARGET
        )
    }

    fn is_nav_details(name: LocalName) -> bool {
        matches!(
            name.as_ref(),
            bytes::ANCHOR | bytes::NAV_CONTENT | bytes::NAV_LABEL
        )
    }

    fn handle_push_root(
        &self,
        el: &BytesStart,
        order: usize,
        stack: &mut Vec<EpubTocEntryData>,
    ) -> ParserResult<()> {
        let mut attributes = el.bytes_attributes();
        let mut root = Self::new_toc_entry(order, stack.len(), &mut attributes)?;

        root.kind = self.get_toc_kind(el.local_name(), &mut attributes)?;
        root.attributes = attributes.try_into()?;
        stack.push(root);
        Ok(())
    }

    fn handle_push_child<'b>(
        &self,
        el: &BytesStart,
        order: usize,
        stack: &mut Vec<EpubTocEntryData>,
        reader: &mut ByteReader<'b>,
    ) -> ParserResult<Option<Event<'b>>> {
        let mut attributes = el.bytes_attributes();
        let mut child = Self::new_toc_entry(order, stack.len(), &mut attributes)?;
        let mut continue_from_event = None;

        // If the version is EPUB 3, then the current element is <li>.
        // Get text in <li> elements that have no direct <a> tag
        // but may contain relevant text before any child <li>.
        if self.version_hint.is_epub3() {
            child.label =
                reader.get_text_till_either(&mut continue_from_event, el, &BytesStart::new("a"))?;
        }
        // Check for order explicitly set by NCX <navPoint> elements
        else if self.version_hint.is_epub2() && el.local_name().as_ref() == bytes::NAV_POINT {
            child.order = attributes
                .take_attribute_value(consts::PLAY_ORDER)?
                .and_then(|value| value.parse().ok())
                .unwrap_or(order);
        }

        child.attributes = attributes.try_into()?;
        stack.push(child);
        Ok(continue_from_event)
    }

    fn handle_pop(&self, toc_groups: &mut TocGroups, stack: &mut Vec<EpubTocEntryData>) {
        let Some(nav_entry) = stack.pop() else {
            return;
        };

        // The nav element has a parent
        if let Some(nav_parent) = stack.last_mut() {
            nav_parent.children.push(nav_entry);
        }
        // The nav element does not have a parent; the root.
        else {
            let version = &self.version_hint;
            let toc_kind = nav_entry.kind.clone();
            toc_groups.insert(EpubTocKey::of(toc_kind, *version), nav_entry);
        }
    }

    fn handle_details(
        resolver: &UriResolver,
        el: &BytesStart,
        is_start: bool,
        stack: &mut [EpubTocEntryData],
        reader: &mut ByteReader,
    ) -> ParserResult<()> {
        let Some(nav_entry) = stack.last_mut() else {
            return Ok(());
        };

        // Get attributes and potentially get the `href/src`
        let mut attributes = el.bytes_attributes();
        let el_name = el.local_name();

        // If currently not on a `navLabel`, then the current element is either
        // `a` or `navContent` with a potentially corresponding `href`/`src` attribute
        if el_name.as_ref() == bytes::ANCHOR {
            nav_entry.href_raw = attributes.take_attribute_value(consts::HREF)?;
            nav_entry.kind = attributes
                .take_attribute_value(consts::EPUB_TYPE)?
                .unwrap_or_default()
                .into();
        } else if el_name.as_ref() == bytes::NAV_CONTENT {
            nav_entry.href_raw = attributes.take_attribute_value(consts::SRC)?;
        }
        // Convert relative href into absolute
        nav_entry.href = nav_entry
            .href_raw
            .as_deref()
            .map(|href_raw| resolver.resolve(href_raw));

        // Get value if the element isn't <ncx:content>.
        // <ncx:content> contains no text to extract.
        if el_name.as_ref() != bytes::NAV_CONTENT && is_start {
            nav_entry.label = reader.get_text_simple(el)?;
        }

        Ok(())
    }

    fn assert_toc(&self, map: &TocGroups) -> ParserResult<()> {
        // Check if the epub contains a main table of contents
        if map.contains_key(&EpubTocKey::of(TocEntryKind::Toc, self.version_hint)) {
            Ok(())
        } else {
            Err(EpubFormatError::NoTocFound.into())
        }
    }

    fn get_toc_kind(
        &self,
        name: LocalName,
        attributes: &mut BytesAttributes,
    ) -> ParserResult<TocEntryKind<'static>> {
        Ok(if self.version_hint.is_epub3() {
            let mut epub_type = self.assert_optional(
                attributes.take_attribute_value(consts::EPUB_TYPE)?,
                "nav[*epub:type]",
            )?;
            // Although rare, `epub:type` allows several properties
            // separated by whitespace for toc elements. As a result,
            // get the first value as it is the most relevant and ignore the rest.
            epub_type.shrink_to(epub_type.find(' ').unwrap_or(epub_type.len()));
            epub_type.into()
        } else {
            match name.as_ref() {
                bytes::NAV_MAP => TocEntryKind::Toc,
                bytes::PAGE_LIST => TocEntryKind::PageList,
                _ => TocEntryKind::Other(Cow::Owned(String::from_utf8(name.as_ref().to_vec())?)),
            }
        })
    }

    fn new_toc_entry(
        order: usize,
        depth: usize,
        attributes: &mut BytesAttributes,
    ) -> ParserResult<EpubTocEntryData> {
        Ok(EpubTocEntryData {
            id: attributes.take_attribute_value(consts::ID)?,
            order,
            depth,
            ..Default::default()
        })
    }
}
