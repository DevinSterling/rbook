use rbook::Epub;
use rbook::epub::EpubOpenOptions;
use std::io::Cursor;

pub const EXAMPLE_UNZIPPED_EPUB: &str = "tests/ebooks/example_epub";
pub const EXAMPLE_ZIPPED_EPUB: &[u8] = include_bytes!("../../tests/ebooks/example.epub");

pub fn open_example_epub_dir() -> Epub {
    Epub::open(EXAMPLE_UNZIPPED_EPUB).unwrap()
}

pub fn open_example_epub_file() -> Epub {
    open_example_epub_file_with(EpubOpenOptions::new())
}

pub fn open_example_epub_file_with(builder: EpubOpenOptions) -> Epub {
    let cursor = Cursor::new(EXAMPLE_ZIPPED_EPUB);
    builder.read(cursor).unwrap()
}
