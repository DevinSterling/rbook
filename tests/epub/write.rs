use crate::epub::manifest::EXPECTED_MANIFEST;
use crate::epub::util::EPUB3_DIR;
use crate::epub::util::TestEpub::{Epub3Dir, Epub3File};
use rbook::Epub;
use rbook::ebook::element::Href;
use rbook::ebook::errors::ArchiveError;
use rbook::ebook::spine::PageDirection;
use rbook::ebook::toc::TocEntryKind;
use rbook::epub::EpubChapter;
use rbook::epub::metadata::EpubVersion;
use rbook::input::Batch;
use std::io::Cursor;
use std::path::Path;
use wasm_bindgen_test::wasm_bindgen_test;

#[test]
fn test_dir_comparison() {
    // NOTE:
    // > If the ebook contained malformations such as an invalid relative paths
    //   within the package, they are corrected in the generated epub.
    // - If such case happens, the original and new will be not equal here.
    let epub_a = Epub3Dir.open_strict();
    let epub_b_bytes = epub_a.write().compression(0).to_vec().unwrap();
    let epub_b = Epub::read(Cursor::new(epub_b_bytes)).unwrap();

    assert_eq!(epub_a.package(), epub_b.package());
    assert_eq!(epub_a.metadata(), epub_b.metadata());
    assert_eq!(epub_a.manifest(), epub_b.manifest());
    assert_eq!(epub_a.spine(), epub_b.spine());
    assert_eq!(epub_a.toc(), epub_b.toc());
    assert_eq!(epub_a, epub_b);
}

#[test]
#[wasm_bindgen_test]
fn test_file_comparison() {
    let epub_a = Epub3File.open_strict();
    let epub_b_bytes = epub_a.write().compression(0).to_vec().unwrap();
    let epub_b = Epub::read(Cursor::new(epub_b_bytes)).unwrap();

    assert_eq!(epub_a.package(), epub_b.package());
    assert_eq!(epub_a.metadata(), epub_b.metadata());
    assert_eq!(epub_a.manifest(), epub_b.manifest());
    assert_eq!(epub_a.spine(), epub_b.spine());
    assert_eq!(epub_a.toc(), epub_b.toc());
    assert_eq!(epub_a, epub_b);
}

#[test]
#[wasm_bindgen_test]
fn test_retain_whitespace() {
    let mut epub = Epub::new();

    #[rustfmt::skip]
    const INSERTED_METADATA: &[(&str, &str)] = &[
        ("dc:title", "Title with non\u{00A0}breaking\u{00A0}space"),
        ("dc:creator", "John<:#&\"'>Doe"),
        ("dc:description", "Paragraph #1\n\t\rParagraph #2\nParagraph #3"),
        ("custom:data", "fixed & that <> \" wow \np1\np2\np3\np4\np5\np6\np7\""),
    ];

    // Insertion
    epub.metadata_mut().push(INSERTED_METADATA.to_vec());

    let epub_bytes = epub.write().compression(0).to_vec().unwrap();
    let epub = Epub::read(Cursor::new(epub_bytes)).unwrap();
    let metadata = epub.metadata();

    for (property, expected_value) in INSERTED_METADATA {
        let entry = metadata.by_property(property).next().unwrap();

        assert_eq!(*property, entry.property());
        assert_eq!(*expected_value, entry.value());
    }
}

#[test]
#[wasm_bindgen_test]
fn test_epub_cleanup() {
    let mut epub = Epub3File.open_strict();
    epub.manifest_mut().clear();
    epub.cleanup();

    assert!(epub.manifest().is_empty());
    assert!(epub.spine().is_empty());
    assert!(epub.toc().contents().unwrap().is_empty());
    assert!(epub.toc().landmarks().unwrap().is_empty());
    assert!(epub.toc().page_list().unwrap().is_empty());
}

