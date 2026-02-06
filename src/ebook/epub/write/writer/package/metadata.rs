use crate::ebook::element::Attributes;
use crate::ebook::epub::consts::{dc, opf, xml};
use crate::ebook::epub::metadata::{
    EpubMetaEntryData, EpubMetaEntryKind, EpubRefinementsData, EpubVersion,
};
use crate::ebook::epub::write::writer::EpubWriterContext;
use crate::ebook::epub::write::writer::package::spine::SpineIdGenerator;
use crate::ebook::epub::write::writer::package::{PackageIdGenerator, PackageWriter};
use crate::ebook::metadata::datetime::DateTime;
use crate::util;
use crate::writer::WriterResult;
use crate::writer::xml::{XmlWriter, write_element};
use std::io::Write;

struct MetadataWriter<'package, 'ebook, W: Write> {
    ctx: &'package EpubWriterContext<'ebook>,
    metadata_id_generator: PackageIdGenerator<'ebook>,
    // Mutable/exclusive references from `PackageWriter` ('writer)
    writer: &'package mut XmlWriter<'ebook, W>,
    spine_id_generator: &'package mut SpineIdGenerator<'ebook>,
}

impl<'package, 'ebook, W: Write> MetadataWriter<'package, 'ebook, W> {
    fn new(
        ctx: &'package EpubWriterContext<'ebook>,
        writer: &'package mut XmlWriter<'ebook, W>,
        spine_id_generator: &'package mut SpineIdGenerator<'ebook>,
    ) -> Self {
        Self {
            metadata_id_generator: PackageIdGenerator::new("meta-", ctx.epub),
            ctx,
            writer,
            spine_id_generator,
        }
    }

    pub(super) fn write_metadata(&mut self) -> WriterResult<()> {
        let epub = self.ctx.epub;
        let metadata_entries = epub.metadata.entries.iter().flat_map(|(_, group)| group);

        write_element! {
            writer: self.writer,
            tag: opf::METADATA,
            attributes: {
                dc::XMLNS_DC => dc::DUBLIN_CORE_NS,
                // Legacy EPUB 2 attribute compatibility
                opf::XMLNS_OPF where self.ctx.supports_epub2() => opf::OPF_NS,
            }
            inner_content: {
                // Handle metadata entries
                for entry in metadata_entries {
                    self.write_metadata_entry(entry, None)?;
                }

                // Check if the publication/modified date must be generated
                self.generate_dates()?;

                // Handle manifest + spine entry refinements
                self.write_manifest_and_spine_refinements()?;

                // Generate the EPUB 2 cover image metadata entry, if needed
                self.generate_epub2_cover_entry()?;
            }
        }
    }

    fn generate_dates(&mut self) -> WriterResult<()> {
        let metadata = self.ctx.epub.metadata();
        // NOTE: If the platform is `wasm32-unknown-unknown`,
        // a publication and modification date will not be generated
        // as `DateTime::try_now` will return `None`.
        let now = std::cell::LazyCell::new(|| DateTime::try_now().map(|now| now.to_string()));

        // Generate the publication date (dc:date)
        if metadata.published_entry().is_none()
            && let Some(now) = now.as_deref()
        {
            write_element! {
                writer: self.writer,
                tag: dc::DATE,
                text: now,
            }?;
        }
        // Generate the modified date (dcterms:modified)
        if self.ctx.supports_epub3()
            && metadata.modified_entry().is_none()
            && let Some(now) = now.as_deref()
        {
            write_element! {
                writer: self.writer,
                tag: opf::META,
                text: now,
                attributes: {
                    opf::PROPERTY => dc::MODIFIED,
                }
            }?;
        }
        Ok(())
    }

    fn generate_epub2_cover_entry(&mut self) -> WriterResult<()> {
        // Generating an EPUB 2 cover entry is only applicable when targeting EPUB 2
        if !self.ctx.supports_epub2()
            // A cover entry exists already; no need to generate one
            || self.ctx.epub.metadata.entries.contains_key(opf::COVER)
        {
            return Ok(());
        }

        // Can't generate a cover image entry if there's no cover image in the manifest
        if let Some(cover_image) = self.ctx.epub.manifest().cover_image() {
            write_element! {
                writer: self.writer,
                tag: opf::META,
                attributes: {
                    opf::NAME => opf::COVER,
                    opf::CONTENT => cover_image.id(),
                }
            }?;
        }
        Ok(())
    }

    fn write_manifest_and_spine_refinements(&mut self) -> WriterResult<()> {
        if !self.ctx.supports_epub3() {
            return Ok(());
        }
        let epub = self.ctx.epub;

        // Write manifest refinements
        for (id, entry) in &epub.manifest.entries {
            let refines = Self::refines_fragment(id);
            self.write_refinements(&refines, &entry.refinements)?;
        }
        // Write spine refinements
        for entry in &epub.spine.entries {
            if entry.refinements.is_empty() {
                continue;
            }
            let refines = Self::refines_fragment(match entry.id.as_deref() {
                Some(id) => id,
                // If a spine entry has refinements yet no id,
                // generate an id as it is required.
                None => self.spine_id_generator.generate_id(entry),
            });
            self.write_refinements(&refines, &entry.refinements)?;
        }
        Ok(())
    }

