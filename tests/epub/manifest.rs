use crate::epub::util::TestEpub::{Epub2Dir, Epub3File};
use rbook::ebook::resource::ResourceKey;
use wasm_bindgen_test::wasm_bindgen_test;

#[test]
#[wasm_bindgen_test]
fn test_manifest() {
    let epub = Epub3File.open_strict();
    let manifest = epub.manifest();
    let mut entries = manifest.iter().collect::<Vec<_>>();
    // sort by `id` as entries are in arbitrary order
    entries.sort_by_key(|entry| entry.id());

    assert_eq!(EXPECTED_MANIFEST.len(), entries.len());
    assert!(!manifest.is_empty());

    for (entry, expected) in entries.into_iter().zip(EXPECTED_MANIFEST) {
        assert_eq!(expected.id, entry.id());
        assert_eq!(expected.href, entry.href().as_str());
        assert_eq!(expected.href_raw, entry.href_raw().as_str());
        assert_eq!(expected.media_type, entry.media_type());
        #[rustfmt::skip]
        assert_eq!(expected.properties, entry.properties().iter().collect::<Vec<_>>());

        for property in expected.properties {
            assert!(entry.properties().has_property(property))
        }
        match expected.media_overlay {
            Some(overlay) => assert_eq!(overlay, entry.media_overlay().unwrap().id()),
            None => assert!(entry.media_overlay().is_none()),
        }
        match expected.fallback {
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
        assert_eq!(expected.media_type, resource.kind().as_str());
        assert_eq!(resource.kind(), &entry.kind());

        match resource.key() {
            ResourceKey::Value(key) => assert_eq!(expected.href, key),
            ResourceKey::Position(_) => unreachable!(),
        }
    }
}

#[test]
#[wasm_bindgen_test]
fn test_manifest_entry_refinements() {
    let epub = Epub3File.open_strict();
    let manifest = epub.manifest();
    let mut entries = manifest.iter().collect::<Vec<_>>();
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

#[test]
#[wasm_bindgen_test]
fn test_skip_manifest() {
    use rbook::ebook::resource::ResourceKind;

    let epub = Epub3File.build(|b| b.skip_manifest(true));
    let manifest = epub.manifest();

    assert_eq!(0, manifest.len());
    assert!(manifest.is_empty());
    assert!(manifest.iter().next().is_none());
    assert!(manifest.images().next().is_none());
    assert!(manifest.readable_content().next().is_none());
    assert!(manifest.by_id("c1a").is_none());
    assert!(manifest.by_href("c2.xhtml").is_none());
    assert!(manifest.by_property("nav").next().is_none());
    assert!(manifest.cover_image().is_none());
    #[rustfmt::skip]
    assert!(manifest.by_kind(ResourceKind::APPLICATION).next().is_none());
    #[rustfmt::skip]
    assert!(manifest.by_kind(
        [ResourceKind::APPLICATION, ResourceKind::IMAGE]).next().is_none()
    );
}

#[test]
#[wasm_bindgen_test]
fn test_reader_skip_manifest() {
    use super::spine::EXPECTED_SPINE;
    use rbook::ebook::errors::FormatError;
    use rbook::epub::errors::EpubError;
    use rbook::reader::errors::ReaderError;

    let epub = Epub3File.build(|b| b.skip_manifest(true));
    let reader = epub.reader();

    assert!(!reader.is_empty());
    assert_eq!(EXPECTED_SPINE.len(), reader.len());
    assert_eq!(EXPECTED_SPINE.len(), reader.remaining());

    for (content_result, expected) in reader.zip(EXPECTED_SPINE) {
        // An error must be guaranteed as the manifest cannot be referenced since it was skipped
        let err = content_result.expect_err("Content retrieval must fail without a manifest");

        match err {
            ReaderError::Format(FormatError::Epub(EpubError::InvalidIdref(idref))) => {
                assert_eq!(expected.idref, idref);
            }
            _ => unreachable!("Any other error should not occur"),
        }
    }
}

#[rustfmt::skip]
#[test]
#[wasm_bindgen_test]
fn test_toc_skip_manifest() {
    use rbook::ebook::toc::TocEntryKind;
    use rbook::epub::metadata::EpubVersion;

    let epub = Epub3File.build(|b| b.skip_manifest(true).skip_toc(false));
    let toc = epub.toc();

    assert!(toc.contents().is_none());
    assert!(toc.page_list().is_none());
    assert!(toc.by_kind_version(TocEntryKind::Landmarks, EpubVersion::EPUB3).is_none());

    for kind in [TocEntryKind::Toc, TocEntryKind::PageList] {
        for version in [EpubVersion::EPUB2, EpubVersion::EPUB3] {
            assert!(toc.by_kind_version(kind, version).is_none());
        }
        assert!(toc.by_kind(kind).is_none());
    }

    // Only the EPUB 2 guide is parsed
    assert!(toc.landmarks().is_some());
    assert!(toc.by_kind(TocEntryKind::Landmarks).is_some());
    assert!(toc.by_kind_version(TocEntryKind::Landmarks, EpubVersion::EPUB2).is_some());
}

#[test]
#[wasm_bindgen_test]
fn test_spine_and_skip_manifest_entry_reference() {
    use super::spine::EXPECTED_SPINE;

    let epub = Epub3File.build(|b| b.skip_manifest(true));
    let spine = epub.spine();

    assert!(!spine.is_empty());
    assert_eq!(EXPECTED_SPINE.len(), spine.len());
    for (entry, expected) in spine.iter().zip(EXPECTED_SPINE) {
        assert!(entry.manifest_entry().is_none());
        assert!(entry.resource().is_none());

        // Check basic integrity
        assert_eq!(entry.id(), expected.id);
        assert_eq!(entry.idref(), expected.idref);
        assert_eq!(entry.is_linear(), expected.is_linear);
    }
}

#[test]
#[wasm_bindgen_test]
fn test_toc_and_skip_manifest_entry_reference() {
    use super::toc::EXPECTED_GUIDE;

    let epub = Epub3File.build(|b| b.skip_manifest(true));
    let toc = epub.toc();
    let guide = toc.landmarks().unwrap();

    // Only the EPUB 2 guide parsable when the manifest is skipped
    assert!(guide.is_root());
    assert_eq!(EXPECTED_GUIDE.len(), guide.len());

    for (entry, expected) in guide.into_iter().zip(EXPECTED_GUIDE) {
        assert!(entry.manifest_entry().is_none());
        assert!(entry.resource().is_none());

        // Check basic integrity
        assert_eq!(expected.depth, entry.depth());
        assert_eq!(expected.href, entry.href_raw().unwrap().as_str());
        assert_eq!(expected.label, entry.label());
        assert_eq!(expected.kind, entry.kind());
    }
}

#[test]
fn test_manifest_iterators() {
    /////////////////////
    // EPUB 3 test file
    /////////////////////
    let epub = Epub3File.open_strict();
    let manifest = epub.manifest();

    let styles: Vec<_> = manifest.styles().collect();
    assert_eq!("text/css", styles[0].media_type());
    assert_eq!(1, styles.len());

    assert_eq!(None, manifest.scripts().next());
    assert_eq!(None, manifest.fonts().next());
    assert_eq!(None, manifest.audio().next());
    assert_eq!(None, manifest.video().next());

    /////////////////////
    // EPUB 2 test file
    /////////////////////
    let epub = Epub2Dir.open_strict();
    let manifest = epub.manifest();

    let scripts: Vec<_> = manifest.scripts().collect();
    assert_eq!("application/javascript", scripts[0].media_type());
    assert_eq!("application/ecmascript", scripts[1].media_type());
    assert_eq!("text/javascript", scripts[2].media_type());
    assert_eq!("application/javascript", scripts[3].media_type());
    assert_eq!(4, scripts.len());

    let styles: Vec<_> = manifest.styles().collect();
    assert_eq!(0, styles.len());

    let fonts: Vec<_> = manifest.fonts().collect();
    assert_eq!("font/woff", fonts[0].media_type());
    assert_eq!("font/ttf", fonts[1].media_type());
    assert_eq!("application/font-woff", fonts[2].media_type());
    assert_eq!(3, fonts.len());

    let audio: Vec<_> = manifest.audio().collect();
    assert_eq!("audio/aac", audio[0].media_type());
    assert_eq!("audio/mp3", audio[1].media_type());
    assert_eq!("audio/ogg", audio[2].media_type());
    assert_eq!(3, audio.len());

    let video: Vec<_> = manifest.video().collect();
    assert_eq!("video/mp4", video[0].media_type());
    assert_eq!("video/mpv", video[1].media_type());
    assert_eq!(2, video.len());
}

/////////////////////////////////////////////////
// TEST DATA
/////////////////////////////////////////////////

pub struct ManifestTestData<'a> {
    pub id: &'a str,
    pub href: &'a str,
    pub href_raw: &'a str,
    pub media_type: &'a str,
    pub media_overlay: Option<&'a str>,
    pub fallback: Option<&'a str>,
    pub properties: &'a [&'a str],
}

