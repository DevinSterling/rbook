mod ncx;
mod xhtml;

use crate::ebook::epub::consts::xml;
use crate::ebook::epub::errors::EpubError;
use crate::ebook::epub::metadata::EpubVersion;
use crate::ebook::epub::parser::{EpubParseConfig, EpubParser};
use crate::ebook::epub::toc::{EpubTocData, EpubTocEntryData, EpubTocKey, TocGroups};
use crate::ebook::errors::EbookResult;
use crate::ebook::toc::TocEntryKind;
use crate::epub::parser::{EpubParserContext, EpubParserValidator};
use crate::parser::ParserResult;
use crate::parser::xml::{XmlAttributes, XmlReader};
use crate::util::uri::UriResolver;

pub(super) struct TocLocation {
    /// Absolute ToC href location
    location: String,
    /// EPUB version associated with the location:
    /// - `.ncx` = EPUB 2
    /// - `.xhtml` = EPUB 3
    version: EpubVersion,
}

impl TocLocation {
    pub(super) fn new(location: String, version: EpubVersion) -> Self {
        Self { location, version }
    }
}

struct TocParser<'a> {
    ctx: EpubParserContext<'a>,
    reader: XmlReader<'a>,
    /// Resolver to turn HREFs within the toc file from relative to absolute
    resolver: UriResolver<'a>,
    /// Stack to keep track of latest nav element entry
    stack: Vec<EpubTocEntryData>,
    /// Container for all root toc entries
    groups: TocGroups,
}

impl<'a> TocParser<'a> {
    pub(super) fn new(ctx: EpubParserContext<'a>, data: &'a [u8], toc_location: &'a str) -> Self {
        Self {
            reader: XmlReader::from_bytes(ctx.is_strict(), data),
            resolver: UriResolver::parent_of(toc_location),
            stack: Vec::new(),
            groups: indexmap::IndexMap::new(),
            ctx,
        }
    }

    fn new_toc_entry(attributes: &mut XmlAttributes<'_>) -> ParserResult<EpubTocEntryData> {
        Ok(EpubTocEntryData {
            id: attributes.remove(xml::ID)?,
            ..EpubTocEntryData::default()
        })
    }

    fn handle_pop(&mut self, version: EpubVersion) {
        let Some(nav_entry) = self.stack.pop() else {
            return;
        };

        // The nav element has a parent
        if let Some(nav_parent) = self.stack.last_mut() {
            nav_parent.children.push(nav_entry);
        } else {
            // The nav element does not have a parent; the root
            let toc_kind = nav_entry.kind.clone().unwrap_or_default();
            self.groups
                .insert(EpubTocKey::new(toc_kind, version), nav_entry);
        }
    }
}

impl EpubParserValidator for TocParser<'_> {
    fn config(&self) -> &EpubParseConfig {
        self.ctx.config
    }
}

impl EpubParser<'_> {
    // `EbookResult` is preferred here over `ParserResult`
    // due to the use of reading from an archive.
    pub(super) fn parse_tocs(
        &self,
        tocs: Vec<TocLocation>,
        toc: &mut EpubTocData,
    ) -> EbookResult<()> {
        for TocLocation { location, version } in tocs {
            let content_toc = self.read_resource(location.as_str())?;
            toc.extend(self.parse_toc(version, &location, &content_toc)?);
        }

        self.remove_guide_if_redundant(toc);

        // Apply preference
        toc.preferred_version = self.config().preferred_toc;

        Ok(())
    }

    fn parse_toc(
        &self,
        version: EpubVersion,
        toc_location: &str,
        data: &[u8],
    ) -> ParserResult<EpubTocData> {
        let parser = TocParser::new(self.ctx, data, toc_location);
        let toc_groups = match version {
            EpubVersion::Epub2(_) => parser.parse_epub2_ncx()?,
            _ => parser.parse_epub3_nav()?,
        };

        // Perform assertions
        if self.is_strict() {
            self.assert_toc(version, &toc_groups)?;
        }

        Ok(EpubTocData::new(toc_groups))
    }

    fn assert_toc(&self, version: EpubVersion, map: &TocGroups) -> ParserResult<()> {
        // Check if the epub contains a main table of contents
        if map.contains_key(&(TocEntryKind::Toc.as_str(), version.as_major())) {
            Ok(())
        } else {
            Err(EpubError::NoTocFound.into())
        }
    }

    /// Enforces [`retain_variants`](crate::epub::EpubOpenOptions::retain_variants).
    /// (EPUB 2 landmarks (guide) is a special case since it is parsed eagerly)
    ///
    /// If not storing all versions, remove redundant ones
    /// to avoid confusing end-users with multiple parsed versions.
    ///
    /// **Removes the legacy EPUB 2 guide if EPUB 3 landmarks are present and preferred.**
    fn remove_guide_if_redundant(&self, toc: &mut EpubTocData) {
        // Removing the guide only matters if the preferred version is not EPUB 2
        if self.config().retain_variants || self.config().preferred_toc == EpubVersion::EPUB2 {
            return;
        }

        let entries = &mut toc.entries;
        let landmarks = TocEntryKind::Landmarks.as_str();
        let epub2_key = (landmarks, EpubVersion::EPUB2);
        let epub3_key = (landmarks, EpubVersion::EPUB3);

        // Only perform removal if both versions exists
        if entries.contains_key(&epub2_key) && entries.contains_key(&epub3_key) {
            entries.shift_remove(&epub2_key);
        }
    }
}
