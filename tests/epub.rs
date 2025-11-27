/// EPUB integration tests
mod epub {
    mod manifest;
    mod reader;
    mod spine;
    mod toc;

    use rbook::ebook::manifest::{Manifest, ManifestEntry};
    use rbook::epub::EpubOpenOptions;
    use rbook::{Ebook, Epub};
    use std::io::Cursor;
    use std::path::Path;

    const EXAMPLE_EPUB: &str = "tests/ebooks/example_epub";

    fn open_example_epub_dir() -> Epub {
        Epub::open(EXAMPLE_EPUB).unwrap()
    }

    fn open_example_epub_file() -> Epub {
        open_example_epub_file_with(EpubOpenOptions::new())
    }

    fn open_example_epub_file_with(builder: EpubOpenOptions) -> Epub {
        let bytes = include_bytes!(concat!(env!("OUT_DIR"), "/example.epub"));
        let cursor = Cursor::new(bytes);
        builder.read(cursor).unwrap()
    }

    #[test]
    fn test_comparison() {
        let epub_a = open_example_epub_file();
        let epub_b = open_example_epub_dir();

        assert_eq!(epub_a, epub_b);
    }

    #[test]
    fn test_read_resources() {
        let epub = open_example_epub_file();
        let location = Path::new(EXAMPLE_EPUB);

        for item in epub.manifest().entries() {
            // Remove absolute prefix to resolve outside the epub container
            let absolute_href = item.href().decode();
            let contained_file = absolute_href.strip_prefix('/').unwrap();
            let actual_file = location.join(contained_file);

            let content_a = std::fs::read(actual_file).unwrap();
            let content_b = epub.read_resource_bytes(item.resource()).unwrap();

            assert_eq!(content_a, content_b);
        }
    }

    #[test]
    fn test_read_resources_str() {
        let epub = open_example_epub_file();
        let location = Path::new(EXAMPLE_EPUB);

        for item in epub.manifest().readable_content() {
            // Remove absolute prefix to resolve outside the epub container
            let absolute_href = item.href().decode();
            let contained_file = absolute_href.strip_prefix('/').unwrap();
            let actual_file = location.join(contained_file);

            let content_a = std::fs::read_to_string(actual_file).unwrap();
            let content_b = epub.read_resource_str(item.resource()).unwrap();

            assert_eq!(content_a, content_b);
        }
    }
}
