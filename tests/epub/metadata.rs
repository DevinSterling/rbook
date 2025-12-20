use rbook::ebook::metadata::MetaEntry;
use rbook::{Ebook, Epub};

#[test]
fn test_self_closing_dc_format() {
    let epub = Epub::options()
        .strict(false)
        .open("tests/ebooks/epub3_relaxed")
        .unwrap();
    let metadata = epub.metadata();
    let format_entries: Vec<_> = metadata.by_property("dc:format").collect();
    assert_eq!(format_entries.len(), 1);
    assert_eq!(format_entries[0].value(), "");
}