    fn write_metadata_entry(
        &mut self,
        meta: &'ebook EpubMetaEntryData,
        refines: Option<&str>,
    ) -> WriterResult<()> {
        let supports_epub3 = self.ctx.supports_epub3();
        let tag = match &meta.kind {
            EpubMetaEntryKind::DublinCore { .. } => &meta.property,
            EpubMetaEntryKind::Link { .. } => opf::LINK,
            // Skip incompatible meta
            EpubMetaEntryKind::Meta {
                version: EpubVersion::EPUB3,
            } if !supports_epub3 => return Ok(()),
            EpubMetaEntryKind::Meta { .. } => opf::META,
        };

        // If absent, an id must be generated if there are refinements
        let has_refinements = !meta.refinements.is_empty();
        let generated_id = (supports_epub3 && has_refinements && meta.id.is_none())
            .then(|| self.metadata_id_generator.generate_id());
        let id = meta.id.as_deref().or(generated_id.as_deref());

        self.writer
            .start_element(tag)?
            // Add common attributes
            .add_attribute(xml::ID, id)
            .add_attribute(opf::REFINES, refines)
            .add_attribute(xml::LANG, meta.language.as_deref())
            .add_attribute(
                opf::TEXT_DIR,
                // Text directionality is an EPUB 3 feature
                (supports_epub3 && !meta.text_direction.is_auto())
                    .then_some(meta.text_direction.as_str()),
            );

        // Check if EPUB 3 refinements should be downgraded to EPUB 2.
        // - Refinements can't be downgraded if the version is EPUB 3,
        //   as it doesn't support it. EpubCheck v5.3.0 will also flag an error.
        if self.ctx.version().is_epub2() {
            self.downgrade_refinements(meta);
        }

        // Write primary data
        match &meta.kind {
            EpubMetaEntryKind::DublinCore { .. } => self.write_dublin_core(meta)?,
            EpubMetaEntryKind::Meta {
                version: EpubVersion::EPUB2,
            } => self.write_meta2(meta)?,
            // Non-EPUB2 meta is defaulted as the newer EPUB3 standard
            EpubMetaEntryKind::Meta { .. } => self.write_meta3(meta)?,
            EpubMetaEntryKind::Link { .. } => self.write_link(meta)?,
        };

        // Handle refinements
        if supports_epub3 && let Some(id) = id {
            self.write_refinements(&Self::refines_fragment(id), &meta.refinements)?;
        }

        Ok(())
    }

    fn write_refinements(
        &mut self,
        refines_fragment: &str,
        refinements: &'ebook EpubRefinementsData,
    ) -> WriterResult<()> {
        if refinements.is_empty() {
            return Ok(());
        }
        for refinement in refinements.iter() {
            // Parent ID references must start with `#`
            self.write_metadata_entry(refinement, Some(refines_fragment))?;
        }
        Ok(())
    }

    fn write_dublin_core(&mut self, dublin_core: &EpubMetaEntryData) -> WriterResult<()> {
        self.writer
            .add_attributes(Self::filter_attributes(&dublin_core.attributes, |_| true))
            .finish_text_element(dublin_core.value.as_str())
    }

    fn write_meta2(&mut self, meta2: &EpubMetaEntryData) -> WriterResult<()> {
        self.writer
            .add_attribute(opf::NAME, meta2.property.as_str())
            .add_attribute(opf::CONTENT, meta2.value.as_str())
            .add_attributes(Self::filter_attributes(&meta2.attributes, |name| {
                matches!(name, opf::NAME | opf::CONTENT)
            }))
            .finish_empty_element()
    }

    fn write_meta3(&mut self, meta3: &EpubMetaEntryData) -> WriterResult<()> {
        self.writer
            .add_attribute(opf::PROPERTY, meta3.property.as_str())
            .add_attributes(Self::filter_attributes(&meta3.attributes, |name| {
                matches!(name, opf::PROPERTY)
            }))
            .finish_text_element(meta3.value.as_str())
    }

    fn write_link(&mut self, link: &EpubMetaEntryData) -> WriterResult<()> {
        self.writer
            .add_attributes(Self::filter_attributes(&link.attributes, |_| true))
            .finish_empty_element()
    }

    /// Filters out any duplicate attributes.
    fn filter_attributes(
        attributes: &Attributes,
        reject: impl Fn(&str) -> bool,
    ) -> impl Iterator<Item = (&str, &str)> {
        attributes.iter_key_value().filter(move |(name, _)| {
            !matches!(*name, xml::ID | opf::REFINES | xml::LANG | opf::TEXT_DIR) || !reject(name)
        })
    }

    /// Converts an entry `id` into a valid `refines` attribute field value.
    ///
    /// `my-id` → `#my-id`
    fn refines_fragment(id: &str) -> String {
        util::str::prefix("#", id)
    }

    /// Downgrading based on <https://idpf.org/epub/20/spec/OPF_2.0_final_spec.html#AppendixA>.
    fn downgrade_refinements(&mut self, meta: &EpubMetaEntryData) {
        if meta.refinements.is_empty() {
            return;
        }

        let writer = &mut self.writer;
        let attributes = &meta.attributes;
        // Retrieves a refinement by property
        let get_ref = |property| {
            meta.refinements
                .by_refinement(property)
                .map(|r| r.value.as_str())
        };

        match meta.property.as_str() {
            dc::IDENTIFIER if !attributes.has_name(opf::OPF_SCHEME) => {
                writer.add_attribute(opf::OPF_SCHEME, get_ref(opf::IDENTIFIER_TYPE));
            }
            dc::CREATOR | dc::CONTRIBUTOR => {
                if !attributes.has_name(opf::OPF_ROLE) {
                    writer.add_attribute(opf::OPF_ROLE, get_ref(opf::ROLE));
                }
                if !attributes.has_name(opf::OPF_FILE_AS) {
                    writer.add_attribute(opf::OPF_FILE_AS, get_ref(opf::FILE_AS));
                }
            }
            _ => {}
        }
    }
}

impl<W: Write> PackageWriter<'_, W> {
    pub(super) fn write_metadata(&mut self) -> WriterResult<()> {
        MetadataWriter::new(self.ctx, &mut self.writer, &mut self.generated_spine_ids)
            .write_metadata()
    }
}
