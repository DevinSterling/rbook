use crate::ebook::element::TextDirection;
use crate::ebook::epub::consts::{dc, opf, opf::bytes, xml};
use crate::ebook::epub::errors::EpubError;
use crate::ebook::epub::metadata::{
    EpubMetaEntryData, EpubMetaGroups, EpubMetadataData, EpubRefinementsData,
};
use crate::ebook::epub::metadata::{EpubMetaEntryKind, EpubVersion};
use crate::ebook::epub::package::EpubPackageData;
use crate::ebook::epub::parser::package::PackageParser;
use crate::ebook::epub::parser::{EpubParseConfig, EpubParserContext, EpubParserValidator};
use crate::parser::ParserResult;
use crate::parser::xml::{XmlAttributes, XmlEvent, XmlReader, XmlStartElement};
use indexmap::IndexMap;
use std::cell::Cell;
use std::collections::HashMap;

/// Stores additional information required during parsing-time.
struct TempEpubMetaEntry {
    depth: Cell<u8>,
    /// Author-specified order relative to associated metadata entries
    display_seq: usize,
    /// Absolute natural order among all metadata entries
    natural_order: usize,
    refines: Option<String>,
    temp_refinements: Vec<TempEpubMetaEntry>,
    data: EpubMetaEntryData,
}

impl TempEpubMetaEntry {
    const DEPTH_UNSET: u8 = u8::MAX;
    /// Flag to detect cycles in malformed epubs.
    const DEPTH_CALCULATION_IN_PROGRESS: u8 = u8::MAX - 1;

    fn new(kind: EpubMetaEntryKind) -> Self {
        Self {
            refines: None,
            temp_refinements: Vec::new(),
            display_seq: usize::MAX,
            natural_order: usize::MAX,
            depth: Cell::new(Self::DEPTH_UNSET),
            data: EpubMetaEntryData {
                kind,
                ..EpubMetaEntryData::default()
            },
        }
    }

    fn finish(self) -> EpubMetaEntryData {
        let mut data = self.data;

        data.refinements = EpubRefinementsData::new(
            self.temp_refinements
                .into_iter()
                .map(TempEpubMetaEntry::finish)
                .collect(),
        );
        data
    }
}

impl std::ops::Deref for TempEpubMetaEntry {
    type Target = EpubMetaEntryData;

    fn deref(&self) -> &Self::Target {
        &self.data
    }
}

impl std::ops::DerefMut for TempEpubMetaEntry {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.data
    }
}

/// When refinement `<meta>` are found, yet have no parent within `<metadata>`,
/// they are added here as their parent may reside in the `<manifest>` or `<spine>`.
pub(super) struct PendingRefinements(HashMap<String, Vec<TempEpubMetaEntry>>);

impl PendingRefinements {
    pub(super) fn new() -> Self {
        Self(HashMap::new())
    }

    pub(super) fn take_refinements(&mut self, parent_id: &str) -> Option<EpubRefinementsData> {
        self.0.remove(parent_id).map(|mut data| {
            MetadataParser::sort_by_display_sequence(&mut data);

            EpubRefinementsData::new(data.into_iter().map(TempEpubMetaEntry::finish).collect())
        })
    }

    fn insert(&mut self, mut refinement: TempEpubMetaEntry) {
        let refines = refinement.refines.take().unwrap_or_default();

        match self.0.get_mut(&refines) {
            Some(group) => group.push(refinement),
            None => {
                self.0.insert(refines, vec![refinement]);
            }
        }
    }
}

struct MetadataParser<'package, 'a> {
    ctx: &'package EpubParserContext<'a>,
    reader: &'package mut XmlReader<'a>,
    package: &'package EpubPackageData,
    pending_refinements: &'package mut PendingRefinements,
}

impl<'package, 'a> MetadataParser<'package, 'a> {
    fn new(
        ctx: &'package EpubParserContext<'a>,
        reader: &'package mut XmlReader<'a>,
        package: &'package EpubPackageData,
        pending_refinements: &'package mut PendingRefinements,
    ) -> Self {
        Self {
            ctx,
            reader,
            package,
            pending_refinements,
        }
    }

