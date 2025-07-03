use crate::ebook::element::TextDirection;
use crate::ebook::epub::consts::{self, bytes};
use crate::ebook::epub::errors::EpubFormatError;
use crate::ebook::epub::metadata::{
    EpubMetaEntryData, EpubMetaGroups, EpubMetadataData, EpubRefinementsData,
};
use crate::ebook::epub::parser::EpubParser;
use crate::ebook::epub::parser::package::PackageData;
use crate::epub::parser::package::PackageContext;
use crate::parser::ParserResult;
use crate::parser::xml::{ByteReader, XmlElement, XmlReader};
use crate::util::sync::Shared;
use quick_xml::events::{BytesStart, Event};
use std::cell::Cell;
use std::collections::{HashMap, HashSet};
use std::ops::Deref;

struct IdMetaWithDepth {
    depth: Cell<u8>,
    meta: EpubMetaEntryData,
}

impl IdMetaWithDepth {
    /// Flag to detect cycles in malformed epubs.
    const IN_PROGRESS: u8 = u8::MAX - 1;
    const UNSET: u8 = u8::MAX;

    fn new(meta: EpubMetaEntryData) -> Self {
        Self {
            depth: Cell::new(Self::UNSET),
            meta,
        }
    }
}

impl Deref for IdMetaWithDepth {
    type Target = EpubMetaEntryData;

    fn deref(&self) -> &Self::Target {
        &self.meta
    }
}

/// When refinement `<meta>` are found, yet have no parent within `<metadata>`,
/// they are added here as their parent may reside in the `<manifest>` or `<spine>`.
pub(crate) struct PendingRefinements(HashMap<String, Vec<EpubMetaEntryData>>);

impl PendingRefinements {
    fn new(refinements: Vec<EpubMetaEntryData>) -> Self {
        let mut map: HashMap<String, Vec<EpubMetaEntryData>> = HashMap::new();

        for mut refinement in refinements {
            if let Some(group) = map.get_mut(refinement.refines.as_deref().unwrap_or_default()) {
                group.push(refinement);
            } else {
                let refines = refinement.refines.take().unwrap_or_default();
                map.insert(refines, vec![refinement]);
            }
        }
        Self(map)
    }

    pub(super) fn empty() -> Self {
        PendingRefinements(HashMap::new())
    }

    pub(super) fn take_refinements(&mut self, parent_id: &str) -> Option<EpubRefinementsData> {
        self.0
            .remove_entry(parent_id)
            .map(|(refines, mut refinements)| {
                // There is at least one entry
                refinements[0].refines.replace(refines);
                EpubRefinementsData::new(refinements)
            })
    }
}

impl<'a> EpubParser<'a> {
    pub(super) fn parse_metadata(
        &mut self,
        ctx: &mut PackageContext,
        package: PackageData,
    ) -> ParserResult<EpubMetadataData> {
        // EpubMeta that have an `id` attribute.
        let mut id_meta = HashMap::new();
        // EpubMeta that have a `refines` attribute but does not carry its own `id`.
        // **Identified as refining meta with a depth of 1.**
        let mut no_id_refinements = Vec::new();
        // EpubMeta not carrying an `id` and `refines` attribute.
        // **Identified as root meta elements with a depth of 0.**
        let mut no_id_generic = Vec::new();

        let mut natural_order = 0;

        while let Some((el, has_inner_content)) = Self::meta_handler(&mut ctx.reader)? {
            let mut attributes = el.bytes_attributes();

            // Retrieve the required property name of the meta
            let property = if el.is_local_name(consts::META) {
                self.assert_optional(
                    attributes.take_attribute_value_any([consts::PROPERTY, consts::NAME])?,
                    "metadata > meta[*property (EPUB 3) or *name (EPUB 2)]",
                )?
            } else {
                String::from_utf8(el.name().as_ref().to_vec())?
            };

            // Retrieve the text content if there is a corresponding end tag.
            // An empty element is most likely from the EPUB2 standard.
            let value = if has_inner_content {
                ctx.reader.get_text_simple(&el)?
            } else {
                self.assert_optional(
                    attributes.take_attribute_value(consts::CONTENT)?,
                    "metadata > meta[*content]",
                )?
            };

            // Optional attributes
            let id = attributes.take_attribute_value(consts::ID)?;
            let refines = attributes
                .take_attribute_value(consts::REFINES)?
                .map(Self::normalize_refines);

            // Attributes where if not specified, inherit from the package
            let language = attributes
                .take_attribute_value(consts::LANG)?
                .map(Shared::new)
                .or_else(|| package.xml_lang.clone());
            let text_direction = attributes
                .take_attribute_value(consts::DIR)?
                .map_or(package.dir, TextDirection::from);

            let meta = EpubMetaEntryData {
                order: natural_order,
                attributes: attributes.try_into()?,
                refines,
                property,
                value,
                language,
                text_direction,
                ..Default::default()
            };

            natural_order += 1;

            if let Some(id) = id {
                id_meta.insert(id, IdMetaWithDepth::new(meta));
            } else if meta.refines.is_some() {
                no_id_refinements.push(meta);
            } else {
                no_id_generic.push(meta);
            }
        }

        if self.settings.strict && !id_meta.contains_key(&package.unique_id) {
            return Err(EpubFormatError::MissingMeta(format!(
                "No identifier found with id: `{}`",
                package.unique_id
            ))
            .into());
        }

        let (groups, pending) =
            self.finalize_metadata(id_meta, no_id_refinements, no_id_generic)?;

        ctx.refinements = pending;

        Ok(EpubMetadataData::new(
            package.unique_id,
            package.version,
            groups,
        ))
    }

