use crate::epub::util::open_example_epub_file_with;
use rbook::ebook::element::Attributes;
use rbook::ebook::manifest::ManifestEntry;
use rbook::ebook::toc::{Toc, TocChildren, TocEntry, TocEntryKind};
use rbook::epub::metadata::EpubVersion;
use rbook::{Ebook, Epub};
use wasm_bindgen_test::wasm_bindgen_test;

#[test]
#[wasm_bindgen_test]
fn test_toc() {
    let epub = open_example_epub_file_with(Epub::options().store_all(true));

    for TocVariantData {
        kind,
        version,
        test_data,
    } in EXPECTED_VARIANTS
    {
        let root = epub.toc().by_kind_version(kind, *version).unwrap();
        let contents = root.children().flatten().collect::<Vec<_>>();

        assert!(root.is_root());
        // The root must contain children
        assert!(!root.children().is_empty());
        assert_eq!(kind, root.kind());
        assert_eq!(test_data.len(), contents.len());

        for (entry, expected) in contents.into_iter().zip(*test_data) {
            assert_eq!(expected.depth, entry.depth());
            assert_eq!(expected.order, entry.order());
            assert_eq!(expected.href, entry.href_raw().unwrap().as_str());
            assert_eq!(expected.label, entry.label());
            assert_eq!(&expected.kind, entry.kind());

            let manifest_entry = entry.manifest_entry().unwrap();
            assert_eq!(entry.href().unwrap().path(), manifest_entry.href());
            // Resources must be identical
            assert_eq!(entry.resource().unwrap(), manifest_entry.resource());
        }
    }
}

#[test]
#[wasm_bindgen_test]
fn test_preference() {
    fn get_test_flag(attributes: Attributes<'_>) -> &str {
        attributes.by_name("rbook:test").unwrap().value()
    }
    let versions = [
        (EpubVersion::EPUB2, "epub2-feature"),
        (EpubVersion::EPUB3, "epub3-feature"),
    ];

    for (version, integrity_check) in versions {
        let epub = open_example_epub_file_with(
            Epub::options()
                .preferred_toc(version)
                .preferred_landmarks(version),
        );
        let toc = epub.toc();
        let toc_root = toc.contents().unwrap();
        let landmarks_root = toc.landmarks().unwrap();

        assert_eq!(
            toc_root,
            toc.by_kind_version(TocEntryKind::Toc, version).unwrap()
        );
        assert_eq!(
            landmarks_root,
            toc.by_kind_version(TocEntryKind::Landmarks, version)
                .unwrap()
        );

        // Check if the provided root is actually the intended one via a flag.
        assert_eq!(integrity_check, get_test_flag(toc_root.attributes()));
        assert_eq!(integrity_check, get_test_flag(landmarks_root.attributes()));
    }
}

#[test]
#[wasm_bindgen_test]
fn test_skip_toc() {
    use rbook::epub::EpubOpenOptions;

    let epub = open_example_epub_file_with(EpubOpenOptions::new().skip_toc(true));
    let toc = epub.toc();

    assert!(toc.contents().is_none());
    assert!(toc.landmarks().is_none());
    assert!(toc.page_list().is_none());
    assert!(toc.kinds().next().is_none());

    for kind in [
        TocEntryKind::Toc,
        TocEntryKind::Landmarks,
        TocEntryKind::PageList,
    ] {
        for version in [EpubVersion::EPUB2, EpubVersion::EPUB3] {
            assert!(toc.by_kind_version(&kind, version).is_none());
        }
        assert!(toc.by_kind(kind).is_none());
    }
}

/////////////////////////////////////////////////
// TEST DATA
/////////////////////////////////////////////////

pub struct TocVariantData<'a> {
    pub kind: TocEntryKind<'a>,
    pub version: EpubVersion,
    pub test_data: &'a [TocTestData<'a>],
}

impl<'a> TocVariantData<'a> {
    const fn new(
        kind: TocEntryKind<'a>,
        version: EpubVersion,
        test_data: &'a [TocTestData<'a>],
    ) -> Self {
        Self {
            kind,
            version,
            test_data,
        }
    }
}

pub struct TocTestData<'a> {
    pub depth: usize,
    pub order: usize,
    pub href: &'a str,
    pub label: &'a str,
    pub kind: TocEntryKind<'a>,
}

impl<'a> TocTestData<'a> {
    const fn new(
        depth: usize,
        order: usize,
        href: &'a str,
        label: &'a str,
        kind: TocEntryKind<'a>,
    ) -> Self {
        Self {
            depth,
            order,
            href,
            label,
            kind,
        }
    }
}

// Reference: example.epub / example_epub
#[rustfmt::skip]
pub const EXPECTED_VARIANTS: &[TocVariantData] = &[
    TocVariantData::new(TocEntryKind::Toc, EpubVersion::EPUB2, EXPECTED_TOC),
    TocVariantData::new(TocEntryKind::Toc, EpubVersion::EPUB3, EXPECTED_TOC),
    TocVariantData::new(TocEntryKind::Landmarks, EpubVersion::EPUB2, EXPECTED_GUIDE),
    TocVariantData::new(TocEntryKind::Landmarks, EpubVersion::EPUB3, EXPECTED_LANDMARKS),
];
#[rustfmt::skip]
pub const EXPECTED_TOC: &[TocTestData] = &[
    TocTestData::new(1, 1, "EPUB/cover.xhtml", "The Cover", TocEntryKind::Unknown),
    TocTestData::new(1, 2, "EPUB/c1.xhtml?q=1#start", "rbook Chapter 1", TocEntryKind::Unknown),
    TocTestData::new(2, 3, "EPUB/c1a.xhtml", "rbook Chapter 1a", TocEntryKind::Unknown),
    TocTestData::new(1, 4, "EPUB/c2.xhtml", "rbook Chapter 2", TocEntryKind::Unknown),
];
#[rustfmt::skip]
pub const EXPECTED_GUIDE: &[TocTestData] = &[
    TocTestData::new(1, 1, "cover.xhtml", "Cover", TocEntryKind::Cover),
    TocTestData::new(1, 2, "../toc.xhtml", "Table of Contents", TocEntryKind::Toc),
    TocTestData::new(1, 3, "c1.xhtml", "Start Here", TocEntryKind::BodyMatter),
];
#[rustfmt::skip]
pub const EXPECTED_LANDMARKS: &[TocTestData] = &[
    TocTestData::new(1, 1, "EPUB/cover.xhtml", "Cover", TocEntryKind::Cover),
    TocTestData::new(1, 2, "toc.xhtml", "Table of Contents", TocEntryKind::Toc),
    TocTestData::new(1, 3, "EPUB/c1.xhtml", "Start Here", TocEntryKind::BodyMatter),
];
