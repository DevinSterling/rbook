use crate::ebook::epub::Epub;
use crate::ebook::epub::consts::{opf, xml};
use crate::ebook::epub::spine::EpubSpineEntryData;
use crate::ebook::epub::write::writer::package::{PackageIdGenerator, PackageWriter};
use crate::writer::WriterResult;
use crate::writer::xml::write_element;
use std::collections::HashMap;
use std::io::Write;

/// Generate an ID for spine entries.
///
/// IDs are only generated for entries have refinements, yet no ID.
/// Refinements require their associated parent spine entries to have an ID.
pub(super) struct SpineIdGenerator<'ebook> {
    generator: PackageIdGenerator<'ebook>,
    generated: HashMap<*const EpubSpineEntryData, String>,
}

impl<'ebook> SpineIdGenerator<'ebook> {
    const PREFIX: &'static str = "ref-";

    pub(super) fn new(epub: &'ebook Epub) -> Self {
        Self {
            generator: PackageIdGenerator::new(Self::PREFIX, epub),
            generated: HashMap::new(),
        }
    }

    pub(super) fn generate_id(&mut self, entry: &EpubSpineEntryData) -> &str {
        self.generated
            .entry(entry as *const EpubSpineEntryData)
            .or_insert(self.generator.generate_id())
    }

    fn get(&self, entry: &EpubSpineEntryData) -> Option<&str> {
        self.generated
            .get(&(entry as *const EpubSpineEntryData))
            .map(|generated_id| generated_id.as_str())
    }
}

impl<W: Write> PackageWriter<'_, W> {
    pub(super) fn write_spine(&mut self) -> WriterResult<()> {
        let spine = &self.ctx.epub.spine;

        write_element! {
            writer: self.writer,
            tag: opf::SPINE,
            attributes: {
                opf::TOC => {
                    self.ctx.toc.epub2_ncx.as_ref().map(|ncx| &*ncx.id)
                },
                // Page progression direction is an EPUB 3 feature
                opf::PAGE_PROGRESSION_DIRECTION => {
                    (self.ctx.supports_epub3() && !spine.page_direction.is_default())
                        .then_some(spine.page_direction.as_str())
                },
            }
            inner_content: {
                for entry in &spine.entries {
                    self.write_itemref(entry)?;
                }
            }
        }
    }

    fn write_itemref(&mut self, itemref: &EpubSpineEntryData) -> WriterResult<()> {
        let supported = self.ctx.supports_epub3();

        write_element! {
            writer: self.writer,
            tag: opf::ITEMREF,
            attributes: {
                // The `id` attribute takes ordering precedence over `idref`
                xml::ID         => itemref.id.as_deref().or(self.generated_spine_ids.get(itemref)),
                opf::IDREF      => itemref.idref.as_str(),
                // Note: a `linear` attribute with a value of `yes` is redundant
                opf::LINEAR where !itemref.linear => opf::NO,

                // EPUB 3 attributes
                opf::PROPERTIES where supported => itemref.properties.as_option_str(),

                ..itemref.attributes.iter_key_value(),
            }
        }
    }
}
