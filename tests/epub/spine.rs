use crate::epub::open_example_epub_file;
use rbook::Ebook;
use rbook::ebook::manifest::ManifestEntry;
use rbook::ebook::spine::{PageDirection, Spine, SpineEntry};
use wasm_bindgen_test::wasm_bindgen_test;

#[test]
#[wasm_bindgen_test]
fn test_spine() {
    let epub = open_example_epub_file();
    let spine = epub.spine();
    let entries = spine.entries().collect::<Vec<_>>();
    #[rustfmt::skip]
    let expected = [
        (
            None,    // id
            "cover", // idref
            false,   // is_linear
            vec![],  // properties
        ),
        (Some("spine-toc"), "toc", true, vec![]),
        (None, "c1", true, vec!["page-spread-left"]),
        (Some("supplementary"), "c1a", false, vec!["rbook:prop", "rbook:prop2"]),
        (None, "c2", true, vec![]),
    ];

    assert_eq!(PageDirection::LeftToRight, spine.page_direction());
    assert_eq!(spine.len(), entries.len());
    assert_eq!(expected.len(), entries.len());

    for (entry, (id, idref, is_linear, properties)) in entries.into_iter().zip(expected) {
        let entry_properties = entry.properties().iter().collect::<Vec<_>>();

        assert_eq!(id, entry.id());
        assert_eq!(is_linear, entry.is_linear());
        assert_eq!(idref, entry.idref());
        assert_eq!(properties, entry_properties);

        for property in properties {
            assert!(entry.properties().has_property(property))
        }

        let manifest_entry = entry.manifest_entry().unwrap();

        assert_eq!(idref, manifest_entry.id());
        assert_eq!(entry.resource().unwrap(), manifest_entry.resource());
    }
}
