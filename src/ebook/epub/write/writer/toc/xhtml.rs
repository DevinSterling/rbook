use crate::ebook::archive::Archive;
use crate::ebook::epub::consts::{epub, ncx, opf, xhtml, xml};
use crate::ebook::epub::toc::EpubTocEntryData;
use crate::ebook::epub::write::writer::EpubWriterContext;
use crate::ebook::epub::write::writer::toc::{self, EPUB3_TOC_FALLBACKS, TocData};
use crate::ebook::resource::Resource;
use crate::ebook::toc::TocEntryKind;
use crate::parser::xml::{XmlEvent, XmlReader};
use crate::util::uri::{self, UriResolver};
use crate::writer::WriterResult;
use crate::writer::xml::{XmlWriter, write_element};
use std::io::Write;

pub(super) struct XhtmlTocWriter<'ebook, W> {
    ctx: &'ebook EpubWriterContext<'ebook>,
    data: &'ebook TocData<'ebook>,
    resolver: UriResolver<'ebook>,
    writer: XmlWriter<'ebook, W>,
}

impl<'ebook, W: Write> XhtmlTocWriter<'ebook, W> {
    pub(super) fn new(
        ctx: &'ebook EpubWriterContext<'ebook>,
        data: &'ebook TocData<'ebook>,
        writer: W,
    ) -> Self {
        Self {
            resolver: UriResolver::parent_of(&data.location),
            writer: XmlWriter::new(writer),
            ctx,
            data,
        }
    }

    pub(super) fn write_xhtml(mut self) -> WriterResult<()> {
        self.writer.write_utf8_declaration()?;

        write_element! {
            writer: self.writer,
            tag: xhtml::HTML,
            attributes: {
                xml::XMLNS  => xhtml::XHTML_NS,
                epub::XMLNS => epub::EPUB_NS,
            }
            inner_content: {
                self.write_nav_head()?;
                self.write_nav_body()?;
            }
        }
    }

    fn write_nav_head(&mut self) -> WriterResult<()> {
        write_element! {
            writer: self.writer,
            tag: xhtml::HEAD,
            inner_content: {
                self.write_nav_head_content()?;
            }
        }
    }

    fn write_nav_head_content(&mut self) -> WriterResult<()> {
        self.write_stylesheets()?;

        write_element! {
            writer: self.writer,
            tag: xhtml::TITLE,
            text: toc::get_toc_root(self.ctx, TocEntryKind::Toc, EPUB3_TOC_FALLBACKS)
                      .map(|root| root.label.as_str())
                      .unwrap_or_default(),
        }
    }

    fn write_stylesheets(&mut self) -> WriterResult<()> {
        let epub = self.ctx.epub;
        let archive = &epub.archive;

        // White CSS stylesheet links
        if let Some(stylesheets) = &self.ctx.config.generated_toc_stylesheets {
            for stylesheet in stylesheets {
                self.write_stylesheet_link(stylesheet)?;
            }
        } else if !self.data.is_generated {
            let toc_utf8 = Resource::from(&*self.data.location);
            let data = archive.read_resource_as_utf8_bytes(&toc_utf8)?;
            // Attempt to preserve original stylesheet links
            let mut stylesheets = StylesheetExtractor::new(&data);

            while let Some(stylesheet) = stylesheets.extract_stylesheet() {
                self.write_stylesheet_link(&stylesheet)?;
            }
        }
        Ok(())
    }

    fn write_stylesheet_link(&mut self, stylesheet: &str) -> WriterResult<()> {
        if uri::has_scheme(stylesheet) {
            return Ok(());
        }

        let epub = self.ctx.epub;
        let package_resolver = UriResolver::parent_of(&epub.package.location);
        let absolute = package_resolver.resolve(stylesheet);

        if epub.manifest.by_href(&absolute).is_none() {
            return Ok(());
        }

        write_element! {
            writer: self.writer,
            tag: xhtml::LINK,
            attributes: {
                xhtml::REL => xhtml::STYLESHEET,
                xhtml::HREF => &*self.resolver.relativize(&absolute),
            }
        }
    }

    fn write_nav_body(&mut self) -> WriterResult<()> {
        write_element! {
            writer: self.writer,
            tag: xhtml::BODY,
            inner_content: {
                self.write_nav_body_content()?;
            }
        }
    }

