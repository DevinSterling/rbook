mod container;
mod package;
mod resource;
mod toc;

use crate::ebook::epub::Epub;
use crate::ebook::epub::metadata::EpubVersion;
use crate::ebook::epub::write::OrphanFilter;
use crate::ebook::epub::write::writer::toc::TocContext;
use crate::writer::WriterResult;
use crate::writer::zip::{ZipFileOptionsExt, ZipWriter};
use std::fmt::Debug;
use std::io::Write;
use std::sync::Arc;
use zip::write::SimpleFileOptions;

#[derive(Clone)]
pub(super) struct EpubWriteConfig {
    /// See [`super::EpubWriteOptions::target`]
    pub(super) targets: EpubWriteTargets,
    /// See [`super::EpubWriteOptions::generate_toc`]
    pub(super) generate_toc: bool,
    /// See [`super::EpubWriteOptions::toc_stylesheet`]
    pub(super) generated_toc_stylesheets: Option<Vec<String>>,
    /// See [`super::EpubWriteOptions::compression`]
    pub(super) compression: u8,
    /// See [`super::EpubWriteOptions::keep_orphans`]
    pub(super) keep_orphans: Option<Arc<dyn OrphanFilter>>,
}

impl Default for EpubWriteConfig {
    fn default() -> Self {
        Self {
            targets: EpubWriteTargets::new(EpubVersion::EPUB2),
            generate_toc: true,
            generated_toc_stylesheets: None,
            compression: 6,
            keep_orphans: None,
        }
    }
}

impl Debug for EpubWriteConfig {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("EpubWriteConfig")
            .field("targets", &self.targets)
            .field("generate_toc", &self.generate_toc)
            .field("compression", &self.compression)
            .finish_non_exhaustive()
    }
}

#[derive(Copy, Clone, Debug, Default)]
pub(super) struct EpubWriteTargets {
    epub2: bool,
    epub3: bool,
}

impl EpubWriteTargets {
    fn new(initial_target: EpubVersion) -> Self {
        let mut targets = EpubWriteTargets::default();
        targets.add(initial_target);
        targets
    }

    pub(super) fn add(&mut self, target: EpubVersion) {
        match target.as_major() {
            EpubVersion::Epub3(_) => self.epub3 = true,
            EpubVersion::Epub2(_) => self.epub2 = true,
            _ => {}
        }
    }

    pub(super) fn clear(&mut self) {
        *self = EpubWriteTargets::default();
    }
}

struct EpubWriterContext<'ebook> {
    epub: &'ebook Epub,
    config: &'ebook EpubWriteConfig,
    toc: TocContext<'ebook>,
}

impl EpubWriterContext<'_> {
    fn supports_epub3(&self) -> bool {
        self.version().is_epub3() || self.config.targets.epub3
    }

    fn supports_epub2(&self) -> bool {
        self.version().is_epub2() || self.config.targets.epub2
    }

    fn version(&self) -> EpubVersion {
        self.epub.package.version.parsed
    }
}

pub(super) struct EpubWriter<'ebook, W: Write> {
    ctx: EpubWriterContext<'ebook>,
    zip: ZipWriter<W>,
}

impl<'ebook, W: Write> EpubWriter<'ebook, W> {
    pub(super) fn new(config: &'ebook EpubWriteConfig, epub: &'ebook Epub, writer: W) -> Self {
        Self {
            ctx: EpubWriterContext {
                toc: TocContext::generate(config, epub),
                epub,
                config,
            },
            zip: ZipWriter::new(
                writer,
                SimpleFileOptions::default()
                    .zip_last_modified_date(epub.metadata().modified())
                    .zip_compression_level(config.compression),
            ),
        }
    }

    pub(super) fn write(mut self) -> WriterResult<W> {
        self.write_mimetype()?;
        self.write_container()?;
        self.write_package()?;
        self.write_toc()?;
        self.write_resources()?;
        self.zip.finish()
    }

    fn write_mimetype(&mut self) -> WriterResult<()> {
        // EPUB requires that the mimetype file must be uncompressed
        self.zip.start_uncompressed_file("mimetype")?;
        self.zip.write_all(b"application/epub+zip")?;
        Ok(())
    }
}
