/// EPUB integration tests
mod epub {
    mod manifest;
    mod metadata;
    mod reader;
    mod spine;
    mod toc;
    mod util;

    use crate::epub::util::TestEpub::{Epub3Dir, Epub3File};
    use rbook::Ebook;
    use rbook::ebook::manifest::{Manifest, ManifestEntry};
    use std::path::Path;

    #[test]
    fn test_comparison() {
        let epub_a = Epub3File.open();
        let epub_b = Epub3Dir.open();

        assert_eq!(
            epub_a, epub_b,
            "Note: Ensure `ebooks/example_epub` is identical to `example.epub`; update the files if necessary."
        );
    }

    #[test]
    fn test_read_resources() {
        let epub = Epub3File.open();
        let location = Path::new(util::EPUB3_DIR);

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
        let epub = Epub3File.open();
        let location = Path::new(util::EPUB3_DIR);

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
