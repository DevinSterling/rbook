//! # rbook
//! - Repository: <https://github.com/DevinSterling/rbook>
//! - Documentation: <https://docs.rs/rbook>
//!
//! An ebook library that supports parsing and reading the epub format.
//!
//! ## Examples
//! Opening and reading an epub file:
//! ```
//! use rbook::Ebook;
//! use rbook::read::ContentType;
//!
//! // Creating an epub instance
//! let epub = rbook::Epub::new("tests/ebooks/moby-dick.epub").unwrap();
//!
//! // Retrieving the title
//! assert_eq!("Moby-Dick", epub.metadata().title().unwrap().value());
//!
//! // Creating a reader instance
//! let mut reader = epub.reader();
//!
//! // Printing the contents of each page
//! while let Some(Ok(content)) = reader.next_page() {
//!     let media_type = content.get_content(ContentType::MediaType).unwrap();
//!     assert_eq!("application/xhtml+xml", media_type);
//!     println!("{content}");
//! }
//!
//! assert_eq!(143, reader.current_index());
//! ```
//! Accessing metadata elements and attributes:
//! ```rust
//! # use rbook::Ebook;
//! # let epub = rbook::Epub::new("tests/ebooks/moby-dick.epub").unwrap();
//! let creators = epub.metadata().creators();
//! let creator = creators.first().unwrap();
//! assert_eq!("Herman Melville", creator.value());
//!
//! // Retrieving an attribute
//! let id = creator.get_attribute("id").unwrap();
//! assert_eq!("creator", id);
//!
//! // Refining (children) metadata and attributes
//! let role = creator.get_child("role").unwrap(); // Child metadata element
//! assert_eq!("aut", role.value());
//!
//! let scheme = role.get_attribute("scheme").unwrap(); // Attribute of an element
//! assert_eq!("marc:relators", scheme)
//! ```

mod archive;
mod formats;
mod utility;

#[cfg(feature = "reader")]
mod reader;
#[cfg(feature = "statistics")]
mod statistics;

pub use self::formats::{epub::Epub, xml, Ebook};
#[cfg(feature = "reader")]
pub use self::reader::Reader;
#[cfg(feature = "statistics")]
pub use self::statistics::Stats;

pub mod epub {
    //! Access to the contents that make up an epub.
    pub use super::formats::epub::{Guide, Manifest, Metadata, Spine, Toc};
}

pub mod result {
    //! Possible results and errors that can be encountered using rbook.
    pub use super::archive::ArchiveError;
    pub use super::formats::{EbookError, EbookResult};
    #[cfg(feature = "reader")]
    pub use super::reader::{ReaderError, ReaderResult};
}

#[cfg(feature = "reader")]
pub mod read {
    //! Access to reader contents.
    pub use super::reader::content::{Content, ContentType};
    pub use super::reader::ReaderIter;
}