    fn write_nav_body_content(&mut self) -> WriterResult<()> {
        // Supports toc, landmarks, pagelist, etc.
        let toc = toc::get_toc_root(self.ctx, TocEntryKind::Toc, EPUB3_TOC_FALLBACKS);
        let landmarks = toc::get_toc_root(self.ctx, TocEntryKind::Landmarks, EPUB3_TOC_FALLBACKS);
        let pagelist = toc::get_toc_root(self.ctx, TocEntryKind::PageList, EPUB3_TOC_FALLBACKS);

        for root in [toc, landmarks, pagelist].into_iter().flatten() {
            self.write_nav_root(root)?;
        }
        // Write remaining
        for (key, root) in &self.ctx.epub.toc.entries {
            if key.version.is_epub3()
                // Avoid rewriting main toc, landmarks, pagelist
                && !matches!(
                    key.kind(),
                    TocEntryKind::Toc | TocEntryKind::Landmarks | TocEntryKind::PageList,
                )
            {
                self.write_nav_root(root)?;
            }
        }
        Ok(())
    }

    fn write_nav_root(&mut self, root: &EpubTocEntryData) -> WriterResult<()> {
        let Some(epub_type) = root.kind.as_deref() else {
            return Ok(());
        };
        let is_page_list = epub_type == TocEntryKind::PageList.as_str();
        let hidden = root
            .attributes
            .get_value(xhtml::HIDDEN)
            // Default to `hidden="hidden"` for page-list if not explicitly set
            .or_else(|| is_page_list.then_some(xhtml::HIDDEN));

        write_element! {
            writer: self.writer,
            tag: xhtml::NAV,
            attributes: {
                xml::ID    => root.id.as_deref(),
                epub::TYPE => epub_type,
                xhtml::HIDDEN => hidden,
                ..root.attributes.iter_key_value(),
            }
            inner_content: {
                // Write label
                write_element! {
                    writer: self.writer,
                    tag: xhtml::H2,
                    text: &root.label,
                    attributes: {
                        epub::TYPE => epub::TITLE,
                    }
                }?;
                // Write nested entries
                self.write_nav_nested_entries(root)?;
            }
        }
    }

    fn write_nav_entry(&mut self, data: &EpubTocEntryData) -> WriterResult<()> {
        write_element! {
            writer: self.writer,
            tag: xhtml::LIST_ITEM,
            attributes: {
                // Ignore the ID as it's written within `write_nav_entry_label`
                xml::ID         => None,
                // Ignore NCX-specific attributes:
                // - This is necessary when writing from a toc
                //   that is derived from an EPUB 2 NCX document
                ncx::PLAY_ORDER => None, // navPoint & pageTarget
                ncx::VALUE      => None, // pageTarget
                ncx::TYPE       => None, // pageTarget
                ..data.attributes.iter_key_value(),
            }
            inner_content: {
                self.write_nav_entry_label(data)?;
                self.write_nav_nested_entries(data)?;
            }
        }
    }

    fn write_nav_entry_label(&mut self, data: &EpubTocEntryData) -> WriterResult<()> {
        let resolved_href = data
            .href
            .as_deref()
            .map(|href| self.resolver.relativize(href));

        write_element! {
            writer: self.writer,
            tag: match &resolved_href {
                // Toc entry points to a specific resource via `href`.
                Some(_) => xhtml::ANCHOR,
                // The entry is a grouping header,
                // most likely containing nested toc entries.
                None => xhtml::SPAN,
            },
            text: &data.label,
            attributes: {
                xml::ID    => data.id.as_deref(),
                epub::TYPE => data.kind.as_deref().and_then(Self::get_epub3_type),
                opf::HREF  => resolved_href.as_deref(),
            }
        }
    }

    fn write_nav_nested_entries(&mut self, data: &EpubTocEntryData) -> WriterResult<()> {
        if data.children.is_empty() {
            return Ok(());
        }

        write_element! {
            writer: self.writer,
            tag: xhtml::ORDERED_LIST,
            inner_content: {
                for child in &data.children {
                    self.write_nav_entry(child)?;
                }
            }
        }
    }

    fn get_epub3_type(epub3_type: &str) -> Option<&str> {
        match epub3_type {
            // Filter out legacy NCX pageTarget type enumerations
            ncx::FRONT | ncx::NORMAL | ncx::SPECIAL => None,
            _ => Some(epub3_type),
        }
    }
}

struct StylesheetExtractor<'ebook> {
    reader: XmlReader<'ebook>,
}

impl<'ebook> StylesheetExtractor<'ebook> {
    fn new(data: &'ebook [u8]) -> Self {
        Self {
            reader: XmlReader::from_bytes(false, data),
        }
    }

    fn extract_stylesheet(&mut self) -> Option<String> {
        while let Some(result) = self.reader.next() {
            let Ok(event) = result else {
                return None;
            };
            let XmlEvent::Start(el) = event else {
                continue;
            };
            if el.is_local_name(xhtml::LINK)
                && let Ok(Some(rel)) = el.get_attribute_raw(xhtml::REL)
                && &*rel == xhtml::STYLESHEET.as_bytes()
                && let Ok(Some(stylesheet)) = el.get_attribute(xhtml::HREF)
            {
                return Some(stylesheet);
            } else if el.is_local_name(xhtml::BODY) {
                return None;
            }
        }
        None
    }
}
