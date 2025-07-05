# rbook

[![Crates.io](https://img.shields.io/crates/v/rbook.svg?style=flat-square)](https://crates.io/crates/rbook)
[![Documentation](https://img.shields.io/badge/documentation-latest%20release-19e.svg?style=flat-square)](https://docs.rs/rbook)
[![License](https://img.shields.io/badge/license-Apache%202.0-maroon?style=flat-square)](LICENSE)

![rbook](https://raw.githubusercontent.com/DevinSterling/devinsterling-com/master/public/images/rbook/rbook.png)

A fast, format-agnostic, ergonomic ebook library with a focus on EPUB.

## Features
| Feature                     | Overview                                                                                    | Documentation                                                        |
|-----------------------------|---------------------------------------------------------------------------------------------|----------------------------------------------------------------------|
| **EPUB 2 and 3**            | Read-only (for now) view of EPUB `2` and `3` formats                                        | [epub module](https://docs.rs/rbook/latest/rbook/ebook/epub)         |
| **Reader**                  | Random‐access or sequential iteration over readable content.                                | [reader module](https://docs.rs/rbook/latest/rbook/reader)           |
| **Detailed Types**          | Abstractions built on expressive traits and types.                                          |                                                                      |
| **Metadata**                | Typed access to titles, creators, publishers, languages, tags, roles, attributes, and more. | [metadata module](https://docs.rs/rbook/latest/rbook/ebook/metadata) |
| **Manifest**                | Lookup and traverse contained resources such as readable content (XHTML) and images.        | [manifest module](https://docs.rs/rbook/latest/rbook/ebook/manifest) |
| **Spine**                   | Chronological reading order and preferred page direction                                    | [spine module](https://docs.rs/rbook/latest/rbook/ebook/spine)       |
| **Table of Contents (ToC)** | Navigation points, including the EPUB 2 guide and EPUB 3 landmarks.                         | [toc module](https://docs.rs/rbook/latest/rbook/ebook/toc)           |
| **Resources**               | Retrieve bytes or UTF-8 strings for any manifest resource                                   | [resource module](https://docs.rs/rbook/latest/rbook/ebook/resource) |

## Usage
```toml
[dependencies]
rbook = "0.6.3"                                           # with default features
# rbook = { version = "0.6.3", default-features = false } # excluding default features
```

Default crate features:
- `prelude`: Convenience prelude ***only*** including common traits.
- `threadsafe`: Enables constraint and support for `Send + Sync`.

## WebAssembly
The `wasm32-unknown-unknown` target is supported by default.

## Examples
- Opening and reading an EPUB file:
  ```rust
  use rbook::epub::{Epub, EpubSettings};
  use rbook::prelude::*; // Prelude for traits
  
  fn main() {
      // Opening an epub (file or directory)
      let epub = Epub::open_with(
         "tests/ebooks/example_epub",
         // Toggle strict EPUB checks (`true` by default)
         EpubSettings::builder().strict(false),
      ).unwrap();
  
      // Retrieving the title
      assert_eq!("Example EPUB", epub.metadata().title().unwrap().value());
      
      // Creating a reader instance:
      let mut reader = epub.reader(); // or `epub.reader_with(EpubReaderSettings)`
      // Printing the epub contents
      while let Some(Ok(data)) = reader.read_next() {
          let media_type = data.manifest_entry().media_type();
          assert_eq!("application/xhtml+xml", media_type);
          println!("{}", data.content());
      }
      
      assert_eq!(Some(4), reader.current_position());
  }
  ```
- Accessing a metadata element:
  ```rust
  use rbook::Epub;
  use rbook::prelude::*;
  
  fn main() {
      let epub = Epub::open("tests/ebooks/example_epub").unwrap();
      
      let creator = epub.metadata().creators().next().unwrap();
      assert_eq!("John Doe", creator.value());
      assert_eq!(Some("Doe, John"), creator.file_as());
      assert_eq!(0, creator.order());
      
      let role = creator.main_role().unwrap();
      assert_eq!("aut", role.code());
      assert_eq!(Some("marc:relators"), role.source());
  }
  ```
- Extracting images from the manifest:
  ```rust
  use rbook::Epub;
  use rbook::prelude::*;
  use std::fs::{self, File};
  use std::path::Path;
  use std::io::Write;
  
  fn main() {
      let epub = Epub::open("example.epub").unwrap();
      
      // Creating a new directory to store the extracted images
      let dir = Path::new("extracted_images");
      fs::create_dir(&dir).unwrap();
      
      for image in epub.manifest().images() {
          let img_href = image.href().as_str();
  
          // Retrieving the raw image data
          let img_data = image.read_bytes().unwrap();
  
          // Retrieving the file name from the image href
          let file_name = Path::new(img_href).file_name().unwrap();
  
          // Creating a new file to store the image data
          let mut file = fs::File::create(dir.join(file_name)).unwrap();
          file.write_all(&img_data).unwrap();
      }
  }
  ```
- Manifest media overlay and fallbacks:
  ```rust
  use rbook::Epub;
  use rbook::prelude::*;
  
  fn main() {
      let epub = Epub::open("tests/ebooks/example_epub").unwrap();
      
      // Retrieving media overlay information
      let chapter_1 = epub.manifest().by_id("c1").unwrap();
      let media_overlay = chapter_1.media_overlay().unwrap();
      let duration = media_overlay.refinements().by_property("media:duration").next().unwrap().value();
      assert_eq!("0:32:29", duration);
      
      // Fallbacks
      let webm_cover = epub.manifest().cover_image().unwrap();
      let kind = webm_cover.resource_kind();
      assert_eq!(("image", "webm"), (kind.maintype(), kind.subtype()));
      
      // If the app does not support `webm`; fallback
      let mut fallbacks = webm_cover.fallbacks();
      let avif_cover = fallbacks.next().unwrap();
      assert_eq!("image/avif", avif_cover.media_type());
      
      // If the app does not support `avif`; fallback
      let png_cover = fallbacks.next().unwrap();
      assert_eq!("image/png", png_cover.media_type());
      
      // No more fallbacks
      assert_eq!(None, fallbacks.next());
  }
  ```
- More examples are available in the documentation: <https://docs.rs/rbook>