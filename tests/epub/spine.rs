use crate::epub::util::{open_example_epub_file, open_example_epub_file_with};
use rbook::Ebook;
use rbook::ebook::manifest::ManifestEntry;
use rbook::ebook::spine::{PageDirection, Spine, SpineEntry};
use rbook::epub::EpubOpenOptions;
use wasm_bindgen_test::wasm_bindgen_test;

#[test]
#[wasm_bindgen_test]
fn test_spine() {
    let epub = open_example_epub_file();
    let spine = epub.spine();
    let entries = spine.entries().collect::<Vec<_>>();

    assert_eq!(PageDirection::LeftToRight, spine.page_direction());
    assert_eq!(spine.len(), entries.len());
    assert_eq!(EXPECTED_SPINE.len(), entries.len());
    assert!(!spine.is_empty());

    for (entry, expected) in entries.into_iter().zip(EXPECTED_SPINE) {
        let entry_properties = entry.properties().iter().collect::<Vec<_>>();

        assert_eq!(expected.id, entry.id());
        assert_eq!(expected.is_linear, entry.is_linear());
        assert_eq!(expected.idref, entry.idref());
        assert_eq!(expected.properties, entry_properties);

        for property in expected.properties {
            assert!(entry.properties().has_property(property))
        }

        let manifest_entry = entry.manifest_entry().unwrap();

        assert_eq!(expected.idref, manifest_entry.id());
        assert_eq!(entry.resource().unwrap(), manifest_entry.resource());
    }
}

#[test]
#[wasm_bindgen_test]
fn test_skip_spine() {
    let epub = open_example_epub_file_with(EpubOpenOptions::new().skip_spine(true));
    let spine = epub.spine();

    assert_eq!(PageDirection::Default, spine.page_direction());
    assert_eq!(0, spine.len());
    assert!(spine.is_empty());
    assert!(spine.entries().next().is_none());
    assert!(spine.by_order(0).is_none());
    assert!(spine.by_id("spine-toc").is_none());
    assert!(spine.by_idref("c1a").is_none());
}

#[test]
#[wasm_bindgen_test]
fn test_reader_skip_spine() {
    use rbook::reader::{Reader, SynchronousReader};

    let epub = open_example_epub_file_with(EpubOpenOptions::new().skip_spine(true));
    let mut reader = epub.reader();

    assert_eq!(0, reader.len());
    assert_eq!(0, reader.remaining());
    assert!(reader.read_next().is_none())
}

/////////////////////////////////////////////////
// TEST DATA
/////////////////////////////////////////////////

pub struct SpineTestData<'a> {
    pub id: Option<&'a str>,
    pub idref: &'a str,
    pub is_linear: bool,
    pub properties: &'a [&'a str],
}

impl<'a> SpineTestData<'a> {
    const fn new(
        id: Option<&'a str>,
        idref: &'a str,
        is_linear: bool,
        properties: &'a [&'a str],
    ) -> Self {
        Self {
            id,
            idref,
            is_linear,
            properties,
        }
    }
}

// Reference: example.epub / example_epub
#[rustfmt::skip]
pub const EXPECTED_SPINE: &[SpineTestData] = &[
    SpineTestData::new(None, "cover", false, &[]),
    SpineTestData::new(Some("spine-toc"), "toc", true, &[]),
    SpineTestData::new(None, "c1", true, &["page-spread-left"]),
    SpineTestData::new(Some("supplementary"), "c1a", false, &["rbook:prop", "rbook:prop2"]),
    SpineTestData::new(None, "c2", true, &[]),
];