#[test]
#[wasm_bindgen_test]
fn test_epub_keep_orphans() {
    use std::collections::HashSet;
    use std::sync::{Arc, Mutex};

    let mut epub = Epub3File.open_strict();
    epub.manifest_mut().clear();

    // Arc + Mutex is required here as `keep_orphans` does not support `FnMut`
    let arc_orphaned_resources = Arc::new(Mutex::new(Vec::new()));
    let shared = arc_orphaned_resources.clone();

    // Retain all orphaned resources
    let bytes = epub
        .write()
        .keep_orphans(move |file: Href| {
            shared.lock().unwrap().push(file.as_str().to_owned());
            true
        })
        .compression(0)
        .to_vec()
        .unwrap();

    let orphaned_resources: HashSet<_> = Arc::into_inner(arc_orphaned_resources)
        .unwrap()
        .into_inner()
        .unwrap()
        .into_iter()
        .collect();
    let expected_orphaned_resources: HashSet<_> = EXPECTED_MANIFEST
        .iter()
        .map(|item| {
            // Decode as `keep_orphans` gives undecoded paths
            // directly from the archive.
            percent_encoding::percent_decode(item.href.as_bytes())
                .decode_utf8_lossy()
                .into_owned()
        })
        .collect();

    assert_eq!(orphaned_resources, expected_orphaned_resources);

    // Check if orphaned resources are retained
    let epub = Epub::read(Cursor::new(bytes)).unwrap();
    for resource in expected_orphaned_resources {
        assert!(epub.read_resource_bytes(resource).is_ok());
    }
}

#[test]
#[wasm_bindgen_test]
fn test_epub_remove_orphans() {
    let mut epub = Epub3File.open_strict();
    epub.manifest_mut().clear();

    let bytes = epub
        .write()
        .keep_orphans(false)
        .compression(0)
        .to_vec()
        .unwrap();

    // Check if orphaned resources are retained
    let epub = Epub::read(Cursor::new(bytes)).unwrap();
    for resource in EXPECTED_MANIFEST.iter().map(|item| item.href) {
        let result = epub.read_resource_bytes(resource);

        assert!(matches!(result, Err(ArchiveError::InvalidResource { .. })));
    }
}

const DATA: &[u8] = b"sample data";

// Specialized assertion macro for metadata entry iterators
macro_rules! assert_iter {
    ($iter:expr, [$($expected:expr),+ $(,)?]) => {
        let mut iter = $iter;
        $(assert_eq!($expected, iter.next().unwrap().value());)+
        assert_eq!(None, iter.next());
    };
}

#[rustfmt::skip]
#[test]
#[wasm_bindgen_test]
fn test_epub2_editor_metadata() {
    let built_epub = Epub::builder()
        .version(2)
        .package_location("EPUB/123.opf")
        .modified_date("2020")
        .published_date("2025")
        .modified_date("2026-03-12T00:00:00Z")
        .generator("abc")
        .title("<One & Two>")
        .title(["My First Story", "Subtitle"])
        .creator(["John Doe", "Jane Doe"])
        .contributor("unknown")
        .clear_meta("dc:contributor")
        .contributor("Doe1")
        .contributor("Doe2")
        .identifier("https://github.com/DevinSterling/rbook")
        .language(["en", "ko"])
        .publisher("rbook")
        .tag(["Action", "Fantasy & Magic"])
        .description("A description\nLine 1\nLine 2\nLine 3")
        .rights("Copyright")
        .meta(("dc:format", "xyz"))
        .page_direction(PageDirection::RightToLeft)
        .build();

    let bytes = built_epub.write().compression(0).to_vec().unwrap();
    let epub = Epub::read(Cursor::new(bytes)).unwrap();

    // Package
    assert_eq!(epub.package().location().as_str(), "/EPUB/123.opf");
    assert!(epub.metadata().version().is_epub2());

    // Metadata
    let metadata = epub.metadata();
    assert_eq!(2025, metadata.published().unwrap().date().year());
    assert_eq!(None, metadata.modified());

    assert_iter!(metadata.identifiers(), ["https://github.com/DevinSterling/rbook"]);
    assert_iter!(metadata.generators(), ["abc"]);
    assert_iter!(metadata.titles(), ["<One & Two>", "My First Story", "Subtitle"]);
    assert_iter!(metadata.creators(), ["John Doe", "Jane Doe"]);
    assert_iter!(metadata.contributors(), ["Doe1", "Doe2"]);
    assert_iter!(metadata.publishers(), ["rbook"]);
    assert_iter!(metadata.languages(), ["en", "ko"]);
    assert_iter!(metadata.tags(), ["Action", "Fantasy & Magic"]);
    assert_iter!(metadata.descriptions(), ["A description\nLine 1\nLine 2\nLine 3"]);
    assert_iter!(metadata.by_property("dc:format"), ["xyz"]);
    assert_iter!(metadata.by_property("dc:rights"), ["Copyright"]);

    // The text direction is not `rtl` since EPUB 2 doesn't support setting the page direction
    assert_eq!(PageDirection::Default, epub.spine().page_direction());
}