    fn meta_handler<'b>(
        reader: &mut ByteReader<'b>,
    ) -> ParserResult<Option<(BytesStart<'b>, bool)>> {
        fn is_meta_element(el: &BytesStart) -> bool {
            el.is_local_name(consts::META) || el.is_prefix(consts::DC_NAMESPACE)
        }

        while let Some(event) = reader.next() {
            match event? {
                // Meta/Dublin Core meta elements
                Event::Start(el) if is_meta_element(&el) => {
                    return Ok(Some((el, true)));
                }
                Event::Empty(el) if is_meta_element(&el) => {
                    return Ok(Some((el, false)));
                }
                Event::End(el) if el.local_name().as_ref() == bytes::METADATA => {
                    break;
                }
                _ => {}
            }
        }
        Ok(None)
    }

    fn finalize_metadata(
        &mut self,
        id_meta: HashMap<String, IdMetaWithDepth>,
        no_id_refinements: Vec<EpubMetaEntryData>,
        mut root_meta: Vec<EpubMetaEntryData>,
    ) -> ParserResult<(EpubMetaGroups, PendingRefinements)> {
        let depths = Self::compute_meta_depths(id_meta, no_id_refinements)?;
        let (roots, pending) = Self::associate_refinements(depths)?;

        root_meta.extend(roots);

        let mut grouped_meta = Self::group_metadata(root_meta)?;

        if self.settings.strict {
            Self::assert_metadata(&grouped_meta)?;
        }

        // Lastly, update meta element order
        Self::adjust_display_sequence(&mut grouped_meta);

        Ok((grouped_meta, pending))
    }

    fn assert_metadata(metadata: &EpubMetaGroups) -> ParserResult<()> {
        for meta in [consts::IDENTIFIER, consts::TITLE, consts::LANGUAGE] {
            if !metadata.contains_key(meta) {
                return Err(EpubFormatError::MissingMeta(meta.to_owned()).into());
            }
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
        id_meta: HashMap<String, IdMetaWithDepth>,
        no_id_refinements: Vec<EpubMetaEntryData>,
    ) -> ParserResult<Vec<Vec<EpubMetaEntryData>>> {
        fn compute_depth(id: &str, id_map: &HashMap<String, IdMetaWithDepth>) -> ParserResult<u8> {
            let meta = match id_map.get(id) {
                Some(meta) => meta,
                // This may happen if the requested element resides in the manifest or spine
                None => return Ok(0),
            };
            let state = meta.depth.get();

            if state == IdMetaWithDepth::IN_PROGRESS {
                return Err(EpubFormatError::CyclicMeta(id.into()).into());
            } else if state != IdMetaWithDepth::UNSET {
                // The depth has already been computed
                return Ok(state);
            }

            // Set marker
            meta.depth.set(IdMetaWithDepth::IN_PROGRESS);

            let depth = match meta.refines.as_deref() {
                Some(parent_id) => 1 + compute_depth(parent_id, id_map)?,
                None => 0,
            };

            meta.depth.set(depth);
            Ok(depth)
        }

        let mut max_depth = 0;

        // Iterate over map keys
        for (id, _) in id_meta.iter() {
            max_depth = max_depth.max(compute_depth(id, &id_meta)?);
        }

        let mut depths = (0..=max_depth + 1).map(|_| Vec::new()).collect::<Vec<_>>();

        for meta in no_id_refinements {
            let refines = meta.refines.as_deref().expect("`refines` should be Some");
            let depth = 1 + id_meta.get(refines).map_or(0, |parent| parent.depth.get());
            depths[depth as usize].push(meta);
        }
        for (id, IdMetaWithDepth { depth, mut meta }) in id_meta {
            // transfer id
            meta.id.replace(id);
            depths[depth.get() as usize].push(meta);
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
        mut depths: Vec<Vec<EpubMetaEntryData>>,
    ) -> ParserResult<(Vec<EpubMetaEntryData>, PendingRefinements)> {
        let mut roots = None;
        let mut pending = Vec::new();

        // Depth 0: root meta elements
        // Depth >=1: refinement meta elements
        while let Some(refinements) = depths.pop() {
            if depths.is_empty() {
                roots.replace(refinements);
                break;
            }

            let current_depth = depths.len();
            let parents = &mut depths[current_depth - 1];
            // All children are guaranteed to have the `refines` attribute
            for child in refinements {
                // Add child metadata to parent metadata
                let parent_id = child.refines.as_deref().expect("`refines` should be Some");

                // Find the parent metadata element. If none, malformed meta
                // The numbers of parents at N depth is generally small (< 10);
                // the overhead of using a hashmap is not needed.
                if let Some(parent) = parents
                    .iter_mut()
                    .find(|parent| parent.id.as_deref() == Some(parent_id))
                {
                    // Add child metadata entry to parent
                    parent.refinements.push(child);
                }
                // If the parent is not found here and the current depth is `1`,
                // the parent may reside in the manifest (<item>) or spine (<itemref>).
                else if current_depth == 1 {
                    pending.push(child);
                }
                // Otherwise, propagate an error.
                else {
                    return Err(EpubFormatError::MissingMeta(format!(
                        "refinement <meta> referencing a non-existent id=`{parent_id}`"
                    ))
                    .into());
                }
            }
        }

        // Note that `roots` may be empty (None) if `strict` mode
        // is disabled and an epub contains absolutely no metadata.
        Ok((roots.unwrap_or_default(), PendingRefinements::new(pending)))
    }

    fn group_metadata(root_meta: Vec<EpubMetaEntryData>) -> ParserResult<EpubMetaGroups> {
        let mut meta_groups = HashMap::new();

        // Group by property
        for meta in root_meta {
            // HashMap::entry(&mut self, key) is not used here as it would require
            // cloning a `String` repeatedly. This is less expensive:
            if let Some(group) = meta_groups.get_mut(&meta.property) {
                group
            } else {
                meta_groups.insert(meta.property.clone(), Vec::new());
                // Calling `unwrap` is safe here as `meta.property` was just added as a key
                meta_groups.get_mut(&meta.property).unwrap()
            }
            .push(meta);
        }

        Ok(meta_groups)
    }

    ////////////////////////////////////////////////////////////////////////////////
    // DISPLAY SEQUENCE SORTING
    ////////////////////////////////////////////////////////////////////////////////

    fn adjust_display_sequence(metadata: &mut EpubMetaGroups) {
        for group in metadata.values_mut() {
            Self::sort_by_display_sequence(group);
        }
    }

    fn sort_by_display_sequence(group: &mut [EpubMetaEntryData]) {
        if group.is_empty() {
            return;
        }

        // First sort the vec as it currently is NOT sorted by natural order
        group.sort_unstable_by_key(|meta| meta.order);

        // Identify the indices the author explicitly wants reserved
        let mut reserved_indices = HashSet::new();
        for meta in group.iter_mut() {
            // Apply this method recursively
            Self::sort_by_display_sequence(&mut meta.refinements);

            meta.order = meta
                .refinements
                .by_refinement(consts::DISPLAY_SEQ)
                .and_then(|refinement| refinement.value.parse().ok())
                // Typically EPUB `display-seq` starts from `1` (1-based index).
                // However, to ensure consistency with the rest of rbook API, use 0-based.
                .map(|mut seq: usize| {
                    seq = seq.saturating_sub(1);
                    // If there is a duplicate display-seq value;
                    // increment by 1 until a slot is free.
                    while !reserved_indices.insert(meta.order) {
                        seq += 1;
                    }
                    seq
                })
                // An order of MAX indicates no explicitly set display-seq by the author
                .unwrap_or(usize::MAX);
        }

        if reserved_indices.len() < group.len() {
            let mut insert_at = 0;
            // Assign meta with no `display-seq` an unreserved index
            for meta in group.iter_mut() {
                if meta.order == usize::MAX {
                    // find the next index that's not reserved
                    while reserved_indices.contains(&insert_at) {
                        insert_at += 1;
                    }
                    meta.order = insert_at;
                    insert_at += 1;
                }
            }
        }

        // Sort
        group.sort_unstable_by_key(|meta| meta.order);
    }
}
