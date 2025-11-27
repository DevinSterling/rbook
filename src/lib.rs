#![warn(missing_docs)]
//! - Repository: <https://github.com/DevinSterling/rbook>
//! - Documentation: <https://docs.rs/rbook>
//!
//! A fast, format-agnostic, ergonomic ebook library with a current focus on EPUB.
//!
//! The primary goal of `rbook` is to provide an easy-to-use high-level API for handling ebooks.
//! Most importantly, this library is designed with future formats in mind
//! (`CBZ`, `FB2`, `MOBI`, etc.) via core traits defined within the [`ebook`] and [`reader`]
//! module, allowing all formats to share the same "base" API.
//!
//! Traits such as [`Ebook`] allow formats to be handled generically.
//! For example, retrieving the data of a cover image agnostic to the
//! concrete format (e.g., [`Epub`]):
//! ```
//! # use rbook::Ebook;
//! # use rbook::ebook::manifest::{Manifest, ManifestEntry};
//! // Here `ebook` may be of any supported format.
//! fn cover_image_bytes<E: Ebook>(ebook: &E) -> Option<Vec<u8>> {
//!     // 1 - An ebook may not have a `cover_image` entry, hence the try operator (`?`).
//!     // 2 - `read_bytes` returns a `Result`; `ok()` coverts the result into `Option`.
//!     ebook.manifest().cover_image()?.read_bytes().ok()
//! }
//! ```
//!
//! # Features
//! Here is a non-exhaustive list of the features `rbook` provides:
//!
//! | Feature                                   | Overview                                                                                    |
//! |-------------------------------------------|---------------------------------------------------------------------------------------------|
//! | [**EPUB 2 and 3**](epub)                  | Read-only (for now) view of EPUB `2` and `3` formats.                                       |
//! | [**Streaming Reader**](reader)            | Random‐access or sequential iteration over readable content.                                |
//! | **Detailed Types**                        | Abstractions built on expressive traits and types.                                          |
//! | [**Metadata**](ebook::metadata)           | Typed access to titles, creators, publishers, languages, tags, roles, attributes, and more. |
//! | [**Manifest**](ebook::manifest)           | Lookup and traverse contained resources such as readable content (XHTML) and images.        |
//! | [**Spine**](ebook::spine)                 | Chronological reading order and preferred page direction.                                   |
//! | [**Table of Contents (ToC)**](ebook::toc) | Navigation points, including the EPUB 2 guide and EPUB 3 landmarks.                         |
//! | [**Resources**](ebook::resource)          | Retrieve bytes or UTF-8 strings for any manifest resource.                                  |
//!
//! ## Default crate features
//! These are toggleable features for `rbook` that are
//! enabled by default in a project's `cargo.toml` file:
//!
//! | Feature                | Description                                                 |
//! |------------------------|-------------------------------------------------------------|
//! | [**prelude**](prelude) | Convenience prelude ***only*** including common traits.     |
//! | **threadsafe**         | Enables constraint and support for [`Send`] + [`Sync`].     |
//!
//! Default features can be disabled and toggled selectively.
//! For example, omitting the `prelude` while retaining the `threadsafe` feature:
//! ```toml
//! [dependencies]
//! rbook = { version = "0.6.7", default-features = false, features = ["threadsafe"] }
//! ```
//!
//! # Opening an [`Ebook`]
//! `rbook` supports several methods to open an ebook ([`Epub`]):
//! - A directory containing the contents of an unzipped ebook:
//!   ```no_run
//!   # use rbook::Epub;
//!   let epub = Epub::open("/ebooks/unzipped_epub_dir");
//!   ```
//! - A file path:
//!   ```no_run
//!   # use rbook::Epub;
//!   let epub = Epub::open("/ebooks/zipped.epub");
//!   ```
//! - Or any implementation of [`Read`](std::io::Read) + [`Seek`](std::io::Seek)
//!   (and [`Send`] + [`Sync`] if the `threadsafe` feature is enabled):
//!   ```no_run
//!   # use rbook::epub::{Epub, EpubSettings};
//!   # let bytes_vec = Vec::new(); // Rea
//!   let cursor = std::io::Cursor::new(bytes_vec);
//!   let epub = Epub::read(cursor, EpubSettings::default());
//!   ```
//!
//! Aside from how the contents of an ebook are stored, settings can also be provided
//! to control parser behavior, such as [strictness](epub::EpubSettings::strict):
//! ```
//! // Import traits
//! use rbook::Ebook;
//! // use rbook::prelude::*; // or the prelude for convenient trait imports
//!
//! use rbook::epub::{Epub, EpubSettings};
//!
//! let epub = Epub::open_with(
//!     "tests/ebooks/example_epub",
//!     EpubSettings::builder().strict(false), // Disable strict checks (`true` by default)
//! ).unwrap();
//! ```
//! # Reading an [`Ebook`]
//! Reading the contents of an ebook is handled by a [`Reader`](reader::Reader),
//! which traverses end-user-readable resources in canonical order:
//! ```
//! # use rbook::{Ebook, Epub};
//! use rbook::reader::{Reader, ReaderContent};
//! # let epub = Epub::open("tests/ebooks/example_epub").unwrap();
//!
//! // Create a reader instance
//! let mut reader = epub.reader();
//!
//! // Print the readable content
//! while let Some(Ok(data)) = reader.read_next() {
//!     assert_eq!("application/xhtml+xml", data.manifest_entry().media_type());
//!     println!("{}", data.content());
//! }
//! ```
//! As with an ebook, a reader can receive settings to control behavior,
//! such as [linearity](epub::reader::EpubReaderSettings::linear_behavior):
//! ```
//! # use rbook::{Ebook, Epub};
//! use rbook::epub::reader::{EpubReaderSettings, LinearBehavior};
//!
//! # let epub = Epub::open("tests/ebooks/example_epub").unwrap();
//! let mut reader = epub.reader_with(
//!     // Make a reader omit non-linear content
//!     EpubReaderSettings::builder().linear_behavior(LinearBehavior::LinearOnly)
//! );
//! ```
//!
//! # Resource retrieval from an [`Ebook`]
//! All files such as text, images, and video are accessible within an ebook programmatically.
//!
//! The simplest way to access and retrieve resources from an ebook is through the
//! [`Manifest`](ebook::manifest::Manifest), specifically through its entries via
//! [`ManifestEntry::read_str`](ebook::manifest::ManifestEntry::read_str) and
//! [`ManifestEntry::read_bytes`](ebook::manifest::ManifestEntry::read_bytes):
//! ```
//! # use rbook::ebook::errors::EbookResult;
//! # use rbook::ebook::manifest::{Manifest, ManifestEntry};
//! # use rbook::{Ebook, Epub};
//! # fn main() -> EbookResult<()> {
//! # let epub = Epub::open("tests/ebooks/example_epub")?;
//! let manifest_entry = epub.manifest().cover_image().unwrap();
//! let cover_image_bytes = manifest_entry.read_bytes()?;
//!
//! // process bytes //
//! # Ok(())
//! # }
//! ```
//!
//! For finer grain control, the [`Ebook`] trait provides two methods
//! that accept a [`Resource`](ebook::resource::Resource) as an argument:
//! - [`Ebook::read_resource_str`] to retrieve the content as a UTF-8 string.
//! - [`Ebook::read_resource_bytes`] to retrieve the content in the form of raw bytes.
//! ```
//! # use rbook::ebook::errors::EbookResult;
//! # use rbook::ebook::manifest::{Manifest, ManifestEntry};
//! # use rbook::{Ebook, Epub};
//! # fn main() -> EbookResult<()> {
//! # let epub = Epub::open("tests/ebooks/example_epub")?;
//! let manifest_entry = epub.manifest().cover_image().unwrap();
//!
//! let bytes_a = epub.read_resource_bytes(manifest_entry.resource())?;
//! let bytes_b = epub.read_resource_bytes("/EPUB/img/cover.webm")?;
//!
//! assert_eq!(bytes_a, bytes_b);
//! # Ok(())
//! # }
//! ```
//!
//! All resource retrieval methods are fallible, and attempts to access malformed or missing
//! resources will return an [`EbookError::Archive`](ebook::errors::EbookError::Archive) error.
//!
//! ## See Also
//! - [`Epub`] documentation of `read_resource_*` methods for normalization details.
//!
//! # The [`prelude`]
//! `rbook` provides a prelude consisting **only** of traits for convenience.
//! It circumvents manually importing each trait and helps keep imports lean:
//! ```no_run
//! // Without the prelude (Verbose; manually importing each trait):
//! /*1*/ use rbook::Ebook;
//! /*2*/ use rbook::ebook::manifest::ManifestEntry;
//! /*3*/ use rbook::ebook::spine::{Spine, SpineEntry};
//! # use rbook::ebook::errors::EbookResult;
//!
//! // With the prelude, lines 1, 2, and 3 can be consolidated into `use rbook::prelude::*;`
//!
//! # fn main() -> EbookResult<()> {
//! # use rbook::ebook::errors::EbookResult;
//! // Retrieve the manifest entry associated with a spine entry:
//! let epub = rbook::Epub::open("tests/ebooks/example_epub")?;
//! let spine_entry = epub.spine().by_order(2).unwrap();
//! let manifest_entry_a = spine_entry.manifest_entry().unwrap();
//!
//! assert_eq!("c1", spine_entry.idref());
//! assert_eq!("c1", manifest_entry_a.id());
//! # Ok(())
//! # }
//! ```
//!
//! The idea of libraries providing a prelude is subjective and may not be desirable.
//! As such, it is set as a default crate feature that can be disabled inside a
//! project's `cargo.toml` file.
//! For example, omitting the `prelude` while retaining the `threadsafe` feature:
//! ```toml
//! [dependencies]
//! rbook = { version = "0.6.7", default-features = false, features = ["threadsafe"] }
//! ```
//!
//! # Examples
//! ## Accessing [`Metadata`](ebook::metadata::Metadata): Retrieving the main title
//! ```
//! # use rbook::{Ebook, Epub};
//! # use rbook::ebook::metadata::{Metadata, MetaEntry, Title, TitleKind, LanguageKind};
//! # let epub = Epub::open("tests/ebooks/example_epub").unwrap();
//! // Retrieve the main title (all titles retrievable via `titles()`)
//! let title = epub.metadata().title().unwrap();
//! assert_eq!("Example EPUB", title.value());
//! assert_eq!(TitleKind::Main, title.kind());
//!
//! // Retrieve the first alternate script of a title
//! let alternate_script = title.alternate_scripts().next().unwrap();
//! assert_eq!("サンプルEPUB", alternate_script.value());
//! assert_eq!("ja", alternate_script.language().scheme().code());
//! assert_eq!(LanguageKind::Bcp47, alternate_script.language().kind());
//! ```
//! ## Accessing [`Metadata`](ebook::metadata::Metadata): Retrieving the first creator
//! ```
//! # use rbook::{Ebook, Epub};
//! # use rbook::ebook::metadata::{Metadata, MetaEntry, Contributor, LanguageKind};
//! # let epub = Epub::open("tests/ebooks/example_epub").unwrap();
//! // Retrieve the first creator
//! let creator = epub.metadata().creators().next().unwrap();
//! assert_eq!("John Doe", creator.value());
//! assert_eq!(Some("Doe, John"), creator.file_as());
//! assert_eq!(0, creator.order());
//!
//! // Retrieve the main role of a creator (all roles retrievable via `roles()`)
//! let role = creator.main_role().unwrap();
//! assert_eq!("aut", role.code());
//! assert_eq!(Some("marc:relators"), role.source());
//!
//! // Retrieve the first alternate script of a creator
//! let alternate_script = creator.alternate_scripts().next().unwrap();
//! assert_eq!("山田太郎", alternate_script.value());
//! assert_eq!("ja", alternate_script.language().scheme().code());
//! assert_eq!(LanguageKind::Bcp47, alternate_script.language().kind());
//! ```
//! ## Extracting images from the [`Manifest`](ebook::manifest::Manifest)
//! ```no_run
//! use std::{fs, path::Path};
//! # use rbook::{Ebook, Epub};
//! # use rbook::ebook::manifest::{Manifest, ManifestEntry};
//! # let epub = Epub::open("tests/ebooks/example_epub").unwrap();
//!
//! // Create an output directory for the extracted images
//! let out = Path::new("extracted_images");
//! fs::create_dir_all(&out).unwrap();
//!
//! for image in epub.manifest().images() {
//!     // Retrieve the raw image bytes
//!     let bytes = image.read_bytes().unwrap();
//!
//!     // Extract the filename from the href and write to disk
//!     let filename = image.href().name().decode(); // Decode as EPUB hrefs may be URL-encoded
//!     fs::write(out.join(&*filename), bytes).unwrap();
//! }
//! ```
//! ## Accessing [`EpubManifest`](epub::manifest::EpubManifest) fallbacks
//! ```
//! # use rbook::{Ebook, Epub};
//! # use rbook::ebook::errors::EbookResult;
//! # use rbook::ebook::manifest::{Manifest, ManifestEntry};
//! # use rbook::ebook::metadata::MetaEntry;
//! # let epub = Epub::open("tests/ebooks/example_epub").unwrap();
//! // Fallbacks
//! let webm_cover = epub.manifest().cover_image().unwrap();
//! let kind = webm_cover.resource_kind();
//! assert_eq!(("image", "webm"), (kind.maintype(), kind.subtype()));
//!
//! // If the app does not support `webm`; fallback
//! let avif_cover = webm_cover.fallback().unwrap();
//! assert_eq!("image/avif", avif_cover.media_type());
//!
//! // If the app does not support `avif`; fallback
//! let png_cover = avif_cover.fallback().unwrap();
//! assert_eq!("image/png", png_cover.media_type());
//!
//! // No more fallbacks
//! assert_eq!(None, png_cover.fallback());
//! ```

mod parser;
mod util;

pub mod ebook;
pub mod reader;

pub use self::{ebook::Ebook, epub::Epub};
pub use crate::ebook::epub;

/// The rbook prelude for convenient imports of the core
/// [`ebook`] and [`reader`] **traits**.
///
/// This is a crate feature, `prelude`, that is enabled by default.
#[cfg(feature = "prelude")]
pub mod prelude {
    pub use crate::ebook::{
        Ebook,
        manifest::{Manifest, ManifestEntry},
        metadata::{Contributor, Identifier, Language, MetaEntry, Metadata, Tag, Title},
        spine::{Spine, SpineEntry},
        toc::{Toc, TocChildren, TocEntry},
    };
    pub use crate::reader::{Reader, ReaderContent};
}
