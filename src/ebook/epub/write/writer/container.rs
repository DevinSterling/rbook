use crate::ebook::epub::consts::{ocf, xml};
use crate::ebook::epub::write::writer::{EpubWriter, EpubWriterContext};
use crate::ebook::resource::consts::mime;
use crate::writer::WriterResult;
use crate::writer::xml::{XmlWriter, write_element};
use std::io::Write;

struct ContainerWriter<'ebook, W> {
    ctx: &'ebook EpubWriterContext<'ebook>,
    writer: XmlWriter<'ebook, W>,
}

impl<'ebook, W: Write> ContainerWriter<'ebook, W> {
    fn new(ctx: &'ebook EpubWriterContext<'ebook>, writer: W) -> Self {
        Self {
            writer: XmlWriter::new(writer),
            ctx,
        }
    }

    fn write_container(mut self) -> WriterResult<()> {
        self.writer.write_utf8_declaration()?;

        write_element! {
            writer: self.writer,
            tag: ocf::CONTAINER,
            attributes: {
                ocf::VERSION => ocf::CONTAINER_VERSION,
                xml::XMLNS   => ocf::CONTAINER_NS,
            }
            inner_content: {
                self.write_root_files()?;
            }
        }
    }

    fn write_root_files(&mut self) -> WriterResult<()> {
        write_element! {
            writer: self.writer,
            tag: ocf::ROOT_FILES,
            inner_content: {
                self.write_root_file()?;
            }
        }
    }

    // When multi-rendition support arrives the parameters will be (&mut self, &Rendition)
    fn write_root_file(&mut self) -> WriterResult<()> {
        write_element! {
            writer: self.writer,
            tag: ocf::ROOT_FILE,
            attributes: {
                // Root file paths must not be prefixed with '/'
                ocf::FULL_PATH  => self.ctx.epub.package.location.trim_start_matches('/'),
                ocf::MEDIA_TYPE => mime::OEBPS_PACKAGE,
            }
        }
    }
}

impl<W: Write> EpubWriter<'_, W> {
    pub(super) fn write_container(&mut self) -> WriterResult<()> {
        self.zip.start_file(ocf::CONTAINER_PATH)?;
        ContainerWriter::new(&self.ctx, &mut self.zip).write_container()
    }
}
