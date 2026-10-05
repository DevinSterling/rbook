use crate::epub::util::TestEpub::Epub3File;
use wasm_bindgen_test::wasm_bindgen_test;

#[test]
#[wasm_bindgen_test]
fn test_manifest_by_spine_index() {
    let mut epub = Epub3File.open_strict();
    let mut manifest = epub.manifest_mut();

    let a = manifest.by_spine_index_mut(0).unwrap();
    assert_eq!("cover", a.as_view().id());

    let b = manifest.by_spine_index_mut(1).unwrap();
    assert_eq!("toc", b.as_view().id());

    let c = manifest.by_spine_index_mut(4).unwrap();
    assert_eq!("c2", c.as_view().id());

    assert!(manifest.by_spine_index_mut(5).is_none());
}

#[test]
#[wasm_bindgen_test]
fn test_contains_new_file_after_modification() {
    let mut epub = Epub3File.open_strict();
    assert!(!epub.contains_resource("my_file.txt"));

    let mut manifest = epub.manifest_mut();
    manifest.push(("my_file.txt", "Hello World!"));
    assert!(epub.contains_resource("my_file.txt"));

    let mut manifest = epub.manifest_mut();
    manifest.remove_by_id("my-file-txt");
    assert!(!epub.contains_resource("my_file.txt"));
}
