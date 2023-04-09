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
    reader.set_current_page(23).unwrap()?;
    assert_eq!(23, reader.current_index());

    // Grab the content of a page without updating the reader index
    let _content = reader.fetch_page(34).unwrap()?;
    assert_eq!(23, reader.current_index());

    // Updating the reader index by going to the next page
    reader.next_page().unwrap()?;
    assert_eq!(24, reader.current_index());

    // Updating the reader index using a string
    reader.set_current_page_str("toc.xhtml").unwrap()?;
    assert_eq!(143, reader.current_index());
    assert!(reader.next_page().is_none());

    reader.previous_page();
    assert_eq!(142, reader.current_index());

    Ok(())
}

#[test]
fn access_content_test() {
    let epub = rbook::Epub::new("tests/ebooks/moby-dick.epub").unwrap();

    let mut reader = epub.reader();

    let content = reader
        .next_page()
        .expect("Should have a next page")
        .expect("Associated content should be valid");

    println!("{content}");

    assert!(content.ends_with(b"</html>"));

    assert_eq!(
        "OPS/titlepage.xhtml",
        content.get_content(ContentType::Path).unwrap()
    );
    assert_eq!(
        "application/xhtml+xml",
        content.get_content(ContentType::MediaType).unwrap()
    );
    assert_eq!("titlepage", content.get_content(ContentType::Id).unwrap());
}

#[test]
fn read_all_while_test() {
    let epub = rbook::Epub::new("tests/ebooks/childrens-literature.epub").unwrap();

    let mut reader = epub.reader();

    // First page is current page
    let mut total_pages = 1;
    println!("{}", reader.current_page().unwrap());

    // Get next pages
    while let Some(content) = reader.next_page() {
        let content = content.expect("Content should be valid");
        println!("{content}");
        assert_eq!(total_pages, reader.current_index());
        total_pages += 1;
    }

    assert_eq!(3, total_pages);
}

#[test]
fn read_all_iterator_test() {
    let epub = rbook::Epub::new("tests/ebooks/childrens-literature.epub").unwrap();

    let reader = epub.reader();
    let mut total_pages = 0;

    for content in &reader {
        let content = content.expect("Content should be valid");
        total_pages += 1;
        println!("{content}");
    }

    assert_eq!(3, total_pages);
}

#[test]
fn read_all_iterator_test2() {
    let epub = rbook::Epub::new("tests/ebooks/moby-dick.epub").unwrap();

    let reader = epub.reader();

    for (i, content) in reader.iter().enumerate() {
        assert_eq!(
            content.expect("Content should be valid"),
            reader
                .fetch_page(i)
                .expect("Should be within bounds")
                .expect("Associated content should be valid"),
        );
    }
}
