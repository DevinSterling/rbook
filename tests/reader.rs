use rbook::Ebook;
use rbook::errors::ReaderError;

#[test]
fn reader_test() -> Result<(), ReaderError> {
    let epub = rbook::Epub::new("example.epub").unwrap();

    // Get a reader instance
    let mut reader = epub.reader();
    assert_eq!(59, reader.page_count());

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
    reader.set_current_page_str("newsletterSignup.xhtml")?;
    assert_eq!(58, reader.current_index());
    assert_eq!(None, reader.next_page());

    reader.previous_page();
    assert_eq!(57, reader.current_index());

    Ok(())
}

#[test]
fn read_all_test() {
    let epub = rbook::Epub::new("example.epub").unwrap();

    let mut reader = epub.reader();

    while let Some(content) = reader.next_page() {
        println!("{content}");
    }
}