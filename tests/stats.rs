use rbook::{Ebook, Stats};

#[test]
fn stats_test() {
    let epub = rbook::Epub::new("tests/ebooks/moby-dick.epub").unwrap();

    // Read the contents of a file
    let file_content = epub.read_bytes_file("chapter_037.xhtml").unwrap();

    let word_count = epub.count_words(&file_content).unwrap();
    assert_eq!(529, word_count);

    let char_count = epub.try_count_total_chars().unwrap();
    assert_eq!(1242359, char_count);
}