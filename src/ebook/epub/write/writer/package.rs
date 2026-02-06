use crate::ebook::epub::Epub;
use crate::ebook::epub::consts::{dc, opf, xml};
use crate::ebook::epub::metadata::EpubMetaEntryData;
use crate::ebook::epub::package::Prefixes;
use crate::ebook::epub::write::writer::package::spine::SpineIdGenerator;
use crate::ebook::epub::write::writer::{EpubWriter, EpubWriterContext};
use crate::util::uri::{self, UriResolver};
use crate::writer::WriterResult;
use crate::writer::xml::{XmlWriter, write_element};
use std::io::Write;

mod guide;
mod manifest;
mod metadata;
mod spine;

pub(super) struct PackageIdGenerator<'ebook> {
    prefix: &'ebook str,
    epub: &'ebook Epub,
    count: Option<usize>,
}

impl<'ebook> PackageIdGenerator<'ebook> {
    pub(super) fn new(prefix: &'ebook str, epub: &'ebook Epub) -> Self {
        Self {
            count: None,
            prefix,
            epub,
        }
    }

    fn check_meta(prefix: &str, max: &mut usize, entry: &EpubMetaEntryData) {
        if let Some(id) = entry.id.as_deref() {
            Self::check_id(prefix, max, id);
        }
        for refinements in entry.refinements.iter() {
            Self::check_meta(prefix, max, refinements);
        }
    }

    fn check_id(prefix: &str, max: &mut usize, id: &str) {
        if let Some(Ok(num)) = id.strip_prefix(prefix).map(|value| value.parse()) {
            *max = (*max).max(num);
        }
    }

    /// Generates a unique ID within the package document.
    fn generate_id(&mut self) -> String {
        let prefix = self.prefix;
        // Find the max to determine where the counter starts at.
        let count = self.count.get_or_insert_with(|| {
            let epub = self.epub;
            let mut max = 0;

            // Check metadata
            for entry in epub.metadata.entries.values().flatten() {
                Self::check_meta(prefix, &mut max, entry);
            }
            // Check manifest
            for (id, entry) in &epub.manifest.entries {
                Self::check_id(prefix, &mut max, id);
                for refinement in entry.refinements.iter() {
                    Self::check_meta(prefix, &mut max, refinement);
                }
            }
            // Check spine
            for entry in &epub.spine.entries {
                if let Some(id) = entry.id.as_deref() {
                    Self::check_id(prefix, &mut max, id);
                }
                for refinement in entry.refinements.iter() {
                    Self::check_meta(prefix, &mut max, refinement);
                }
            }
            max
        });

        *count += 1;

        format!("{prefix}{count}")
    }
}

pub(super) struct PackageWriter<'ebook, W> {
    ctx: &'ebook EpubWriterContext<'ebook>,
    resolver: UriResolver<'ebook>,
    writer: XmlWriter<'ebook, W>,
    /// If a spine entry contains refinements yet has no ID,
    /// an ID must be generated.
    ///
    /// Unlike NCX navPoint and metadata entries
    /// (which have an ID generated in-place if missing),
    /// spine entries are referenced in multiple areas:
    /// - [`PackageWriter::write_metadata`]
    /// - [`PackageWriter::write_spine`]
    ///
    /// As a result, generated IDs must be stored and accessed
    /// within those areas.
    generated_spine_ids: SpineIdGenerator<'ebook>,
}

impl<'ebook, W: Write> PackageWriter<'ebook, W> {
    fn new(ctx: &'ebook EpubWriterContext<'ebook>, writer: W) -> Self {
        Self {
            resolver: UriResolver::parent_of(&ctx.epub.package.location),
            writer: XmlWriter::new(writer),
            generated_spine_ids: SpineIdGenerator::new(ctx.epub),
            ctx,
        }
    }

    fn write_opf(mut self) -> WriterResult<()> {
        let supported = self.ctx.supports_epub3();
        let package = &self.ctx.epub.package;

        self.writer.write_utf8_declaration()?;

        write_element! {
            writer: self.writer,
            tag: opf::PACKAGE,
            attributes: {
                xml::XMLNS     => opf::OPF_NS,
                // Block writing xmlns:dc/opf on the package element (via `package.attributes`)
                // - They are written on the `<metadata>` element.
                dc::XMLNS_DC   => None,
                opf::XMLNS_OPF => None,
                opf::VERSION   => package.version.raw.as_str(),
                opf::UNIQUE_ID => package.unique_identifier.as_str(),

                // EPUB 3 attributes
                opf::TEXT_DIR where supported && !package.text_direction.is_auto() => package.text_direction.as_str(),
                xml::LANG     where supported => package.language.as_deref(),
                opf::PREFIX   where supported => Self::prefixes_to_string(&package.prefixes).as_deref(),

                ..package.attributes.iter_key_value(),
            }
            inner_content: {
                self.write_metadata()?;
                self.write_manifest()?;
                self.write_spine()?;
                self.write_guide()?;
            }
        }
    }

    fn prefixes_to_string(prefixes: &Prefixes) -> Option<String> {
        let mut buffer = String::new();

        for prefix in prefixes {
            buffer.push_str(prefix.name());
            buffer.push_str(": ");
            buffer.push_str(prefix.uri());
            buffer.push(' ');
        }
        // Remove the last inserted space
        buffer.pop();

        (!buffer.is_empty()).then_some(buffer)
    }
}

impl<W: Write> EpubWriter<'_, W> {
    pub(super) fn write_package(&mut self) -> WriterResult<()> {
        // ZIP entries must use decoded paths.
        let package_location = uri::decode(&self.ctx.epub.package.location);
        self.zip.start_file(&package_location)?;
        PackageWriter::new(&self.ctx, &mut self.zip).write_opf()
    }
}