    fn parse_metadata(mut self) -> ParserResult<EpubMetadataData> {
        // EpubMeta that have an `id` attribute.
        let mut id_meta = HashMap::new();
        // EpubMeta that have a `refines` attribute but does not carry its own `id`.
        // **Identified as refining meta with a depth of 1.**
        let mut no_id_refinements = Vec::new();
        // EpubMeta not carrying an `id` and `refines` attribute.
        // **Identified as root meta elements with a depth of 0.**
        let mut no_id_generic = Vec::new();
        let mut natural_order = 0;

        while let Some((kind, el)) = self.next_entry()? {
            let (id, mut entry) = self.parse_metadata_entry(kind, &el)?;
            entry.natural_order = natural_order;
            natural_order += 1;

            if let Some(id) = id {
                id_meta.insert(id, entry);
            } else if entry.refines.is_some() {
                no_id_refinements.push(entry);
            } else {
                no_id_generic.push(entry);
            }
        }

        let groups = self.organize_metadata(id_meta, no_id_refinements, no_id_generic)?;

        if self.is_strict() {
            self.assert_metadata(&groups)?;
        }

        Ok(EpubMetadataData::new(groups))
    }

    fn parse_metadata_entry(
        &mut self,
        kind: EpubMetaEntryKind,
        el: &XmlStartElement<'_>,
    ) -> ParserResult<(Option<String>, TempEpubMetaEntry)> {
        let mut attributes = el.attributes()?;
        let mut entry = TempEpubMetaEntry::new(kind);

        // Parse the specific kind of entry
        match kind {
            EpubMetaEntryKind::DublinCore {} => self.parse_dublin_core(el, &mut entry)?,
            EpubMetaEntryKind::Meta { version } => {
                self.parse_meta(version, el, &mut attributes, &mut entry)?;
            }
            EpubMetaEntryKind::Link {} => {
                // No specialized parsing required
            }
        };

        let id = attributes.remove(xml::ID)?;

        entry.language = attributes.remove(xml::LANG)?;
        entry.refines = attributes
            .remove(opf::REFINES)?
            .map(Self::normalize_refines);
        entry.text_direction = attributes
            .remove(opf::TEXT_DIR)?
            .map_or(TextDirection::Auto, TextDirection::from);
        entry.attributes = attributes.try_into()?;

        Ok((id, entry))
    }

    // Handle `<dc:*>` elements
    fn parse_dublin_core(
        &mut self,
        el: &XmlStartElement<'_>,
        entry: &mut TempEpubMetaEntry,
    ) -> ParserResult<()> {
        let property = el.name_decoded()?.into_owned();
        let value = if !el.is_self_closing() {
            self.reader.get_element_text(el)?
        } else if !self.is_strict() {
            String::new()
        } else {
            // Dublin Core elements should not be self-closing; <dc:title/> is invalid.
            return Err(EpubError::MissingValue(property).into());
        };

        entry.property = property;
        entry.value = value;

        Ok(())
    }

    // Handle `<meta>` elements
    fn parse_meta(
        &mut self,
        structural_version: EpubVersion,
        el: &XmlStartElement<'_>,
        attributes: &mut XmlAttributes<'_>,
        entry: &mut TempEpubMetaEntry,
    ) -> ParserResult<()> {
        let is_epub2 = structural_version.is_epub2();

        // Retrieve the `<meta>` property
        let (key, err_msg) = if is_epub2 {
            (opf::NAME, "metadata > meta[*name]")
        } else {
            (opf::PROPERTY, "metadata > meta[*property]")
        };
        let property = self.require_attribute(attributes.remove(key)?, err_msg)?;

        // Retrieve the `<meta>` value
        let value = if is_epub2 {
            //////////////////////////////////
            // Epub 2 meta value extraction //
            //////////////////////////////////
            self.require_attribute(
                attributes.remove(opf::CONTENT)?,
                "metadata > meta[*content]",
            )?
        } else if !el.is_self_closing() {
            //////////////////////////////////
            // Epub 3 meta value extraction //
            //////////////////////////////////
            self.reader.get_element_text(el)?
        } else if !self.is_strict() {
            // Rare but can happen, attempt to recover if the epub is non-standard:
            // `<meta property="a" content="b" />`
            attributes.remove(opf::CONTENT)?.unwrap_or_default()
        } else {
            return Err(EpubError::MissingValue(property).into());
        };

        entry.property = property;
        entry.value = value;

        Ok(())
    }

