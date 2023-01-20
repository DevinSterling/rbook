//! # rbook
//! - Repository: <https://github.com/DevinSterling/rbook>
//! - Documentation: <https://docs.rs/rbook>
//!
//! An ebook library that supports parsing and reading the epub format.
//!
//! ## Examples
//! Opening and reading an epub file:
//! ```rust
//! use rbook::Ebook;
//! 
//! // Creating an epub instance
//! let epub = rbook::Epub::new("example.epub").unwrap();
//! 
//! // Retrieving the title
//! println!("Title = {}", epub.metadata().title().value());
//! 
//! // Creating a reader instance
//! let mut reader = epub.reader();
//! 
//! // Printing the contents of each page
//! while let Some(content) = reader.next_page() {
//!     println!("{content}")
//! }
//! ```
//! Accessing metadata elements and attributes:
//! ```rust
//! # use rbook::Ebook;
//! # let epub = rbook::Epub::new("example.epub").unwrap();
//! 
//! // Retrieving the first creator metadata element
//! let creator = epub.metadata().creators().unwrap().first().unwrap();
//! 
//! // Retrieving an attribute
//! let id = creator.get_attribute("id").unwrap();
//! 
//! // Retrieving a child element
//! let role = creator.get_child("role").unwrap();
//! let scheme = role.get_attribute("scheme").unwrap();
//! 
//! assert_eq!("id", id.name());
//! assert_eq!("creator01", id.value());
//! assert_eq!("aut", role.value());
//! assert_eq!("marc:relators", scheme.value());
//! ```

mod formats;
mod utility;
mod archive;
#[cfg(feature = "reader")]
mod reader;
#[cfg(feature = "statistics")]
mod statistics;

pub use self::{
    formats::{
        xml,
        Ebook,
        epub::Epub,
    }
};
#[cfg(feature = "reader")]
pub use self::reader::Reader;
#[cfg(feature = "statistics")]
pub use self::statistics::Stats;

pub mod epub {
    pub use super::formats::epub::{
        Metadata,
        Manifest,
        Spine,
        Guide,
        Toc,
    };
}

pub mod errors {
    pub use super::{
        formats::EbookError,
        archive::ArchiveError,
    };
    #[cfg(feature = "reader")]
    pub use super::reader::ReaderError;
}
