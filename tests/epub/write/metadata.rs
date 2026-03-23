use rbook::Epub;
use rbook::epub::metadata::DetachedEpubMetaEntry;
use wasm_bindgen_test::wasm_bindgen_test;

#[rustfmt::skip]
#[test]
#[wasm_bindgen_test]
fn test_metadata_insert_ordering() {
    let mut epub = Epub::new();
    let mut metadata = epub.metadata_mut();
    // By default, there is a generator entry, which is not needed here
    metadata.clear();

    metadata.push([
        DetachedEpubMetaEntry::title("Title").into_any(),
        DetachedEpubMetaEntry::creator("John Doe").into_any(),
    ]);

    metadata.insert(1, [
        DetachedEpubMetaEntry::creator("Jane Doe").into_any(),
        DetachedEpubMetaEntry::title("Subtitle").into_any(),
        DetachedEpubMetaEntry::tag("Action").into_any(),
    ]);

    metadata.insert(0, [
        DetachedEpubMetaEntry::title("Main Title").into_any(),
        DetachedEpubMetaEntry::tag("Adventure").into_any(),
    ]);

    metadata.insert(9, [
        DetachedEpubMetaEntry::creator("I").into_any(),
        DetachedEpubMetaEntry::title("Title #4").into_any(),
    ]);

    let metadata = epub.metadata();
    let all: Vec<_> = metadata
        .iter()
        .map(|meta| (meta.property().as_str(), meta.value()))
        .collect();

    // Entries are grouped and insertion order is relative to the property group
    assert_eq!(all, [
        ("dc:title", "Main Title"),
        ("dc:title", "Title"),
        ("dc:title", "Subtitle"),
        ("dc:title", "Title #4"),
        ("dc:creator", "John Doe"),
        ("dc:creator", "Jane Doe"),
        ("dc:creator", "I"),
        ("dc:subject", "Adventure"),
        ("dc:subject", "Action"),
    ]);
}

#[rustfmt::skip]
#[test]
#[wasm_bindgen_test]
fn test_metadata_insert_out_of_bounds() {
    let mut epub = Epub::new();
    let mut metadata = epub.metadata_mut();

    metadata.push(DetachedEpubMetaEntry::title("A"));
    // Passing an out-of-bounds index appends to the end
    metadata.insert(999, [
        DetachedEpubMetaEntry::title("B"),
        DetachedEpubMetaEntry::title("C"),
    ]);
    metadata.insert(usize::MAX, DetachedEpubMetaEntry::title("D"));

    let metadata = epub.metadata();
    let titles: Vec<_> = metadata.titles().map(|t| t.value()).collect();

    assert_eq!(titles, ["A", "B", "C", "D"]);
}

#[rustfmt::skip]
#[test]
#[wasm_bindgen_test]
fn test_metadata_property_iteration_order() {
    let mut epub = Epub::new();
    let mut metadata = epub.metadata_mut();
    // By default, there is a generator entry, which is not needed here
    metadata.clear();

    metadata.push([
        DetachedEpubMetaEntry::title("A").into_any(),
        DetachedEpubMetaEntry::creator("X").into_any(),
        DetachedEpubMetaEntry::title("B").into_any(),
    ]);
    metadata.push(DetachedEpubMetaEntry::dublin_core("source"));
    // The `0` here is relative to other `cover` entries (none exist yet),
    // so this creates a new property group.
    // New property groups are appended after existing groups, so `cover` appears last.
    metadata.insert(0, DetachedEpubMetaEntry::meta_name("cover"));

    let metadata = epub.metadata();
    let properties: Vec<_> = metadata
        .iter()
        .map(|m| m.property().as_str())
        .collect();

    assert_eq!(properties, [
        "dc:title",
        "dc:title",
        "dc:creator",
        "dc:source",
        "cover",
    ]);
}
