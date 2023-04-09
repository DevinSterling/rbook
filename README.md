# rbook

[![Crates.io](https://img.shields.io/crates/v/rbook.svg?style=flat-square)](https://crates.io/crates/rbook)
[![Documentation](https://img.shields.io/badge/documentation-latest%20release-19e.svg?style=flat-square)](https://docs.rs/rbook)

An ebook library that supports parsing and reading the epub format.

## Usage
Including default features:
```toml
[dependencies]
rbook = "0.5.0"
```
Excluding default features and selection:
```toml
[dependencies]
rbook = { version = "0.5.0", default-features = false, features = ["multi-thread"] }
```
Default features are the following:
- `reader`: Enables reading of the ebook file by file.
- `statistics`: Enables word/character counting.

Non-default optional features:
- `multi-thread`: Enables support for multithreaded environments.
## Examples
Other examples can be found in the ['tests'](tests) directory.

Opening and reading an epub file:
```rust
use rbook::Ebook;

fn main() {
    // Creating an epub instance
    let epub = rbook::Epub::new("example.epub").unwrap();

    // Retrieving the title
    assert_eq!("Jane and John", epub.metadata().title().unwrap().value());

    // Creating a reader instance
    let reader = epub.reader();

    // Printing the contents of each page
    for content_result in &reader {
        let content = content_result.unwrap();
        let media_type = content.get_content(ContentType::MediaType).unwrap();
        assert_eq!("application/xhtml+xml", media_type);
        println!("{}", content);
    }
}
```

Accessing metadata elements and attributes:
```rust
use rbook::Ebook;

fn main() {
    let epub = rbook::Epub::new("example.epub").unwrap();

    // Retrieving the first creator metadata element
    let creator = epub.metadata().creators().first().unwrap();
    assert_eq!("John Doe", creator.value());

    // Retrieving an attribute
    let id = creator.get_attribute("id").unwrap();
    assert_eq!("creator01", id);

    // Retrieving a child element
    let role = creator.get_child("role").unwrap();
    assert_eq!("aut", role.value());

    let scheme = role.get_attribute("scheme").unwrap();
    assert_eq!("marc:relators", scheme);
}
```

Alternative way of accessing elements:
```rust
use rbook::Ebook;
use rbook::xml::Find;

fn main() {
    let epub = rbook::Epub::new("example.epub").unwrap();

    // Retrieving the title
    let title = epub.metadata().find_value("title").unwrap();
    assert_eq!("Jane and John", title);
    
    // Retrieving creator
    let creator = epub.metadata().find_value("creator").unwrap();
    assert_eq!("John Doe", creator);

    // Retrieving role
    let role = epub.metadata().find_value("creator > role").unwrap();
    assert_eq!("aut", role);

    // Retrieving file-as
    let file_as = epub.metadata().find_value("creator > file-as").unwrap();
    assert_eq!("Doe, John", file_as);
}
```

Extracting images:
```rust
use rbook::Ebook;
use std::fs::{self, File};
use std::path::Path;

fn main() {
    let epub = rbook::Epub::new("example.epub").unwrap();

    let img_elements = epub.manifest().images();

    // Create new directory to store extracted images
    let dir = Path::new("extracted_images");
    fs::create_dir(&dir).unwrap();

    for img_element in img_elements {
        let img_href = img_element.value();

        // Retrieve image contents
        let img = epub.read_bytes_file(img_href).unwrap();

        // Retrieve file name from image href
        let file_name = Path::new(img_href).file_name().unwrap();

        // Create new file
        let mut file = File::create(dir.join(file_name)).unwrap();
        file.write_all(&img).unwrap();
    }
}
```

## Sample ebooks
Sample ebooks in the ['tests/ebooks'](tests/ebooks) directory are provided as is from 
[IDPF](https://idpf.github.io/epub3-samples/30/samples.html) under the 
[CC-BY-SA 3.0](http://creativecommons.org/licenses/by-sa/3.0/) license.