#[test]
#[wasm_bindgen_test]
fn test_epub3_editor_metadata() {
    let built_epub = Epub::builder()
        .identifier("123")
        .title("Example\u{00A0}EPUB")
        .creator("Jane Doe")
        .language("en")
        .page_direction(PageDirection::RightToLeft)
        .build();

    let bytes = built_epub.write().compression(0).to_vec().unwrap();
    let epub = Epub::read(Cursor::new(bytes)).unwrap();

    // Package
    assert_eq!(epub.package().location().as_str(), "/OEBPS/package.opf");
    assert!(epub.metadata().version().is_epub3());

    // Metadata
    let metadata = epub.metadata();

    // Generated dates are unsupported for `wasm32/64-unknown-unknown`
    let is_generated = !cfg!(all(target_family = "wasm", target_os = "unknown"));
    // Check if automatically generated
    assert_eq!(is_generated, metadata.published().is_some());
    assert_eq!(is_generated, metadata.modified().is_some());

    assert_iter!(
        metadata.generators(),
        [concat!("rbook v", env!("CARGO_PKG_VERSION"))]
    );
    assert_iter!(metadata.identifiers(), ["123"]);
    assert_iter!(metadata.titles(), ["Example\u{00A0}EPUB"]);
    assert_iter!(metadata.creators(), ["Jane Doe"]);
    assert_iter!(metadata.languages(), ["en"]);

    assert_eq!(PageDirection::RightToLeft, epub.spine().page_direction());
}

#[test]
#[wasm_bindgen_test]
fn test_epub2_editor_chapters() {
    let built_epub = Epub::builder()
        .version(2)
        .chapter([
            EpubChapter::new("Part I").xhtml(DATA).children([
                EpubChapter::new("I").xhtml(DATA),
                EpubChapter::new("II").xhtml(DATA),
            ]),
            EpubChapter::new("Part II")
                .xhtml(DATA)
                .href("v2.xhtml")
                .children([
                    EpubChapter::new("I").xhtml(DATA),
                    EpubChapter::new("II").href("v2c2.xhtml").xhtml(DATA),
                ]),
            EpubChapter::new("Part III")
                .children(EpubChapter::unlisted("/v3extras.xhtml").xhtml(DATA)),
            EpubChapter::new("Part III+").children(
                EpubChapter::new("Nested")
                    .children(EpubChapter::new("Finale").href("/v3extras.xhtml#finale")),
            ),
        ])
        .build();

    let bytes = built_epub.write().compression(0).to_vec().unwrap();
    let epub = Epub::read(Cursor::new(bytes)).unwrap();

    #[rustfmt::skip]
    let expected_manifest = [
        ("part-i", "part-i.xhtml", "application/xhtml+xml"),
        ("i", "i.xhtml", "application/xhtml+xml"),
        ("ii", "ii.xhtml", "application/xhtml+xml"),
        ("part-ii", "v2.xhtml", "application/xhtml+xml"),
        ("i-1", "i-1.xhtml", "application/xhtml+xml"),
        ("ii-1", "v2c2.xhtml", "application/xhtml+xml"),
        ("v3extras-xhtml", "../v3extras.xhtml", "application/xhtml+xml"),
        // Auto-generated toc documents
        ("ncx", "toc.ncx", "application/x-dtbncx+xml"), // EPUB 2
    ];
    // Indices 0..8 are all added to the spine
    let idrefs = &expected_manifest[..7];

    // Manifest
    let manifest = epub.manifest();
    assert_eq!(manifest.len(), expected_manifest.len());

    for ((id, href, mime), entry) in expected_manifest.into_iter().zip(manifest) {
        assert_eq!(id, entry.id());
        assert_eq!(href, entry.href_raw());
        assert_eq!(mime, entry.media_type());

        // `ncx` content is generated dynamically, so it will not equal `DATA`
        if id != "ncx" {
            assert_eq!(DATA, entry.read_bytes().unwrap());
        }
    }

    // Spine
    let spine = epub.spine();
    assert_eq!(idrefs.len(), spine.len());

    for ((idref, _, _), entry) in idrefs.iter().zip(spine) {
        assert_eq!(None, entry.id());
        assert_eq!(*idref, entry.idref());
        assert!(entry.is_linear());
    }

    #[rustfmt::skip]
    let expected_toc_contents = [
        (1, "nav-point-1", "Part I", "part-i.xhtml"),
        (2, "nav-point-2", "I", "i.xhtml"),
        (2, "nav-point-3", "II", "ii.xhtml"),
        (1, "nav-point-4", "Part II", "v2.xhtml"),
        (2, "nav-point-5", "I", "i-1.xhtml"),
        (2, "nav-point-6", "II", "v2c2.xhtml"),
        (1, "nav-point-7", "Part III+", "../v3extras.xhtml#finale"),
        (2, "nav-point-8", "Nested", "../v3extras.xhtml#finale"),
        (3, "nav-point-9", "Finale", "../v3extras.xhtml#finale"),
   ];

    let contents = epub.toc().contents().unwrap();
    assert_eq!(EpubVersion::EPUB2, contents.version());
    assert_eq!("Table of Contents", contents.label());
    assert_eq!(0, contents.depth());
    assert_eq!(3, contents.len());
    assert_eq!(expected_toc_contents.len(), contents.total_len());

    for ((depth, id, label, href), entry) in
        expected_toc_contents.into_iter().zip(contents.flatten())
    {
        assert_eq!(depth, entry.depth());
        assert_eq!(Some(id), entry.id());
        assert_eq!(label, entry.label());
        assert_eq!(Some(href), entry.href_raw().map(|raw| raw.as_str()));
    }
}