impl<'a> ManifestTestData<'a> {
    const fn new(
        id: &'a str,
        href: &'a str,
        href_raw: &'a str,
        media_type: &'a str,
        media_overlay: Option<&'a str>,
        fallback: Option<&'a str>,
        properties: &'a [&'a str],
    ) -> Self {
        Self {
            id,
            href,
            href_raw,
            media_type,
            media_overlay,
            fallback,
            properties,
        }
    }
}

// Reference: example.epub / example_epub
#[rustfmt::skip]
pub const EXPECTED_MANIFEST: &[ManifestTestData] = &[
    ManifestTestData::new("c1", "/EPUB/c1.xhtml", "c1.xhtml", "application/xhtml+xml", Some("c1_audio"), None, &[]),
    ManifestTestData::new("c1_audio", "/EPUB/overlay/chapter1_audio.smil", "overlay/chapter1_audio.smil", "application/smil+xml", None, None, &[]),
    ManifestTestData::new("c1a", "/EPUB/c1a.xhtml", "c1a.xhtml", "application/xhtml+xml", None, None, &[]),
    ManifestTestData::new("c2", "/EPUB/c2.xhtml", "c2.xhtml", "application/xhtml+xml", Some("c2_audio"), None, &[]),
    ManifestTestData::new("c2_audio", "/EPUB/overlay/chapter2_audio.smil", "overlay/chapter2_audio.smil", "application/smil+xml", None, None, &[]),
    ManifestTestData::new("cover", "/EPUB/cover.xhtml", "cover.xhtml", "application/xhtml+xml", None, None, &[]),
    ManifestTestData::new("cover-image1", "/EPUB/img/cover.webm", "img/cover.webm", "image/webm", None, Some("cover-image2"), &["cover-image"]),
    ManifestTestData::new("cover-image2", "/EPUB/img/cover.avif", "img/cover.avif", "image/avif", None, Some("cover-image3"), &[]),
    ManifestTestData::new("cover-image3", "/EPUB/img/cover.png", "img/cover.png", "image/png", None, None, &[]),
    ManifestTestData::new("style", "/file%20name%20with%20spaces.css", "../file%20name%20with%20spaces.css", "text/css", None, None, &[]),
    ManifestTestData::new("toc", "/toc.xhtml", "../toc.xhtml", "application/xhtml+xml", None, None, &["scripted", "nav"]),
    ManifestTestData::new("toc-ncx", "/toc.ncx", "../toc.ncx", "application/x-dtbncx+xml", None, None, &[]),
];
