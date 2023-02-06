use rbook::read::ContentType;
use rbook::result::ReaderError;
use rbook::Ebook;

#[test]
fn reader_test() -> Result<(), ReaderError> {
    let epub = rbook::Epub::new("tests/ebooks/moby-dick.epub").unwrap();

    // Get a reader instance
    let mut reader = epub.reader();
    assert_eq!(144, reader.page_count());

    // Updating the reader index
    reader.set_current_page(23)?;
    assert_eq!(23, reader.current_index());

    // Grab the content of a page without updating the reader index
    let _content = reader.fetch_page(34)?;
    assert_eq!(23, reader.current_index());

    // Updating the reader index by going to the next page
    reader.try_next_page()?;
    assert_eq!(24, reader.current_index());

    // Updating the reader index using a string
    reader.set_current_page_str("toc.xhtml")?;
    assert_eq!(143, reader.current_index());
    assert_eq!(None, reader.next_page());

    reader.previous_page();
    assert_eq!(142, reader.current_index());

    Ok(())
}

#[test]
fn access_content_test() {
    let epub = rbook::Epub::new("tests/ebooks/moby-dick.epub").unwrap();

    let mut reader = epub.reader();

    let content = reader.next_page().unwrap();

    println!("{content}");

    assert!(content.ends_with("</html>"));

    assert_eq!(
        "OPS/titlepage.xhtml",
        content.get(ContentType::Path).unwrap()
    );
    assert_eq!(
        "application/xhtml+xml",
        content.get(ContentType::Type).unwrap()
    );
    assert_eq!("titlepage", content.get(ContentType::Id).unwrap());
}

#[test]
fn read_all_test() {
    let epub = rbook::Epub::new("tests/ebooks/childrens-literature.epub").unwrap();

    let mut reader = epub.reader();
    let mut count = 0;

    println!("{}", reader.current_page().unwrap());

    while let Some(content) = reader.next_page() {
        count += 1;

        println!("{content}");

        assert_eq!(count, reader.current_index());
    }

    assert_eq!(2, count);
}
