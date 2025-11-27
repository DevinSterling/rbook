use crate::epub::open_example_epub_file_with;
use rbook::Ebook;
use rbook::ebook::element::Attributes;
use rbook::ebook::manifest::ManifestEntry;
use rbook::ebook::toc::{Toc, TocChildren, TocEntry, TocEntryKind};
use rbook::epub::EpubSettings;
use rbook::epub::metadata::EpubVersion;
use wasm_bindgen_test::wasm_bindgen_test;

#[test]
#[wasm_bindgen_test]
fn test_toc() {
    let epub = open_example_epub_file_with(EpubSettings::builder().store_all(true));
    #[rustfmt::skip]
    let expected_toc = vec![
        (
            1,                      // depth
            1,                      // order
            "EPUB/cover.xhtml",     // href
            "The Cover",            // label
            TocEntryKind::Unknown,  // kind
        ),
        (1, 2, "EPUB/c1.xhtml?q=1#start", "rbook Chapter 1", TocEntryKind::Unknown),
        (2, 3, "EPUB/c1a.xhtml", "rbook Chapter 1a", TocEntryKind::Unknown),
        (1, 4, "EPUB/c2.xhtml", "rbook Chapter 2", TocEntryKind::Unknown),
    ];
    #[rustfmt::skip]
    let expected_guide = vec![
        (1, 1, "cover.xhtml", "Cover", TocEntryKind::Cover),
        (1, 2, "../toc.xhtml", "Table of Contents", TocEntryKind::Toc),
        (1, 3, "c1.xhtml", "Start Here", TocEntryKind::BodyMatter),
    ];
    #[rustfmt::skip]
    let expected_landmarks = vec![
        (1, 1, "EPUB/cover.xhtml", "Cover", TocEntryKind::Cover),
        (1, 2, "toc.xhtml", "Table of Contents", TocEntryKind::Toc),
        (1, 3, "EPUB/c1.xhtml", "Start Here", TocEntryKind::BodyMatter),
    ];
    #[rustfmt::skip]
    let expected_variants = [
        (TocEntryKind::Toc, EpubVersion::EPUB2, &expected_toc),
        (TocEntryKind::Toc, EpubVersion::EPUB3, &expected_toc),
        (TocEntryKind::Landmarks, EpubVersion::EPUB2, &expected_guide),
        (TocEntryKind::Landmarks, EpubVersion::EPUB3, &expected_landmarks),
    ];

    for (kind, version, expected) in expected_variants {
        let root = epub.toc().by_kind_version(&kind, version).unwrap();
        let contents = root.children().flatten().collect::<Vec<_>>();

        assert!(root.is_root());
        // The root must contain children
        assert!(!root.children().is_empty());
        assert_eq!(&kind, root.kind());
        assert_eq!(expected.len(), contents.len());

        for (entry, (depth, order, href, label, kind)) in contents.into_iter().zip(expected) {
            assert_eq!(depth, &entry.depth());
            assert_eq!(order, &entry.order());
            assert_eq!(href, &entry.href_raw().unwrap().as_str());
            assert_eq!(label, &entry.label());
            assert_eq!(kind, entry.kind());

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
            EpubSettings::builder()
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
