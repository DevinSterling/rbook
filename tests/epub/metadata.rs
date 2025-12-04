use crate::epub::util::{open_example_epub_file_with, open_example_epub2_dir_with};
use rbook::Ebook;
use rbook::ebook::metadata::{Metadata, Version};
use rbook::epub::EpubOpenOptions;
use rbook::epub::metadata::EpubVersion;
use wasm_bindgen_test::wasm_bindgen_test;

#[test]
#[wasm_bindgen_test]
fn test_skip_metadata_epub3() {
    let epub = open_example_epub_file_with(EpubOpenOptions::new().skip_metadata(true));
    let metadata = epub.metadata();

    assert_eq!("3.3", metadata.version_str());
    assert_eq!(EpubVersion::Epub3(Version(3, 3)), metadata.version());

    assert!(metadata.publication_date().is_none());
    assert!(metadata.modified_date().is_none());
    assert!(metadata.identifier().is_none());
    assert!(metadata.language().is_none());
    assert!(metadata.title().is_none());
    assert!(metadata.description().is_none());

    assert!(metadata.identifiers().next().is_none());
    assert!(metadata.languages().next().is_none());
    assert!(metadata.titles().next().is_none());
    assert!(metadata.descriptions().next().is_none());
    assert!(metadata.creators().next().is_none());
    assert!(metadata.contributors().next().is_none());
    assert!(metadata.publishers().next().is_none());
    assert!(metadata.tags().next().is_none());
    assert!(metadata.entries().next().is_none());
    assert!(metadata.links().next().is_none());

    assert!(metadata.by_property("dc:title").next().is_none());
    assert!(metadata.by_id("uid").is_none());
}

#[test]
fn test_skip_metadata_epub2() {
    let epub = open_example_epub2_dir_with(EpubOpenOptions::new().skip_metadata(true));
    let metadata = epub.metadata();

    assert_eq!(EpubVersion::Epub2(Version(2, 5)), metadata.version());

    // Minimal as this is primarily tested in `test_skip_metadata_epub3`
    assert!(metadata.title().is_none());
    assert!(metadata.entries().next().is_none());
    assert!(metadata.links().next().is_none());
}