#[test]
#[wasm_bindgen_test]
fn test_epub3_editor_chapters() {
    let built_epub = Epub::builder()
        .chapter([
            EpubChapter::new("Volume I").xhtml(DATA).children([
                EpubChapter::new("I").href("v1c1.xhtml").xhtml(DATA),
                EpubChapter::new("II").href("v1c2.xhtml").xhtml(DATA),
                EpubChapter::new("III").href("v1c3.xhtml").xhtml(DATA),
                EpubChapter::new("IV").xhtml(DATA),
                EpubChapter::new("V").xhtml(DATA),
            ]),
            EpubChapter::new("Volume II")
                .xhtml(DATA)
                .href("v2.xhtml")
                .children([
                    EpubChapter::new("I").xhtml(DATA),
                    EpubChapter::new("II").xhtml(DATA),
                    EpubChapter::new("III")
                        .href("chapters/v2c3.xhtml")
                        .xhtml(DATA),
                    EpubChapter::new("IV").xhtml(DATA),
                ]),
            EpubChapter::new("Volume III").xhtml(DATA).children([
                EpubChapter::new("I").href("v3c1.xhtml").xhtml(DATA),
                EpubChapter::new("I.I").href("v3c1.xhtml#s1"),
                EpubChapter::new("I.II").href("v3c1.xhtml#s2"),
                EpubChapter::unlisted("v3extras.xhtml").xhtml(DATA),
            ]),
            EpubChapter::new("Volume III+").children(
                EpubChapter::new("Nested")
                    .children(EpubChapter::new("Finale").href("v3extras.xhtml#finale")),
            ),
        ])
        .toc_title("Story ToC")
        .build();

    let bytes = built_epub.write().compression(0).to_vec().unwrap();
    let epub = Epub::read(Cursor::new(bytes)).unwrap();

    #[rustfmt::skip]
    let expected_manifest = [
        ("volume-i", "volume-i.xhtml", "application/xhtml+xml"),
        ("i", "v1c1.xhtml", "application/xhtml+xml"),
        ("ii", "v1c2.xhtml", "application/xhtml+xml"),
        ("iii", "v1c3.xhtml", "application/xhtml+xml"),
        ("iv", "iv.xhtml", "application/xhtml+xml"),
        ("v", "v.xhtml", "application/xhtml+xml"),
        ("volume-ii", "v2.xhtml", "application/xhtml+xml"),
        ("i-1", "i-1.xhtml", "application/xhtml+xml"),
        ("ii-1", "ii-1.xhtml", "application/xhtml+xml"),
        ("iii-1", "chapters/v2c3.xhtml", "application/xhtml+xml"),
        ("iv-1", "iv-1.xhtml", "application/xhtml+xml"),
        ("volume-iii", "volume-iii.xhtml", "application/xhtml+xml"),
        ("i-2", "v3c1.xhtml", "application/xhtml+xml"),
        ("v3extras-xhtml", "v3extras.xhtml", "application/xhtml+xml"),
        // Auto-generated toc documents
        ("ncx", "toc.ncx", "application/x-dtbncx+xml"), // EPUB 2
        ("nav", "toc.xhtml", "application/xhtml+xml"), // EPUB 3
    ];
    // Indices 0..15 are all added to the spine
    let idrefs = &expected_manifest[..14];

    // Manifest
    let manifest = epub.manifest();
    assert_eq!(manifest.len(), expected_manifest.len());

    for ((id, href, mime), entry) in expected_manifest.into_iter().zip(manifest) {
        assert_eq!(id, entry.id());
        assert_eq!(href, entry.href_raw());
        assert_eq!(mime, entry.media_type());

        // `ncx` & `nav` content is generated dynamically, so it will not equal `DATA`
        if !matches!(id, "ncx" | "nav") {
            assert_eq!(DATA, entry.read_bytes().unwrap());
        }
    }

    // Spine
    let spine = epub.spine();
    assert_eq!(idrefs.len(), spine.len());

    for ((idref, _, _), entry) in idrefs.iter().zip(spine) {
        assert_eq!(None, entry.id());
        assert_eq!(*idref, entry.idref());
        assert!(entry.is_linear());
    }

    #[rustfmt::skip]
    let expected_toc_contents = [
        (1, "Volume I", Some("volume-i.xhtml")),
        (2, "I", Some("v1c1.xhtml")),
        (2, "II", Some("v1c2.xhtml")),
        (2, "III", Some("v1c3.xhtml")),
        (2, "IV", Some("iv.xhtml")),
        (2, "V", Some("v.xhtml")),
        (1, "Volume II", Some("v2.xhtml")),
        (2, "I", Some("i-1.xhtml")),
        (2, "II", Some("ii-1.xhtml")),
        (2, "III", Some("chapters/v2c3.xhtml")),
        (2, "IV", Some("iv-1.xhtml")),
        (1, "Volume III", Some("volume-iii.xhtml")),
        (2, "I", Some("v3c1.xhtml")),
        (2, "I.I", Some("v3c1.xhtml#s1")),
        (2, "I.II", Some("v3c1.xhtml#s2")),
        (1, "Volume III+", None),
        (2, "Nested", None),
        (3, "Finale", Some("v3extras.xhtml#finale")),
   ];

    let contents = epub.toc().contents().unwrap();
    assert_eq!(EpubVersion::EPUB3, contents.version());
    assert_eq!("Story ToC", contents.label());
    assert_eq!(0, contents.depth());
    assert_eq!(4, contents.len());
    assert_eq!(expected_toc_contents.len(), contents.total_len());

    for ((depth, label, href), entry) in expected_toc_contents.into_iter().zip(contents.flatten()) {
        assert_eq!(depth, entry.depth());
        assert_eq!(label, entry.label());
        assert_eq!(href, entry.href_raw().map(|raw| raw.as_str()));
    }
}

