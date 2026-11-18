use crate::epub::util::TestEpub::{self, Epub3Dir, Epub3File};
use rbook::ebook::errors::{ArchiveError, EbookError};
use wasm_bindgen_test::wasm_bindgen_test;

#[test]
fn max_resource_bytes_for_epub_dir() {
    max_resource_bytes_shared(Epub3Dir);
}

#[test]
#[wasm_bindgen_test]
fn max_resource_bytes_for_epub_file() {
    max_resource_bytes_shared(Epub3File);
}

fn max_resource_bytes_shared(fixture: TestEpub) {
    let baseline = fixture.open_strict();
    let container_len = baseline
        .read_resource_bytes("/META-INF/container.xml")
        .unwrap()
        .len() as u64;
    let opf_len = baseline
        .read_resource_bytes(baseline.package().location().as_str())
        .unwrap()
        .len() as u64;

    // Limit of 0 rejects `container.xml`
    let err = fixture.build_fail(|b| b.max_resource_bytes(0));
    assert_max_bytes_exceeded(err, 0, "/META-INF/container.xml");

    // Exact `container.xml` size allows it but rejects `example.opf`
    let err = fixture.build_fail(|b| b.max_resource_bytes(container_len));
    assert_max_bytes_exceeded(err, container_len, "/EPUB/example.opf");

    // The largest file `example.opf` allows everything
    let epub = fixture.build(|b| b.max_resource_bytes(opf_len));
    assert!(epub.read_resource_bytes("example.opf").is_ok());
    assert!(epub.read_resource_bytes("c1.xhtml").is_ok());
    assert!(epub.read_resource_bytes("img/cover.png").is_ok());
}

fn assert_max_bytes_exceeded(err: EbookError, expected_limit: u64, expected_resource: &str) {
    let EbookError::Archive(ArchiveError::MaxBytesExceeded { limit, resource }) = err else {
        panic!("expected MaxBytesExceeded");
    };

    assert_eq!(expected_limit, limit);
    assert_eq!(Some(expected_resource), resource.key().value());
}

#[cfg(feature = "write")]
#[test]
#[wasm_bindgen_test]
fn directly_added_resources_not_affected_by_max_resource_bytes() {
    use rbook::Epub;
    use std::io::Cursor;

    const KIB_8: u64 = 8 * 1024;
    let mut epub = Epub3File.build(|b| b.max_resource_bytes(KIB_8));

    // Add a file that is 16KiB
    let raw_data = [0u8; KIB_8 as usize * 2];
    epub.edit().resource(("binary.file", &raw_data));

    let file = epub.read_resource_bytes("binary.file").unwrap();
    assert_eq!(raw_data, *file);

    let written = epub.write().to_vec().unwrap();
    let epub2 = Epub::options()
        .max_resource_bytes(KIB_8)
        .read(Cursor::new(written))
        .unwrap();

    // Because `binary.file` wasn't added explicitly to `epub2` through the write API,
    // it adheres to the set `max_resource_bytes`
    let file = epub2.read_resource_bytes("binary.file").unwrap_err();
    assert_max_bytes_exceeded(file.into(), KIB_8, "/EPUB/binary.file");

    let file_entry = epub2.manifest().by_id("binary-file").unwrap();
    let file = file_entry.read_bytes().unwrap_err();
    assert_max_bytes_exceeded(file.into(), KIB_8, "/EPUB/binary.file");
}
