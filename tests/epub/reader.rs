use crate::epub::open_example_epub_file;
use rbook::Ebook;
use rbook::epub::reader::{EpubReaderContent, EpubReaderSettings, LinearBehavior};
use rbook::reader::errors::ReaderResult;
use rbook::reader::{Reader, ReaderContent};
use wasm_bindgen_test::wasm_bindgen_test;

#[test]
#[wasm_bindgen_test]
fn reader_linear_behavior() {
    let epub = open_example_epub_file();
    #[rustfmt::skip]
    let linear_behaviors = [
        (
            LinearBehavior::Original,                // Spine entry positioning
            vec!["cover", "toc", "c1", "c1a", "c2"], // Expected spine entry order
        ),
        (LinearBehavior::LinearOnly, vec!["toc", "c1", "c2"]),
        (LinearBehavior::NonLinearOnly, vec!["cover", "c1a"]),
        (LinearBehavior::PrependNonLinear, vec!["cover", "c1a", "toc", "c1", "c2"]),
        (LinearBehavior::AppendNonLinear, vec!["toc", "c1", "c2", "cover", "c1a"]),
    ];

    for (linear_behavior, cases) in linear_behaviors {
        let mut reader =
            epub.reader_with(EpubReaderSettings::builder().linear_behavior(linear_behavior));

        assert_eq!(cases.len(), reader.len());

        for case in cases {
            assert_eq!(case, reader.next().unwrap().unwrap().spine_entry().idref());
        }
        assert!(reader.next().is_none());

        reader.next();
    }
}

#[test]
#[wasm_bindgen_test]
fn reader_cursor() -> ReaderResult<()> {
    let epub = open_example_epub_file();
    let mut reader = epub.reader();

    fn idref(content: EpubReaderContent) -> &str {
        content.spine_entry().idref()
    }

    assert_eq!(None, reader.current_position());

    // Jump
    assert_eq!("toc", idref(reader.read(1)?));
    assert_eq!(Some(1), reader.current_position());
    assert_eq!(3, reader.remaining());

    // Move Backward
    assert_eq!("cover", idref(reader.read_prev().unwrap()?));
    assert_eq!(Some(0), reader.current_position());
    assert_eq!(4, reader.remaining());

    // Move backward
    assert!(reader.read_prev().is_none());
    assert!(reader.read_prev().is_none());

    // Move forward
    assert_eq!("toc", idref(reader.next().unwrap()?));
    assert_eq!(Some(1), reader.current_position());
    assert_eq!(3, reader.remaining());

    // Jump
    assert_eq!("c2", idref(reader.read("c2")?));
    assert_eq!(Some(4), reader.current_position());
    assert_eq!(0, reader.remaining());
    assert!(reader.next().is_none());
    assert!(reader.next().is_none());
    assert_eq!(Some(4), reader.current_position());

    // Move backward
    assert_eq!("c1a", idref(reader.read_prev().unwrap()?));
    assert_eq!(Some(3), reader.current_position());
    assert_eq!(1, reader.remaining());

    // Jump
    assert_eq!("c1", idref(reader.read(2)?));
    assert_eq!(Some(2), reader.current_position());
    assert_eq!(2, reader.remaining());

    assert_eq!(5, reader.len());

    Ok(())
}
