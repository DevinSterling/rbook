use crate::ebook::epub::consts::{opf, xml};
use crate::ebook::epub::toc::EpubTocEntryData;
use crate::ebook::epub::write::writer::package::PackageWriter;
use crate::ebook::epub::write::writer::toc::{self, EPUB2_TOC_FALLBACKS};
use crate::ebook::toc::TocEntryKind;
use crate::writer::WriterResult;
use crate::writer::xml::write_element;
use std::io::Write;

impl<W: Write> PackageWriter<'_, W> {
    pub(super) fn write_guide(&mut self) -> WriterResult<()> {
        if !self.ctx.supports_epub2() {
            return Ok(());
        }
        let Some(landmarks) =
            toc::get_toc_root(self.ctx, TocEntryKind::Landmarks, EPUB2_TOC_FALLBACKS)
        else {
            return Ok(());
        };

        write_element! {
            writer: self.writer,
            tag: opf::GUIDE,
            inner_content: {
                for reference in &landmarks.children {
                    self.write_reference(reference)?;
                }
            }
        }
    }

    fn write_reference(&mut self, data: &EpubTocEntryData) -> WriterResult<()> {
        let (Some(kind), Some(href)) = (data.kind.as_deref(), data.href.as_deref()) else {
            return Ok(());
        };

        // Arbitrary attributes (data.attributes) are ignored here
        write_element! {
            writer: self.writer,
            tag: opf::REFERENCE,
            attributes: {
                xml::ID    => data.id.as_deref(),
                opf::TYPE  => kind,
                opf::TITLE => data.label.as_str(),
                opf::HREF  => &*self.resolver.relativize(href),
            }
        }
    }
}