    fn extract_kind(el: &XmlStartElement) -> ParserResult<Option<EpubMetaEntryKind>> {
        Ok(if el.is_prefix(dc::PREFIX) {
            Some(EpubMetaEntryKind::DublinCore {})
        } else if el.is_local_name(opf::META) {
            // Empty tag <meta name="" content=""/>:       EPUB 2
            // Start tag <meta name="" content=""></meta>: EPUB 2
            // Start tag <meta property="">...</meta>:     EPUB 3
            Some(EpubMetaEntryKind::Meta {
                // EPUB 2 `<meta>` does NOT use the `property` attribute
                version: if el.has_attribute(opf::PROPERTY)? {
                    EpubVersion::EPUB3
                } else {
                    EpubVersion::EPUB2
                },
            })
        } else if el.is_local_name(opf::LINK) {
            Some(EpubMetaEntryKind::Link {})
        } else {
            None
        })
    }

    fn next_entry(&mut self) -> ParserResult<Option<(EpubMetaEntryKind, XmlStartElement<'a>)>> {
        while let Some(event) = self.reader.next() {
            let el = match event? {
                XmlEvent::Start(el) => el,
                XmlEvent::End(el) if el.local_name().as_ref() == bytes::METADATA => break,
                _ => continue,
            };

            // Ignore unknown elements
            if let Some(kind) = Self::extract_kind(&el)? {
                return Ok(Some((kind, el)));
            }
        }
        Ok(None)
    }

    fn organize_metadata(
        &mut self,
        id_meta: HashMap<String, TempEpubMetaEntry>,
        no_id_refinements: Vec<TempEpubMetaEntry>,
        mut root_meta: Vec<TempEpubMetaEntry>,
    ) -> ParserResult<EpubMetaGroups> {
        let depths = Self::compute_meta_depths(id_meta, no_id_refinements)?;
        let roots = self.associate_refinements(depths)?;

        root_meta.extend(roots);
        Ok(Self::group_metadata(root_meta))
    }

    fn assert_metadata(&self, metadata: &EpubMetaGroups) -> ParserResult<()> {
        if !self.is_strict() {
            return Ok(());
        }

        // Check required metadata elements
        for (meta, error) in [
            (dc::TITLE, EpubError::MissingTitle),
            (dc::LANGUAGE, EpubError::MissingLanguage),
        ] {
            if !metadata.contains_key(meta) {
                return Err(error.into());
            }
        }

        // Check unique identifier
        let uid = &self.package.unique_identifier;
        if !metadata
            .get(dc::IDENTIFIER)
            .is_some_and(|ids| ids.iter().any(|id| id.id.as_deref() == Some(uid)))
        {
            return Err(EpubError::InvalidUniqueIdentifier(uid.to_owned()).into());
        }
        Ok(())
    }

    fn normalize_refines(mut refines: String) -> String {
        if refines.starts_with('#') {
            refines.remove(0);
        }
        refines
    }

    ////////////////////////////////////////////////////////////////////////////////
    // META REFINEMENTS AND GROUPING
    ////////////////////////////////////////////////////////////////////////////////

    fn compute_meta_depths(
        id_meta: HashMap<String, TempEpubMetaEntry>,
        no_id_refinements: Vec<TempEpubMetaEntry>,
    ) -> ParserResult<Vec<Vec<TempEpubMetaEntry>>> {
        fn compute_depth(
            id: &str,
            id_map: &HashMap<String, TempEpubMetaEntry>,
        ) -> ParserResult<u8> {
            let Some(meta) = id_map.get(id) else {
                // This may happen if the requested element resides in the manifest or spine
                return Ok(0);
            };
            let state = meta.depth.get();

            if state == TempEpubMetaEntry::DEPTH_CALCULATION_IN_PROGRESS {
                // Realistically, this *should* never happen
                return Err(EpubError::CyclicMeta(id.into()).into());
            } else if state != TempEpubMetaEntry::DEPTH_UNSET {
                // The depth has already been computed
                return Ok(state);
            }

            // Set marker
            meta.depth
                .set(TempEpubMetaEntry::DEPTH_CALCULATION_IN_PROGRESS);

            let depth = match meta.refines.as_deref() {
                Some(parent_id) => 1 + compute_depth(parent_id, id_map)?,
                None => 0,
            };

            meta.depth.set(depth);
            Ok(depth)
        }

        let mut max_depth = 0;

        // Iterate over map keys
        for id in id_meta.keys() {
            max_depth = max_depth.max(compute_depth(id, &id_meta)?);
        }

        let mut depths = (0..=max_depth + 1).map(|_| Vec::new()).collect::<Vec<_>>();

        for meta in no_id_refinements {
            let refines = meta.refines.as_deref().expect("`refines` should be Some");
            let depth = 1 + id_meta.get(refines).map_or(0, |parent| parent.depth.get());
            depths[depth as usize].push(meta);
        }
        for (id, mut data) in id_meta {
            // transfer id
            data.id.replace(id);
            depths[data.depth.get() as usize].push(data);
        }

        Ok(depths)
    }

