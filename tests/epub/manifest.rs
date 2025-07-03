use crate::epub::open_example_epub_file;
use rbook::Ebook;
use rbook::ebook::manifest::{Manifest, ManifestEntry};
use rbook::ebook::metadata::MetaEntry;
use rbook::ebook::resource::ResourceKey;
use wasm_bindgen_test::wasm_bindgen_test;

#[test]
#[wasm_bindgen_test]
fn test_manifest() {
    let epub = open_example_epub_file();
    let manifest = epub.manifest();
    let mut entries = manifest.entries().collect::<Vec<_>>();
    // sort by `id` as entries are in arbitrary order
    entries.sort_by_key(|entry| entry.id());

    #[rustfmt::skip]
    let expected = [
        (
            "c1",                    // id
            "/EPUB/c1.xhtml",        // href (resolved)
            "c1.xhtml",              // href (original/raw)
            "application/xhtml+xml", // media-type
            Some("c1_audio"),        // media-overlay
            None,                    // fallback
            vec![]                   // properties
        ),
        ("c1_audio", "/EPUB/overlay/chapter1_audio.smil", "overlay/chapter1_audio.smil", "application/smil+xml", None, None, vec![]),
        ("c1a", "/EPUB/c1a.xhtml", "c1a.xhtml", "application/xhtml+xml", None, None, vec![]),
        ("c2", "/EPUB/c2.xhtml", "c2.xhtml", "application/xhtml+xml", Some("c2_audio"), None, vec![]),
        ("c2_audio", "/EPUB/overlay/chapter2_audio.smil", "overlay/chapter2_audio.smil", "application/smil+xml", None, None, vec![]),
        ("cover", "/EPUB/cover.xhtml", "cover.xhtml", "application/xhtml+xml", None, None, vec![]),
        ("cover-image1", "/EPUB/img/cover.webm", "img/cover.webm", "image/webm", None, Some("cover-image2"), vec!["cover-image"]),
        ("cover-image2", "/EPUB/img/cover.avif", "img/cover.avif", "image/avif", None, Some("cover-image3"), vec![]),
        ("cover-image3", "/EPUB/img/cover.png", "img/cover.png", "image/png", None, None, vec![]),
        ("style", "/file%20name%20with%20spaces.css", "../../file%20name%20with%20spaces.css", "text/css", None, None, vec![]),
        ("toc", "/toc.xhtml", "../toc.xhtml", "application/xhtml+xml", None, None, vec!["scripted", "nav"]),
        ("toc-ncx", "/toc.ncx", "../toc.ncx", "application/x-dtbncx+xml", None, None, vec![]),
    ];

    assert_eq!(expected.len(), entries.len());
    assert!(!manifest.is_empty());

    for (entry, (id, href, href_raw, media_type, overlay, fallback, properties)) in
        entries.into_iter().zip(expected)
    {
        assert_eq!(id, entry.id());
        assert_eq!(href, entry.href().as_str());
        assert_eq!(href_raw, entry.href_raw().as_str());
        assert_eq!(media_type, entry.media_type());
        assert_eq!(properties, entry.properties().iter().collect::<Vec<_>>());

        for property in properties {
            assert!(entry.properties().has_property(property))
        }
        match overlay {
            Some(overlay) => assert_eq!(overlay, entry.media_overlay().unwrap().id()),
            None => assert!(entry.media_overlay().is_none()),
        }
        match fallback {
            Some(fallback) => {
                assert_eq!(fallback, entry.fallback().unwrap().id());
                assert_eq!(fallback, entry.fallbacks().next().unwrap().id());
            }
            None => {
                assert!(entry.fallback().is_none());
                assert!(entry.fallbacks().next().is_none());
            }
        }

        // Ensure the resource matches
        let resource = entry.resource();
        assert_eq!(media_type, resource.kind().as_str());
        assert_eq!(resource.kind(), &entry.resource_kind());

        match resource.key() {
            ResourceKey::Value(key) => assert_eq!(href, key),
            ResourceKey::Position(_) => unreachable!(),
        }
    }
}

#[test]
#[wasm_bindgen_test]
fn test_manifest_entry_refinements() {
    let epub = open_example_epub_file();
    let manifest = epub.manifest();
    let mut entries = manifest.entries().collect::<Vec<_>>();
    // sort by `id` as entries are in arbitrary order
    entries.sort_by_key(|entry| entry.id());

    #[rustfmt::skip]
    let expected = [
        vec![],
        vec![("c1_audio", "media:duration", "0:32:29")],
        vec![],
        vec![],
        vec![("c2_audio", "media:duration", "0:29:49")],
        vec![],
        vec![],
        vec![],
        vec![],
        vec![],
        vec![],
    ];

    for (entry, expected) in entries.into_iter().zip(expected) {
        assert_eq!(expected.len(), entry.refinements().iter().count());

        for (parent_id, property, value) in expected {
            let refinements = entry
                .refinements()
                .by_property(property)
                .collect::<Vec<_>>();

            assert!(entry.refinements().has_property(property));
            assert_eq!(1, refinements.len());
            assert_eq!(value, refinements[0].value());
            assert_eq!(Some(parent_id), refinements[0].refines());
            assert_eq!(parent_id, entry.id());
        }
    }
}
