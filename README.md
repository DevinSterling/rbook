# rbook

[![Crates.io](https://img.shields.io/crates/v/rbook.svg?logo=rust&style=flat-square)](https://crates.io/crates/rbook)
[![Documentation](https://img.shields.io/badge/documentation-latest%20release-19e.svg?logo=docs.rs&style=flat-square)](https://docs.rs/rbook)
[![License](https://img.shields.io/badge/license-Apache%202.0-maroon?logo=apache&style=flat-square)](LICENSE)

![rbook](https://raw.githubusercontent.com/DevinSterling/devinsterling-com/master/public/images/rbook/rbook.png)

> A fast, format-agnostic, ergonomic ebook library with a focus on EPUB.

The primary goal of `rbook` is to provide an easy-to-use high-level API for reading, creating, and modifying ebooks.
Most importantly, this library is designed with future formats in mind
(`CBZ`, `FB2`, `MOBI`, etc.) via core traits defined within the [ebook](https://docs.rs/rbook/latest/rbook/ebook) 
and [reader](https://docs.rs/rbook/latest/rbook/reader) module, allowing all formats to share the same "base" API.

## Documentation
- [API documentation](https://docs.rs/rbook)
- [Changelog](CHANGELOG.md)

## Features
Here is a non-exhaustive list of the features `rbook` provides:

| Feature                     | Overview                                                                                                        | Documentation                                                        |
|-----------------------------|-----------------------------------------------------------------------------------------------------------------|----------------------------------------------------------------------|
| **EPUB 2 and 3**            | Simple and advanced read/write view of EPUB `2` and `3` formats.                                                | [epub module](https://docs.rs/rbook/latest/rbook/ebook/epub)         |
| **Reader**                  | Random‐access or sequential iteration over readable content.                                                    | [reader module](https://docs.rs/rbook/latest/rbook/reader)           |
| **Detailed Types**          | Abstractions built on expressive traits and types.                                                              |                                                                      |
| **Metadata**                | Typed access to dates, titles, creators, publishers, languages, tags, roles, attributes, and more.              | [metadata module](https://docs.rs/rbook/latest/rbook/ebook/metadata) |
| **Manifest**                | Lookup and traverse contained resources such as readable content (XHTML) and images.                            | [manifest module](https://docs.rs/rbook/latest/rbook/ebook/manifest) |
| **Spine**                   | Chronological reading order and preferred page direction.                                                       | [spine module](https://docs.rs/rbook/latest/rbook/ebook/spine)       |
| **Table of Contents (ToC)** | Navigation points, including the EPUB 2 guide and EPUB 3 landmarks.                                             | [toc module](https://docs.rs/rbook/latest/rbook/ebook/toc)           |
| **Resources**               | On-demand retrieval of bytes or strings for any manifest resource; data is not loaded up-front until requested. | [resource module](https://docs.rs/rbook/latest/rbook/ebook/resource) |

### Default crate features
These are toggleable features for `rbook` that are
enabled by default in a project's `Cargo.toml` file:

| Feature        | Description                                                                                           |
|----------------|-------------------------------------------------------------------------------------------------------|
| **write**      | Creation and modification of EPUB `2` and `3` formats.                                                |
| **prelude**    | Convenience [prelude](https://docs.rs/rbook/latest/rbook/prelude) ***only*** including common traits. |
| **threadsafe** | Enables `Send` + `Sync` constraint for `Epub`.                                                        |

## Usage
`rbook` can be used by adding it as a dependency in a project's `Cargo.toml` file:
```toml
[dependencies]
rbook = "0.7.1"                                           # With default features
# rbook = { version = "0.7.1", default-features = false } # Excluding default features
```

## WebAssembly
The `wasm32-unknown-unknown` target is supported by default.

## Examples
### Opening and reading an EPUB file
```rust
use rbook::Epub;

fn main() {
    // Open an epub from a file or directory
    // * `Read + Seek` implementations supported via `read(...)` for byte streams/buffers
    let epub = Epub::open("example.epub").unwrap();

    // Create a reader instance 
    // * Configurable via `reader_builder()`
    for data_result in epub.reader() {
        let data = data_result.unwrap();
        let kind = data.manifest_entry().kind();
        assert_eq!("application/xhtml+xml", kind.as_str());
        assert_eq!("xhtml", kind.subtype());

        // Print the readable content
        println!("{}", data.content());
    }
}
```
### Accessing metadata: Retrieving the main title
```rust
use rbook::Epub;
use rbook::ebook::metadata::{LanguageKind, TitleKind};

fn main() {
    let epub = Epub::options()
        .strict(true) // Enable strict checks (`false` by default)
        .skip_toc(true) // Skips ToC-related parsing, such as toc.ncx (`false` by default)
        .open("example.epub")
        .unwrap();    

    // Retrieve the main title (all titles retrievable via `titles()`)
    let title = epub.metadata().title().unwrap();
    assert_eq!("Example EPUB", title.value());
    assert_eq!(TitleKind::Main, title.kind());

    // Retrieve the first alternate script of a title
    let alternate_script = title.alternate_scripts().next().unwrap();
    assert_eq!("サンプルEPUB", alternate_script.value());
    assert_eq!("ja", alternate_script.language().scheme().code());
    assert_eq!(LanguageKind::Bcp47, alternate_script.language().kind());
}
```
### Accessing metadata: Retrieving the year and first creator
```rust
use rbook::Epub;
use rbook::ebook::metadata::LanguageKind;

fn main() {
    // If only metadata is needed, skipping helps quicken parsing time and reduce space.
    let epub = Epub::options()
        // These flags are `false` by default
        .skip_toc(true)
        .skip_manifest(true)
        .skip_spine(true)
        .open("example.epub")
        .unwrap();

    // Retrieve the published date and time
    let published = epub.metadata().published().unwrap();
    assert_eq!(2023, published.date().year());
    assert!(published.time().is_local());

    // Retrieve the first creator
    let creator = epub.metadata().creators().next().unwrap();
    assert_eq!("John Doe", creator.value());
    assert_eq!(Some("Doe, John"), creator.file_as());
    assert_eq!(0, creator.order());

    // Retrieve the main role of a creator (all roles retrievable via `roles()`)
    let role = creator.main_role().unwrap();
    assert_eq!("aut", role.code());
    assert_eq!(Some("marc:relators"), role.source());

    // Retrieve the first alternate script of a creator
    let alternate_script = creator.alternate_scripts().next().unwrap();
    assert_eq!("山田太郎", alternate_script.value());
    assert_eq!("ja", alternate_script.language().scheme().code());
    assert_eq!(LanguageKind::Bcp47, alternate_script.language().kind());
}
```
### Extracting images from the manifest
```rust
use rbook::Epub;
use std::fs::{self, File};
use std::path::Path;

fn main() {
    let epub = Epub::open("example.epub").unwrap();
    
    // Create an output directory for the extracted images
    let out = Path::new("extracted_images");
    fs::create_dir_all(&out).unwrap();
    
    for image in epub.manifest().images() {
        // Extract the filename from the href and write to disk
        let filename = image.href().name().decode(); // Decode as EPUB hrefs are percent-encoded

        // Copy the raw image bytes
        let file = File::create(out.join(&*filename)).unwrap();
        image.copy_bytes(file).unwrap();
    }
}
```
### Modifying an existing EPUB
```rust
use rbook::epub::{Epub, EpubChapter};
use rbook::ebook::errors::EbookResult;

fn main() -> EbookResult<()> {
    Epub::open("old.epub")?
        .edit()
        // Appending a creator
        .creator("Jane Doe")
        // Appending a chapter
        .chapter(EpubChapter::new("Chapter 1337").xhtml_body("1337"))
        // Setting the modified date to now
        .modified_now()
        .write()
        .compression(9)
        .save("new.epub")
}
```
### Creating a backwards-compatible EPUB 3 file

> This example uses the [high-level builder API](https://docs.rs/rbook/latest/rbook/ebook/epub/struct.EpubEditor.html).  
> See the [epub module](https://docs.rs/rbook/latest/rbook/ebook/epub)
> for lower-level control over the manifest, spine, etc.

```rust
use rbook::epub::{Epub, EpubChapter};
use rbook::ebook::errors::EbookResult;
use rbook::ebook::toc::TocEntryKind;
use std::path::Path;

const XHTML: &[u8] = b"<xhtml>...</xhtml>"; // Example data
    
fn main() -> EbookResult<()> {
    Epub::builder()
        .identifier("urn:example")
        .title("Doe Story")
        .creator(["John Doe", "Jane Doe"])
        .language("en")
        // Reference a file stored on disk or provide in-memory bytes
        .cover_image(("cover.png", Path::new("local/file/cover.png")))
        .chapter([
            // Standard Chapter (Auto-generates href/filename "volume_i.xhtml")
            EpubChapter::new("Volume I").xhtml(XHTML).children(
                // Providing an explicit href (v1c1.xhtml)
                EpubChapter::new("I: Intro")
                    .kind(TocEntryKind::Introduction)
                    .href("v1c1.xhtml")
                    .xhtml_body("<p>Basic text</p>"),
            ),
            EpubChapter::new("Volume II").children([
                // Referencing an XHTML file stored on the OS file system
                EpubChapter::new("I").href("v2/c1.xhtml").xhtml(Path::new("path/to/c1.xhtml")),
                // Navigation-only entry linking to a fragment in another chapter
                EpubChapter::new("Section 1").href("v2/c1.xhtml#section-1"),
                // Resource included in spine/manifest but omitted from ToC
                EpubChapter::unlisted("v3extras.xhtml").xhtml(XHTML),
            ]),
        ])
        .write()
        .compression(0)
        // Save to disk or alternatively write to memory
        .save("doe_story.epub")
}
```

> [!TIP]
> More examples are available in the documentation: <https://docs.rs/rbook>

## MSRV
The current Minimum Supported Rust Version is [1.88.0](https://releases.rs/docs/1.88.0/).

## License
Licensed under [**Apache License, Version 2.0**](LICENSE).

### Contribution
Unless you explicitly state otherwise, any contribution intentionally submitted 
for inclusion in the work by you, as defined in the Apache-2.0 license,
shall be licensed as above, without any additional terms or conditions.