    /// Processes metadata elements by depth to ensure the correct association order
    /// for multi-level refinement chains.
    ///
    /// Parents must not be processed before their children, or nesting will be lost.
    ///
    /// Any depth-1 children whose parent is missing are collected as *pending*
    /// (they most likely refine into the manifest or spine).
    ///
    /// Returns a tuple containing two [`Vec`]:
    /// 1. Remaining depth-0 metadata elements (the roots).
    /// 2. Orphan metadata elements with no parent yet.
    fn associate_refinements(
        &mut self,
        mut depths: Vec<Vec<TempEpubMetaEntry>>,
    ) -> ParserResult<Vec<TempEpubMetaEntry>> {
        let mut roots = Vec::new();

        // Depth 0: root meta elements
        // Depth >=1: refinement meta elements
        while let Some(refinements) = depths.pop() {
            if depths.is_empty() {
                roots = refinements;
                break;
            }

            let current_depth = depths.len();
            let parents = &mut depths[current_depth - 1];
            // All children are guaranteed to have the `refines` attribute
            for child in refinements {
                // Add child metadata to parent metadata
                let parent_id = child.refines.as_deref().expect("`refines` should be Some");

                // Find the parent metadata element. If none, malformed meta
                // The number of parents at N depth is generally small (< 10);
                // the overhead of using a hashmap is not needed.
                if let Some(parent) = parents
                    .iter_mut()
                    .find(|parent| parent.id.as_deref() == Some(parent_id))
                {
                    // Display sequence is a special case
                    if child.property == opf::DISPLAY_SEQ {
                        // If the given value is invalid, default to the lowest priority
                        parent.display_seq = child.value.parse().ok().unwrap_or(usize::MAX);
                    }

                    // Add child metadata entry to parent
                    parent.temp_refinements.push(child);
                }
                // If the parent is not found here and the current depth is `1`,
                // the parent may reside in the manifest (<item>) or spine (<itemref>).
                else if current_depth == 1 {
                    self.pending_refinements.insert(child);
                }
                // Otherwise, propagate an error.
                else if self.is_strict() {
                    return Err(EpubError::InvalidRefines(parent_id.to_owned()).into());
                }
            }
        }

        Ok(roots)
    }

    fn group_metadata(mut root_meta: Vec<TempEpubMetaEntry>) -> EpubMetaGroups {
        let mut temp_groups = IndexMap::new();

        // Natural order of metadata entries takes precedence
        root_meta.sort_unstable_by_key(|entry| entry.natural_order);

        // Group by property
        for meta in root_meta {
            if let Some(group) = temp_groups.get_mut(&meta.property) {
                group
            } else {
                // entry API is only used in this branch as it would
                // otherwise require cloning a `String` repeatedly.
                temp_groups
                    .entry(meta.property.clone())
                    .or_insert(Vec::new())
            }
            .push(meta);
        }

        // Update local meta entry order
        for group in temp_groups.values_mut() {
            Self::sort_by_display_sequence(group);
        }

        // Finalize into complete form
        temp_groups
            .into_iter()
            .map(|(key, value)| {
                (
                    key,
                    value.into_iter().map(TempEpubMetaEntry::finish).collect(),
                )
            })
            .collect()
    }

    fn sort_by_display_sequence(group: &mut [TempEpubMetaEntry]) {
        // Sort refinements as they must be ordered as well
        for meta in group.iter_mut() {
            Self::sort_by_display_sequence(&mut meta.temp_refinements);
        }

        // Sort with the display sequence taking precedence and natural order as a fallback
        group.sort_unstable_by_key(|meta| (meta.display_seq, meta.natural_order));
    }
}

impl EpubParserValidator for MetadataParser<'_, '_> {
    fn config(&self) -> &EpubParseConfig {
        self.ctx.config
    }
}

impl<'a> PackageParser<'a, '_> {
    pub(super) fn parse_metadata(&mut self) -> ParserResult<EpubMetadataData> {
        let package = self.package.as_ref().ok_or(EpubError::NoPackageFound)?;

        MetadataParser::new(self.ctx, &mut self.reader, package, &mut self.refinements)
            .parse_metadata()
    }
}
