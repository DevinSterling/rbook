use crate::ebook::epub::EpubVersion;
use crate::ebook::epub::consts::{self, bytes};
use crate::ebook::epub::errors::EpubFormatError;
use crate::ebook::epub::manifest::{InternalEpubManifest, InternalEpubManifestEntry};
use crate::ebook::epub::parser::EpubParser;
use crate::ebook::epub::parser::package::TocLocation;
use crate::epub::parser::package::PackageContext;
use crate::parser::ParserResult;
use crate::parser::xml::XmlElement;
use std::collections::HashMap;

impl EpubParser<'_> {
    pub(super) fn parse_manifest(
        &self,
        ctx: &mut PackageContext,
    ) -> ParserResult<InternalEpubManifest> {
        let mut entries = HashMap::new();

        while let Some(el) = Self::simple_handler(&mut ctx.reader, bytes::MANIFEST, bytes::ITEM)? {
            let mut attributes = el.bytes_attributes();

            // Required fields
            let id = self.assert_option(
                attributes.take_attribute_value(consts::ID)?,
                "manifest > item[*id]",
            )?;
            let (href, href_raw) = self.assert_option(
                attributes
                    .take_attribute_value(consts::HREF)?
                    .map(|href_raw| (ctx.resolver.resolve(&href_raw), href_raw)),
                "manifest > item[*href]",
            )?;
            let mut media_type = self.assert_option(
                attributes.take_attribute_value(consts::MEDIA_TYPE)?,
                "manifest > item[*media_type]",
            )?;

            // Optional fields
            let media_overlay = attributes.take_attribute_value(consts::MEDIA_OVERLAY)?;
            let fallback = attributes.take_attribute_value(consts::FALLBACK)?;
            let properties = attributes.take_attribute_value(consts::PROPERTIES)?.into();
            let refinements = ctx.refinements.take_refinements(&id).unwrap_or_default();

            // Set media_type to lowercase to enforce uniformity.
            media_type.make_ascii_lowercase();
            entries.insert(
                id,
                InternalEpubManifestEntry {
                    attributes: attributes.try_into()?,
                    refinements,
                    href,
                    href_raw,
                    media_type,
                    fallback,
                    media_overlay,
                    properties,
                },
            );
        }

        Ok(InternalEpubManifest::new(entries))
    }

    pub(super) fn get_toc_hrefs(
        &self,
        manifest: &InternalEpubManifest,
    ) -> ParserResult<Vec<TocLocation>> {
        let mut hrefs = Vec::new();
        let mut formats = vec![
            // 0: Epub version associated with the format
            // 1: Target attribute key
            // 2: Target attribute value: `get_attr(key) == target`
            (EpubVersion::EPUB3, consts::PROPERTIES, consts::NAV_PROPERTY),
            (EpubVersion::EPUB2, consts::MEDIA_TYPE, consts::NCX_TYPE),
        ];
        let mut remove_found_format = None;

        for (_, entry) in manifest.iter() {
            for (i, (version, type_key, target_type)) in formats.iter_mut().enumerate() {
                // Retrieve the href of a corresponding `nav` or `ncx` item element
                let location = TocLocation {
                    href: match Self::get_toc_href(entry, type_key, target_type) {
                        Some(href) => href,
                        // Attempt to retrieve the href with the next format
                        None => continue,
                    },
                    version: *version,
                };

                // Exit early condition
                if !self.config.store_all && &self.config.preferred_toc == version {
                    return Ok(vec![location]);
                }
                hrefs.push(location);
                remove_found_format.replace(i);
                break;
            }
            if let Some(remove_at_index) = remove_found_format.take() {
                formats.swap_remove(remove_at_index);
            }
            // When all formats are found
            if formats.is_empty() {
                break;
            }
        }
        if self.config.strict && hrefs.is_empty() {
            Err(EpubFormatError::NoTocReference.into())
        } else {
            Ok(hrefs)
        }
    }

    fn get_toc_href(
        entry: &InternalEpubManifestEntry,
        type_key: &str,
        target_type: &str,
    ) -> Option<String> {
        if type_key == consts::PROPERTIES {
            // EPUB 3 ONLY: This type of attribute value is a collection of properties
            // separated by whitespace, as a result, split and search for the target
            entry.properties.has_property(target_type)
        } else {
            entry.media_type == target_type
        }
        .then(|| entry.href.clone())
    }
}
