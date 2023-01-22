# rbook

[![Crates.io](https://img.shields.io/crates/v/rbook.svg?style=flat-square)](https://crates.io/crates/rbook)
[![Documentation](https://img.shields.io/badge/documentation-latest%20release-19e.svg?style=flat-square)](https://docs.rs/rbook)

An ebook library that supports parsing and reading the epub format.

## Usage
Including default features:
```toml
[dependencies]
rbook = "0.1.2"
```
Excluding default features:
```toml
[dependencies]
rbook = { version = "0.1.2", default-features = false }
```
Default features are the following:
- `reader`: Enables reading of the ebook file by file
- `statistics`: Enables word/character counting
## Examples
Other examples can be found in the ['tests'](tests) directory.

Opening and reading an epub file:
```rust
use rbook::Ebook;

fn main() {
    // Creating an epub instance
    let epub = rbook::Epub::new("example.epub").unwrap();

    // Retrieving the title
    println!("Title = {}", epub.metadata().title().value());

    // Creating a reader instance
    let mut reader = epub.reader();

    // Printing the contents of each page
    while let Some(content) = reader.next_page() {
        println!("{}", content)
    }
}
```

Accessing metadata elements and attributes:
```rust
use rbook::Ebook;

fn main() {
    let epub = rbook::Epub::new("example.epub").unwrap();

    // Retrieving the first creator metadata element
    let creator = epub.metadata().creators().unwrap().first().unwrap();

    // Retrieving an attribute
    let id = creator.get_attribute("id").unwrap();

    // Retrieving a child element
    let role = creator.get_child("role").unwrap();
    let scheme = role.get_attribute("scheme").unwrap();

    assert_eq!("id", id.name());
    assert_eq!("creator01", id.value());
    assert_eq!("aut", role.value());
    assert_eq!("marc:relators", scheme.value());
}
```

Extracting images:
```rust
use rbook::Ebook;

fn main() {
    let epub = rbook::Epub::new("example.epub").unwrap();

    let img_elements = epub.manifest().images().unwrap();

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