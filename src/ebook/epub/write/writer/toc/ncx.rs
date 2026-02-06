use crate::ebook::epub::consts::{ncx, xml};
use crate::ebook::epub::toc::EpubTocEntryData;
use crate::ebook::epub::write::writer::EpubWriterContext;
use crate::ebook::epub::write::writer::toc::{self, EPUB2_TOC_FALLBACKS, TocData};
use crate::ebook::toc::TocEntryKind;
use crate::util::uri::UriResolver;
use crate::writer::WriterResult;
use crate::writer::xml::{XmlWriter, write_element};
use std::io::Write;

struct NavPointIdGenerator<'ebook> {
    prefix: &'ebook str,
    nav_map: Option<&'ebook EpubTocEntryData>,
    count: Option<usize>,
}

impl<'ebook> NavPointIdGenerator<'ebook> {
    fn new(prefix: &'ebook str, nav_map: Option<&'ebook EpubTocEntryData>) -> Self {
        Self {
            count: None,
            prefix,
            nav_map,
        }
    }

    fn check_entry(prefix: &str, max: &mut usize, root: &EpubTocEntryData) {
        if let Some(num) = root
            .id
            .as_deref()
            .and_then(|id| id.strip_prefix(prefix)?.parse::<usize>().ok())
        {
            *max = (*max).max(num);
        }

        for entry in &root.children {
            Self::check_entry(prefix, max, entry);
        }
    }

    fn generate_id(&mut self) -> String {
        let prefix = self.prefix;
        // Find the max to determine where the counter starts at.
        let count = self.count.get_or_insert_with(|| {
            let mut max = 0;
            if let Some(nav_map) = self.nav_map {
                Self::check_entry(self.prefix, &mut max, nav_map);
            }
            max
        });

        *count += 1;

        format!("{prefix}{count}")
    }
}

pub(super) struct NcxTocWriter<'ebook, W> {
    ctx: &'ebook EpubWriterContext<'ebook>,
    resolver: UriResolver<'ebook>,
    writer: XmlWriter<'ebook, W>,
    // Only required for the NCX navPoints, which require an id
    id_generator: NavPointIdGenerator<'ebook>,
}

impl<'ebook, W: Write> NcxTocWriter<'ebook, W> {
    pub(super) fn new(
        ctx: &'ebook EpubWriterContext<'ebook>,
        data: &'ebook TocData<'ebook>,
        writer: W,
    ) -> Self {
        const NAV_POINT_ID_PREFIX: &str = "nav-point-";

        Self {
            resolver: UriResolver::parent_of(&data.location),
            writer: XmlWriter::new(writer),
            id_generator: NavPointIdGenerator::new(
                NAV_POINT_ID_PREFIX,
                toc::get_toc_root(ctx, TocEntryKind::Toc, EPUB2_TOC_FALLBACKS),
            ),
            ctx,
        }
    }

    pub(super) fn write_ncx(mut self) -> WriterResult<()> {
        // DOCTYPE for NCX is omitted as EPUB 2 does not require it:
        // https://idpf.org/epub/20/spec/OPF_2.0_latest.htm#Section2.4.1.2
        // DTD reference:
        // https://www.daisy.org/z3986/2005/ncx-2005-1.dtd
        self.writer.write_utf8_declaration()?;

        write_element! {
            writer: self.writer,
            tag: ncx::NCX,
            attributes: {
                xml::XMLNS   => ncx::NCX_NS,
                ncx::VERSION => ncx::NCX_VERSION,
            }
            inner_content: {
                self.write_ncx_head()?;
                self.write_ncx_doc_title()?;
                self.write_ncx_nav_map()?;
                self.write_ncx_page_list()?;
            }
        }
    }

    fn write_ncx_head(&mut self) -> WriterResult<()> {
        const UNKNOWN: &str = "urn:unknown";
        const ZERO: &str = "0";

        let uid = self
            .ctx
            .epub
            .metadata()
            .identifier()
            .map(|id| id.value())
            .unwrap_or(UNKNOWN);
        let depth = self
            .ctx
            .epub
            .toc()
            .contents()
            .map(|root| root.max_depth())
            .unwrap_or_default()
            // Depth must be at least 1
            .max(1);

        write_element! {
            writer: self.writer,
            tag: ncx::HEAD,
            inner_content: {
                self.write_ncx_meta(ncx::DTB_UID, uid)?;
                self.write_ncx_meta(ncx::DTB_DEPTH, &depth.to_string())?;
                self.write_ncx_meta(ncx::DTB_TOTAL_PAGE_COUNT, ZERO)?;
                self.write_ncx_meta(ncx::DTB_MAX_PAGE_NUMBER, ZERO)?;
            }
        }
    }

    fn write_ncx_meta(&mut self, name: &str, content: &str) -> WriterResult<()> {
        write_element! {
            writer: self.writer,
            tag: ncx::META,
            attributes: {
                ncx::NAME    => name,
                ncx::CONTENT => content,
            }
        }
    }