#[test]
#[wasm_bindgen_test]
fn test_epub3_editor_landmarks() {
    let built_epub = Epub::builder()
        .chapter([
            EpubChapter::new("Introduction")
                .kind(TocEntryKind::Introduction)
                .xhtml(DATA),
            EpubChapter::new("Copyright")
                .kind(TocEntryKind::CopyrightPage)
                .xhtml(DATA),
            EpubChapter::new("Volume I")
                .kind(TocEntryKind::Volume)
                .xhtml(DATA)
                .children([
                    EpubChapter::new("Prologue")
                        .kind(TocEntryKind::Prologue)
                        .xhtml(DATA),
                    EpubChapter::new("I").href("v1-chapter1.xhtml").xhtml(DATA),
                    EpubChapter::new("I.I")
                        .kind(TocEntryKind::Chapter)
                        .href("v1-chapter1.xhtml#i"),
                    EpubChapter::new("II").xhtml(DATA),
                ]),
        ])
        .landmarks_title("Points of Interest")
        .build();

    let bytes = built_epub.write().compression(0).to_vec().unwrap();
    let epub = Epub::read(Cursor::new(bytes)).unwrap();

    #[rustfmt::skip]
    let expected_landmarks_contents = [
        (TocEntryKind::Introduction, "Introduction", "introduction.xhtml"),
        (TocEntryKind::CopyrightPage, "Copyright", "copyright.xhtml"),
        (TocEntryKind::Volume, "Volume I", "volume-i.xhtml"),
        (TocEntryKind::Prologue, "Prologue", "prologue.xhtml"),
        (TocEntryKind::Chapter, "I.I", "v1-chapter1.xhtml#i"),
    ];

    let landmarks = epub.toc().landmarks().unwrap();
    assert_eq!(EpubVersion::EPUB3, landmarks.version());
    assert_eq!("Points of Interest", landmarks.label());
    assert_eq!(0, landmarks.depth());
    assert_eq!(1, landmarks.max_depth());
    assert_eq!(5, landmarks.len());
    assert_eq!(expected_landmarks_contents.len(), landmarks.total_len());

    for ((kind, label, href), entry) in expected_landmarks_contents
        .into_iter()
        .zip(landmarks.flatten())
    {
        assert_eq!(kind, entry.kind());
        assert_eq!(label, entry.label());
        assert_eq!(Some(href), entry.href_raw().map(|raw| raw.as_str()));
    }
}

