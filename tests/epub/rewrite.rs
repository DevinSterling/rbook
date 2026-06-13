use crate::epub::util;
use crate::epub::util::TestEpub::Epub3File;
use rbook::ebook::errors::EbookResult;
use rbook::epub::rewrite::{EpubRewriteOptions, PathRewrite};
use rbook::reader::errors::ReaderResult;
use wasm_bindgen_test::wasm_bindgen_test;

#[test]
#[wasm_bindgen_test]
fn test_manifest_entry_read_str_with() -> EbookResult<()> {
    let epub = Epub3File.open_strict();
    let rewrite = EpubRewriteOptions::default().rewrite_paths(PathRewrite::root_relative());

    let entry = epub.manifest().by_id("toc").unwrap();
    let xhtml = entry.read_str_with(&rewrite)?;

    for path in util::xml::extract_attributes(&xhtml, &[b"href", b"src"]) {
        assert!(path.starts_with("/"));
    }
    Ok(())
}

#[test]
#[wasm_bindgen_test]
fn test_epub_read_resource_str_with() -> EbookResult<()> {
    const CSS: &str = "body { color: red; }";
    let epub = Epub3File.open_strict();
    let rewrite = EpubRewriteOptions::default().inject_css(CSS);

    let xhtml = epub.read_resource_str_with("/toc.xhtml", &rewrite)?;
    let inserted = format!("<style>/*<![CDATA[*/{CSS}/*]]>*/</style>");
    assert!(xhtml.contains(&inserted));
    Ok(())
}

#[test]
#[wasm_bindgen_test]
fn test_reader_rewrite_paths() -> ReaderResult<()> {
    let prefixes = ["localhost:8080/", "ebook://", "prefix-", "/"];
    let epub = Epub3File.open_strict();

    for prefix in prefixes {
        let reader = epub
            .reader_builder()
            .rewrite(EpubRewriteOptions::default().rewrite_paths(PathRewrite::prefix(prefix)))
            .create();

        for data_result in reader {
            let data = data_result?;
            let xhtml = data.content();

            for path in util::xml::extract_attributes(xhtml, &[b"href", b"src"]) {
                // Strip query/fragment;
                let end = path.find(['#', '?']).unwrap_or(path.len());
                let path = &path[..end];

                // check for prefix
                assert!(path.starts_with(prefix));
                // strip prefix and make path "absolute" to container root
                let absolute = format!("/{}", path.strip_prefix(prefix).unwrap());
                // Ensure location is valid
                epub.read_resource_str(absolute)?;
            }
        }
    }
    Ok(())
}

#[test]
#[wasm_bindgen_test]
fn test_reader_inject_css() -> ReaderResult<()> {
    let injected_css = ["nav > ol { list-style: none }", ";</'&>comment<\">"];
    let epub = Epub3File.open_strict();

    for css in injected_css {
        let reader = epub
            .reader_builder()
            .rewrite(EpubRewriteOptions::default().inject_css(css))
            .create();

        for data_result in reader {
            let data = data_result?;
            let xhtml = data.content();
            let inserted = format!("<style>/*<![CDATA[*/{css}/*]]>*/</style>");

            // Check for inserted style element
            assert!(xhtml.contains(&inserted));
        }
    }
    Ok(())
}
