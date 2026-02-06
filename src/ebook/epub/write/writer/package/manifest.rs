use crate::ebook::epub::consts::{opf, xml};
use crate::ebook::epub::manifest::EpubManifestEntryData;
use crate::ebook::epub::write::writer::package::PackageWriter;
use crate::ebook::epub::write::writer::toc::TocData;
use crate::ebook::resource::consts::mime;
use crate::writer::WriterResult;
use crate::writer::xml::write_element;
use std::io::Write;

impl<W: Write> PackageWriter<'_, W> {
    pub(super) fn write_manifest(&mut self) -> WriterResult<()> {
        let entries = &self.ctx.epub.manifest.entries;

        write_element! {
            writer: self.writer,
            tag: opf::MANIFEST,
            inner_content: {
                for (id, entry) in entries {
                    self.write_item(id, entry)?;
                }
                self.write_generated_toc_items()?;
            }
        }
    }

    fn write_item(&mut self, id: &str, item: &EpubManifestEntryData) -> WriterResult<()> {
        let supported = self.ctx.supports_epub3();

        write_element! {
            writer: self.writer,
            tag: opf::ITEM,
            attributes: {
                xml::ID            => id,
                opf::HREF          => &*self.resolver.relativize(&item.href),
                opf::MEDIA_TYPE    => item.media_type.as_str(),
                opf::FALLBACK      => item.fallback.as_deref(),

                // EPUB 3 attributes
                opf::MEDIA_OVERLAY where supported => item.media_overlay.as_deref(),
                opf::PROPERTIES    where supported => item.properties.as_option_str(),

                ..item.attributes.iter_key_value(),
            }
        }
    }

    fn write_generated_toc_items(&mut self) -> WriterResult<()> {
        // EPUB 2 NCX ToC
        if let Some(ncx) = &self.ctx.toc.epub2_ncx
            && ncx.is_generated
        {
            self.write_generated_toc_item(ncx, mime::NCX, &[])?;
        }
        // EPUB 3 NAV XHTML ToC
        if let Some(nav) = &self.ctx.toc.epub3_nav
            && nav.is_generated
        {
            self.write_generated_toc_item(
                nav,
                mime::XHTML,
                &[(opf::PROPERTIES, opf::NAV_PROPERTY)],
            )?;
        }
        Ok(())
    }

    fn write_generated_toc_item(
        &mut self,
        toc: &TocData<'_>,
        media_type: &str,
        attributes: &[(&str, &str)],
    ) -> WriterResult<()> {
        write_element! {
            writer: self.writer,
            tag: opf::ITEM,
            attributes: {
                xml::ID         => &*toc.id,
                opf::HREF       => &*self.resolver.relativize(&toc.location),
                opf::MEDIA_TYPE => media_type,
                ..attributes.iter().copied(),
            }
        }
    }
}