    fn write_ncx_doc_title(&mut self) -> WriterResult<()> {
        let title = toc::get_toc_root(self.ctx, TocEntryKind::Toc, EPUB2_TOC_FALLBACKS)
            .map(|root| root.label.as_str())
            .unwrap_or_default();

        write_element! {
            writer: self.writer,
            tag: ncx::DOC_TITLE,
            inner_content: {
                self.write_ncx_text(title)?;
            }
        }
    }

    fn write_ncx_nav_map(&mut self) -> WriterResult<()> {
        let Some(root) = toc::get_toc_root(self.ctx, TocEntryKind::Toc, EPUB2_TOC_FALLBACKS) else {
            return Ok(());
        };

        write_element! {
            writer: self.writer,
            tag: ncx::NAV_MAP,
            attributes: {
                xml::ID    => root.id.as_deref(),
                // Arbitrary attributes aren't supported
                // - `navMap` elements support (id)
            }
            inner_content: {
                for entry in &root.children {
                    self.write_ncx_nav_point(entry)?;
                }
            }
        }
    }

    fn write_ncx_page_list(&mut self) -> WriterResult<()> {
        let Some(root) = toc::get_toc_root(self.ctx, TocEntryKind::PageList, EPUB2_TOC_FALLBACKS)
        else {
            return Ok(());
        };

        write_element! {
            writer: self.writer,
            tag: ncx::PAGE_LIST,
            attributes: {
                xml::ID    => root.id.as_deref(),
                ncx::CLASS => root.attributes.get_value(ncx::CLASS),
                // Arbitrary attributes aren't supported
                // - `pageList` elements support (id | class)
            }
            inner_content: {
                for entry in &root.children {
                    self.write_ncx_page_target(entry)?;
                }
            }
        }
    }

    fn write_ncx_nav_point(&mut self, data: &EpubTocEntryData) -> WriterResult<()> {
        fn find_first_href(data: &EpubTocEntryData) -> Option<&str> {
            match data.href.as_deref() {
                Some(href) => Some(href),
                None => data.children.iter().find_map(find_first_href),
            }
        }

        // An href must be available
        let Some(href) = find_first_href(data) else {
            return Ok(());
        };

        // For `navPoint` elements, an `id` is required.
        let generated_id = data.id.is_none().then(|| self.id_generator.generate_id());

        write_element! {
            writer: self.writer,
            tag: ncx::NAV_POINT,
            attributes: {
                xml::ID         => data.id.as_deref().or(generated_id.as_deref()),
                ncx::CLASS      => data.attributes.get_value(ncx::CLASS),
                // The `playOrder` attribute is ignored as it is not required by the EPUB 2 spec:
                // https://idpf.org/epub/20/spec/OPF_2.0_latest.htm#Section2.4.1.2
                ncx::PLAY_ORDER => None,
                // Arbitrary attributes aren't supported
                // - `navPoint` elements support (id | class | playOrder)
            }
            inner_content: {
                self.write_ncx_nav_label(&data.label)?;
                self.write_ncx_content_src(href)?;
                // Write nested entries
                for child in &data.children {
                    self.write_ncx_nav_point(child)?;
                }
            }
        }
    }

    fn write_ncx_page_target(&mut self, data: &EpubTocEntryData) -> WriterResult<()> {
        let Some(src) = &data.href else { return Ok(()) };
        // Normalize the type (front | normal | special)
        let type_value = data
            .kind
            .as_deref()
            .and_then(|kind| match kind {
                ncx::FRONT | ncx::NORMAL | ncx::SPECIAL => Some(kind),
                _ => None,
            })
            .unwrap_or(ncx::NORMAL);

        write_element! {
            writer: self.writer,
            tag: ncx::PAGE_TARGET,
            attributes: {
                xml::ID         => data.id.as_deref(),
                ncx::CLASS      => data.attributes.get_value(ncx::CLASS),
                ncx::TYPE       => type_value,
                ncx::VALUE      => data.attributes.get_value(ncx::VALUE),
                // The `playOrder` attribute is ignored
                ncx::PLAY_ORDER => None,
                //write::VALUE => class.map(|class| class.value()),
                // Arbitrary attributes aren't supported
                // - `navPoint` elements support (id | class | type | value | playOrder)
            }
            inner_content: {
                self.write_ncx_nav_label(&data.label)?;
                self.write_ncx_content_src(src)?;
            }
        }
    }

    fn write_ncx_nav_label(&mut self, label: &str) -> WriterResult<()> {
        write_element! {
            writer: self.writer,
            tag: ncx::NAV_LABEL,
            inner_content: {
                self.write_ncx_text(label)?;
            }
        }
    }

    fn write_ncx_text(&mut self, text: &str) -> WriterResult<()> {
        write_element! {
            writer: self.writer,
            tag: ncx::TEXT,
            text: text,
        }
    }

    fn write_ncx_content_src(&mut self, src: &str) -> WriterResult<()> {
        write_element! {
            writer: self.writer,
            tag: ncx::CONTENT,
            attributes: {
                ncx::SRC => &*self.resolver.relativize(src),
            }
        }
    }
}