#[test]
#[wasm_bindgen_test]
fn test_epub_editor_container_resources() {
    let container_resources = [
        ("/META-INF/com.apple.ibooks.display-options.xml", "ibooks"),
        ("/rbook.properties", "abc=123"),
    ];

    let mut built_epub = Epub::new();
    for (location, data) in container_resources {
        built_epub.edit().container_resource(location, data);
        assert_eq!(
            data.as_bytes(),
            built_epub.read_resource_bytes(location).unwrap(),
        );
    }

    let bytes = built_epub.write().compression(0).to_vec().unwrap();
    let epub = Epub::read(Cursor::new(bytes)).unwrap();

    for (location, data) in container_resources {
        assert_eq!(data.as_bytes(), epub.read_resource_bytes(location).unwrap());
    }
}

#[test]
#[wasm_bindgen_test]
fn test_epub_editor_ignore_container_resources() {
    // These specific resources must be ignored as rbook create
    // them within the internal `EpubWriter`.
    let container_resources = [
        ("/mimetype", "application/json"),
        ("/META-INF/container.xml", "<p>Hello</p>"),
        ("/OEBPS/package.opf", "<p>World</p>"),
    ];

    let mut built_epub = Epub::new();
    for (location, data) in container_resources {
        built_epub.edit().container_resource(location, data);
        assert_eq!(
            data.as_bytes(),
            built_epub.read_resource_bytes(location).unwrap(),
        );
    }

    let bytes = built_epub.write().compression(0).to_vec().unwrap();
    let epub = Epub::read(Cursor::new(bytes)).unwrap();

    for (location, data) in container_resources {
        assert_ne!(data.as_bytes(), epub.read_resource_bytes(location).unwrap());
    }
}

