use std::env;
use std::error::Error;
use std::path::PathBuf;
use zip_extensions::write::zip_create_from_directory;

const INPUT_EPUB_DIR: &str = "tests/ebooks/example_epub";
const OUTPUT_EPUB_FILE: &str = "example.epub";

/// Convenient script to convert the example epub directory into a `.epub` file.
fn main() -> Result<(), Box<dyn Error>> {
    let out_path = PathBuf::from(env::var("OUT_DIR").unwrap());

    zip_create_from_directory(
        &out_path.join(OUTPUT_EPUB_FILE),
        &PathBuf::from(INPUT_EPUB_DIR), // Argument must be &PathBuf
    )?;

    Ok(())
}
