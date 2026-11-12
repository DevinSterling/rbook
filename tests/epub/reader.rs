use crate::epub::util::TestEpub::{Epub3File, Epub3Relaxed};
use rbook::epub::reader::{EpubReaderContent, LinearBehavior};
use rbook::reader::errors::{ReaderError, ReaderResult};
use wasm_bindgen_test::wasm_bindgen_test;

#[test]
#[wasm_bindgen_test]
fn test_reader_linear_behavior() {
    let epub = Epub3File.open_strict();
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
        let mut reader = epub
            .reader_builder()
            .linear_behavior(linear_behavior)
            .create();

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
fn test_reader_cursor() -> ReaderResult<()> {
    let epub = Epub3File.open_strict();
    let mut reader = epub.reader();

    fn idref(content: EpubReaderContent<'_>) -> &str {
        content.spine_entry().idref()
    }

    assert!(!reader.is_empty());
    assert_eq!(5, reader.len());
    assert_eq!(reader.len(), reader.remaining());
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

#[test]
#[wasm_bindgen_test]
fn test_reader_into_content_integrity() -> ReaderResult<()> {
    let epub = Epub3File.open_strict();

    // Ensure integrity
    for content_result in epub.reader() {
        let entry_a = content_result?;
        let entry_b = entry_a.clone();

        let href = entry_a.manifest_entry().href();
        let resource = epub.read_resource_str(href)?;

        assert_eq!(resource, entry_a.into_string());
        assert_eq!(resource.as_bytes(), entry_b.into_bytes());
    }

    Ok(())
}

#[test]
fn test_reader_unresolved_spine_entries() -> ReaderResult<()> {
    let epub = Epub3Relaxed.build(|b| b.strict(false));

    // By default, unresolved spine entries are retained and yield an error
    let results: Vec<_> = epub.reader().collect();
    assert_eq!(3, results.len());
    assert!(matches!(results[0], Err(ReaderError::Format(_))));
    assert_eq!(
        "chapter",
        results[1].as_ref().unwrap().spine_entry().idref()
    );
    assert!(matches!(results[2], Err(ReaderError::Format(_))));

    #[rustfmt::skip]
    let linear_behaviors = [
        (LinearBehavior::Original, vec!["chapter"]),
        (LinearBehavior::LinearOnly, vec!["chapter"]),
        (LinearBehavior::NonLinearOnly, vec![]),
        (LinearBehavior::PrependNonLinear, vec!["chapter"]),
        (LinearBehavior::AppendNonLinear, vec!["chapter"]),
    ];

    for (linear_behavior, cases) in linear_behaviors {
        let mut reader = epub
            .reader_builder()
            .linear_behavior(linear_behavior)
            .skip_unresolved(true)
            .create();

        assert_eq!(cases.len(), reader.len());
        assert_eq!(cases.len(), reader.size_hint().0);

        for (position, case) in cases.into_iter().enumerate() {
            let content = reader.next().unwrap()?;
            assert_eq!(case, content.spine_entry().idref());
            assert_eq!(position, content.position());
        }
        assert!(reader.next().is_none());
    }

    // Skipped entries are not addressable
    let mut reader = epub.reader_builder().skip_unresolved(true).create();
    assert_eq!(0, reader.seek("chapter")?);
    assert!(matches!(
        reader.seek("cover"),
        Err(ReaderError::NoMapping(_))
    ));
    assert!(matches!(
        reader.get(1),
        Err(ReaderError::OutOfBounds { .. })
    ));

    Ok(())
}
