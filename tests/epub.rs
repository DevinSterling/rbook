/// EPUB integration tests
mod epub {
    mod manifest;
    mod metadata;
    mod reader;
    mod spine;
    mod toc;
    mod util;
    #[cfg(feature = "write")]
    mod write;

    use crate::epub::util::TestEpub::{Epub3Dir, Epub3File};
    use rbook::ebook::errors::ArchiveError;
    use std::path::Path;

    #[test]
    fn test_comparison() {
        let epub_a = Epub3File.open_strict();
        let epub_b = Epub3Dir.open_strict();

        assert_eq!(
            epub_a, epub_b,
            "Note: Ensure `ebooks/example_epub` is identical to `example.epub`; update the files if necessary."
        );
    }

    #[test]
    fn test_read_resources() {
        let epub = Epub3File.open_strict();
        let location = Path::new(util::EPUB3_DIR);

        for item in epub.manifest().iter() {
            // Remove absolute prefix to resolve outside the epub container
            let absolute_href = item.href().decode();
            let contained_file = absolute_href.strip_prefix('/').unwrap();
            let actual_file = location.join(contained_file);

            let content_a = std::fs::read(actual_file).unwrap();
            let content_b = epub.read_resource_bytes(item).unwrap();

            assert_eq!(content_a, content_b);
        }
    }

    #[test]
    fn test_read_resources_str() {
        let epub = Epub3File.open_strict();
        let location = Path::new(util::EPUB3_DIR);

        for item in epub.manifest().readable_content() {
            // Remove absolute prefix to resolve outside the epub container
            let absolute_href = item.href().decode();
            let contained_file = absolute_href.strip_prefix('/').unwrap();
            let actual_file = location.join(contained_file);

            let content_a = std::fs::read_to_string(actual_file).unwrap();
            let content_b = epub.read_resource_str(item).unwrap();

            assert_eq!(content_a, content_b);
        }
    }

    /// Test path traversal when using a directory as the backend
    #[test]
    fn test_path_traversal() {
        const MAX_DEPTH: usize = 8;
        const SEPARATORS: [&str; 6] = ["/", "\\", "%2f", "%5c", "%252f", "%255c"];
        const BACKTRACKS: [&str; 3] = ["..", "%2e", "%252e"];
        const TARGET: &str = "Cargo.toml";

        // Epub3Dir is contained within: tests/ebooks/example_epub
        // package directory is: tests/ebooks/example_epub/EPUB
        let epub = Epub3Dir.open_strict();

        let project_dir = Path::new(env!("CARGO_MANIFEST_DIR"));
        let secret = project_dir.join(TARGET);

        // Ensure the target file exists
        assert!(secret.exists(), "`Cargo.toml` should exist");

        // Attempt to reach it via various traversal strings
        let mut attacks = vec![secret.into_os_string().into_string().unwrap()];

        // Add unix and window-style paths
        for sep in SEPARATORS {
            for backtrack in BACKTRACKS {
                let parent_dir = format!("{backtrack}{sep}");

                for i in 1..MAX_DEPTH {
                    let traversal = parent_dir.repeat(i);

                    // Combinations:
                    // - "..//Cargo.toml"
                    // - "../Cargo.toml"
                    // - "/../Cargo.toml"
                    // - "//../Cargo.toml"
                    attacks.push(format!("{traversal}/{TARGET}"));
                    attacks.push(format!("{traversal}{TARGET}"));
                    attacks.push(format!("{sep}{traversal}{TARGET}"));
                    attacks.push(format!("{sep}{sep}{traversal}{TARGET}"));
                }
            }
        }

        for attack in attacks {
            let result = epub.read_resource_bytes(&attack);

            // Reading must fail
            assert!(
                matches!(result, Err(ArchiveError::InvalidResource { .. })),
                "Potential path traversal vulnerability: {attack}",
            );
        }
    }
}