#[test]
#[wasm_bindgen_test]
fn test_epub_editor_resources() {
    let expected_manifest_entry = [
        ("resource.jpg", "resource-jpg", "image/jpeg"),
        ("resource.jpeg", "resource-jpeg", "image/jpeg"),
        ("resource.png", "resource-png", "image/png"),
        ("resource.svg", "resource-svg", "image/svg+xml"),
        ("resource.gif", "resource-gif", "image/gif"),
        ("resource.webp", "resource-webp", "image/webp"),
        ("resource.xhtml", "resource-xhtml", "application/xhtml+xml"),
        ("resource.html", "resource-html", "text/html"),
        ("resource.htm", "resource-htm", "text/html"),
        ("resource.css", "resource-css", "text/css"),
        ("resource.js", "resource-js", "text/javascript"),
        ("resource.smil", "resource-smil", "application/smil+xml"),
        ("resource.ncx", "resource-ncx", "application/x-dtbncx+xml"),
        ("resource.xml", "resource-xml", "application/xml"),
        ("resource.ttf", "resource-ttf", "font/ttf"),
        ("resource.otf", "resource-otf", "font/otf"),
        ("resource.woff", "resource-woff", "font/woff"),
        ("resource.woff2", "resource-woff2", "font/woff2"),
        ("resource.mp3", "resource-mp3", "audio/mpeg"),
        ("resource.m4a", "resource-m4a", "audio/mp4"),
        ("resource.aac", "resource-aac", "audio/aac"),
        ("resource.mp4", "resource-mp4", "video/mp4"),
        ("resource.m4v", "resource-m4v", "video/mp4"),
        ("resource.webm", "resource-webm", "video/webm"),
        ("resource.rs", "resource-rs", "application/octet-stream"),
    ];

    let built_epub = Epub::builder()
        // Set the version to EPUB 2
        .version(2)
        .resource(Batch(
            expected_manifest_entry
                .into_iter()
                .map(|(href, _, _)| (href, DATA)),
        ))
        .build();

    let bytes = built_epub
        .write()
        .compression(0)
        // No toc generation necessary here
        .generate_toc(false)
        .to_vec()
        .unwrap();

    let epub = Epub::read(Cursor::new(bytes)).unwrap();
    let manifest = epub.manifest();
    assert_eq!(expected_manifest_entry.len(), manifest.len());

    for ((href, id, mime), entry) in expected_manifest_entry.into_iter().zip(manifest) {
        assert_eq!(id, entry.id());
        assert_eq!(href, entry.href_raw());
        assert_eq!(mime, entry.media_type());
        assert_eq!(DATA, entry.read_bytes().unwrap());
    }
}

#[test]
fn test_epub_os_files() {
    let example_epub = Path::new(EPUB3_DIR);
    let pkg_dir = example_epub.join("EPUB");

    let built_epub = Epub::builder()
        .chapter([
            EpubChapter::new("Cover").xhtml(pkg_dir.join("cover.xhtml")),
            EpubChapter::new("Chapter 1").xhtml(pkg_dir.join("c1.xhtml")),
            EpubChapter::new("Chapter 1-A").xhtml(pkg_dir.join("c1a.xhtml")),
            EpubChapter::new("Chapter 2").xhtml(pkg_dir.join("c2.xhtml")),
        ])
        .resource(("c1_audio.smil", pkg_dir.join("overlay/chapter1_audio.smil")))
        .cover_image(("cover_image.png", pkg_dir.join("img/cover.png")))
        .build();

    let bytes = built_epub.write().compression(0).to_vec().unwrap();
    let epub = Epub::read(Cursor::new(bytes)).unwrap();

    let expected_manifest_entry = [
        // (id, file reference on disk)
        ("cover", Some("cover.xhtml")),
        ("chapter-1", Some("c1.xhtml")),
        ("chapter-1-a", Some("c1a.xhtml")),
        ("chapter-2", Some("c2.xhtml")),
        ("c1-audio-smil", Some("overlay/chapter1_audio.smil")),
        ("cover-image-png", Some("img/cover.png")),
        // File content is generated dynamically; not stored on disk
        ("ncx", None),
        ("nav", None),
    ];

    for ((id, file), entry) in expected_manifest_entry.into_iter().zip(epub.manifest()) {
        assert_eq!(id, entry.id());

        // Ignore generated content; not derived from disk
        if let Some(file) = file {
            let path = pkg_dir.join(file);

            assert_eq!(
                std::fs::read(&path).unwrap(),
                entry.read_bytes().unwrap(),
                "File/Resource should match: {path:?}"
            )
        }
    }
}
