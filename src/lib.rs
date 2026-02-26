//! - Repository: <https://github.com/DevinSterling/rbook>
//! - Documentation: <https://docs.rs/rbook>
//!
//! A fast, format-agnostic, ergonomic ebook library with a current focus on EPUB.
//!
//! The primary goal of `rbook` is to provide an easy-to-use high-level API
//! for reading, creating, and modifying ebooks.
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
//!     // 2 - `read_bytes` returns a `Result`; `ok()` converts the result into `Option`.
//!     ebook.manifest().cover_image()?.read_bytes().ok()
//! }
//! ```
//!
//! # Features
//! Here is a non-exhaustive list of the features `rbook` provides:
//!
//! | Feature                                   | Overview                                                                                                        |
//! |-------------------------------------------|-----------------------------------------------------------------------------------------------------------------|
//! | [**EPUB 2 and 3**](epub)                  | Read/write view of EPUB `2` and `3` formats.                                                                    |
//! | [**Streaming Reader**](reader)            | Random‐access or sequential iteration over readable content.                                                    |
//! | **Detailed Types**                        | Abstractions built on expressive traits and types.                                                              |
//! | [**Metadata**](ebook::metadata)           | Typed access to titles, creators, publishers, languages, tags, roles, attributes, and more.                     |
//! | [**Manifest**](ebook::manifest)           | Lookup and traverse contained resources such as readable content (XHTML) and images.                            |
//! | [**Spine**](ebook::spine)                 | Chronological reading order and preferred page direction.                                                       |
//! | [**Table of Contents (ToC)**](ebook::toc) | Navigation points, including the EPUB 2 guide and EPUB 3 landmarks.                                             |
//! | [**Resources**](ebook::resource)          | On-demand retrieval of bytes or strings for any manifest resource; data is not loaded up-front until requested. |                                  |
//!
//! ## Default crate features
//! These are toggleable features for `rbook` that are
//! enabled by default in a project's `Cargo.toml` file:
//!
//! | Feature                | Description                                             |
//! |------------------------|---------------------------------------------------------|
//! | **write**              | Creation and modification of EPUB `2` and `3` formats.  |
//! | [**prelude**](prelude) | Convenience prelude ***only*** including common traits. |
//! | **threadsafe**         | Enables `Send` + `Sync` constraint for `Epub`.          |
//!
//! Default features can be disabled and toggled selectively.
//! For example, only retaining the `threadsafe` default feature:
//! ```toml
//! [dependencies]
//! rbook = { version = "0.7.2", default-features = false, features = ["threadsafe"] }
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
//!   # use rbook::Epub;
//!   # let bytes = Vec::new();
//!   let cursor = std::io::Cursor::new(bytes);
//!   let epub = Epub::options().read(cursor);
//!   ```
//!
//! Aside from how the contents of an ebook are stored, options can also be given
//! to control parser behavior, such as [strictness](epub::EpubOpenOptions::strict):
//! ```
//! # use rbook::Epub;
//! let epub = Epub::options()
//!     .strict(true) // Enable strict checks (`false` by default)
//!     // If only metadata is needed, skipping helps quicken parsing time and reduce space.
//!     .skip_toc(true) // Skips ToC-related parsing, such as toc.ncx (`false` by default)
//!     .skip_manifest(true) // Skips manifest-related parsing (`false` by default)
//!     .skip_spine(true) // Skips spine-related parsing (`false` by default)
//!     .open("tests/ebooks/example_epub")
//!     .unwrap();
//! ```
//! # Reading an [`Ebook`]
//! Reading the contents of an ebook is handled by a [`Reader`](reader::Reader),
//! which traverses end-user-readable resources in canonical order:
//! ```
//! # use rbook::Epub;
//! # let epub = Epub::open("tests/ebooks/example_epub").unwrap();
//! // Create a reader instance
//! let mut reader = epub.reader();
//!
//! // Print the readable content
//! while let Some(Ok(data)) = reader.read_next() {
//!     let kind = data.manifest_entry().kind();
//!
//!     assert_eq!("application/xhtml+xml", kind.as_str());
//!     assert_eq!("xhtml", kind.subtype());
//!     println!("{}", data.content());
//! }
//! ```
//! Prior to creation, a reader can receive options to control its behavior,
//! such as [linearity](epub::reader::EpubReaderOptions::linear_behavior):
//! ```
//! # use rbook::Epub;
//! use rbook::epub::reader::LinearBehavior;
//!
//! # let epub = Epub::open("tests/ebooks/example_epub").unwrap();
//! let mut reader = epub.reader_builder()
//!                      // Make a reader omit non-linear content
//!                      .linear_behavior(LinearBehavior::LinearOnly)
//!                      .create();
//! ```
//!
//! # Resource retrieval from an [`Ebook`]
//! All files such as text, images, and video are accessible within an ebook programmatically.
//!
//! The simplest way to access and retrieve resources from an ebook is through the
//! [`Manifest`](ebook::manifest::Manifest), specifically through its entries via:
//! - [`ManifestEntry::copy_bytes`](ebook::manifest::ManifestEntry::copy_bytes)
//!   to copy the content directly into any [`Write`](std::io::Write) implementation.
//! - [`ManifestEntry::read_bytes`](ebook::manifest::ManifestEntry::read_bytes)
//!   to retrieve the content as bytes ([`Vec<u8>`](Vec)).
//! - [`ManifestEntry::read_str`](ebook::manifest::ManifestEntry::read_str)
//!   to retrieve the content as a [`String`].
//! ```
//! # use rbook::Epub;
//! # fn main() -> rbook::ebook::errors::EbookResult<()> {
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
//! - [`Ebook::copy_resource`]
//! - [`Ebook::read_resource_bytes`]
//! - [`Ebook::read_resource_str`]
//! ```
//! # use rbook::Epub;
//! # fn main() -> rbook::ebook::errors::EbookResult<()> {
//! # let epub = Epub::open("tests/ebooks/example_epub")?;
//! let manifest_entry = epub.manifest().cover_image().unwrap();
//!
//! let bytes_a = epub.read_resource_bytes(manifest_entry)?;
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
//! - [`Epub::copy_resource`] for normalization details.
//!
//! # Examples
//! ## Accessing [`Metadata`](ebook::metadata::Metadata): Retrieving the main title
//! ```
//! # use rbook::Epub;
//! # use rbook::ebook::metadata::{TitleKind, LanguageKind};
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
//! ## Accessing [`Metadata`](ebook::metadata::Metadata): Retrieving the date and first creator
//! ```
//! # use rbook::Epub;
//! # use rbook::ebook::metadata::LanguageKind;
//! # let epub = Epub::open("tests/ebooks/example_epub").unwrap();
//! // Retrieve the publication year
//! let published = epub.metadata().published().unwrap();
//! assert_eq!(2023, published.date().year());
//! assert_eq!(1, published.date().month());
//! assert_eq!(25, published.date().day());
//! assert!(published.time().is_local());
//!
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
//! use std::fs::{self, File};
//! use std::path::Path;
//! # use rbook::Epub;
//! # let epub = Epub::open("tests/ebooks/example_epub").unwrap();
//!
//! // Create an output directory for the extracted images
//! let out = Path::new("extracted_images");
//! fs::create_dir_all(&out).unwrap();
//!
//! for image in epub.manifest().images() {
//!     // Extract the filename from the href and write to disk
//!     let filename = image.href().name().decode(); // Decode EPUB hrefs are percent-encoded
//!
//!     // Copy the raw image bytes
//!     let mut file = File::create(out.join(&*filename)).unwrap();
//!     image.copy_bytes(&mut file).unwrap();
//! }
//! ```
//! ## Accessing [`EpubManifest`](epub::manifest::EpubManifest) fallbacks
//! ```
//! # use rbook::Epub;
//! # let epub = Epub::open("tests/ebooks/example_epub").unwrap();
//! // Fallbacks
//! let webm_cover = epub.manifest().cover_image().unwrap();
//! let kind = webm_cover.kind();
//! assert_eq!(("image", "webm"), (kind.maintype(), kind.subtype()));
//!
//! // If the program does not support `webm`, fallback
//! let avif_cover = webm_cover.fallback().unwrap();
//! assert_eq!("image/avif", avif_cover.media_type());
//!
//! // If the program does not support `avif`, fallback
//! let png_cover = avif_cover.fallback().unwrap();
//! assert_eq!("image/png", png_cover.media_type());
//!
//! // No fallbacks remaining
//! assert_eq!(None, png_cover.fallback());
//! ```
//! ## [Editing](Epub::edit) an [`Epub`]
//! ```no_run
//! # #[cfg(feature = "write")]
//! # {
//! # use rbook::Epub;
//! # fn main() -> rbook::ebook::errors::EbookResult<()> {
//! use rbook::epub::EpubChapter;
//!
//! Epub::open("old.epub")?
//!     .edit()
//!     // Appending a creator
//!     .author("Jane Doe")
//!     // Appending a chapter
//!     .chapter(EpubChapter::new("Chapter 1337").xhtml_body("1337"))
//!     // Setting the modified date to now
//!     .modified_now()
//!     .write()
//!     .compression(9)
//!     .save("new.epub")
//! # }
//! # }
//! ```
//! ### Creating a backwards-compatible EPUB 3 file
//!
//! > This example uses the [high-level builder API](epub::EpubEditor).
//! > See the [`epub`] module for lower-level control over the manifest, spine, etc.
//!
//! ```no_run
//! # #[cfg(feature = "write")]
//! # {
//! # use rbook::Epub;
//! # use rbook::ebook::errors::EbookResult;
//! use rbook::ebook::toc::TocEntryKind;
//! use rbook::epub::EpubChapter;
//! use std::path::Path;
//!
//! const XHTML: &[u8] = b"<xhtml>...</xhtml>"; // Example data
//!
//! # fn main() -> EbookResult<()> {
//! Epub::builder()
//!     .identifier("urn:example")
//!     .title("Doe Story")
//!     .author(["John Doe", "Jane Doe"])
//!     .language("en")
//!     // Reference a file stored on disk or provide in-memory bytes
//!     .cover_image(("cover.png", Path::new("local/file/cover.png")))
//!     .chapter([
//!         // Standard Chapter (Auto-generates href/filename "volume_i.xhtml")
//!         EpubChapter::new("Volume I").xhtml(XHTML).children(
//!             // Providing an explicit href (v1c1.xhtml)
//!             EpubChapter::new("I: Intro")
//!                 .kind(TocEntryKind::Introduction)
//!                 .href("v1c1.xhtml")
//!                 .xhtml_body("<p>Basic text</p>"),
//!         ),
//!         EpubChapter::new("Volume II").children([
//!             // Referencing an XHTML file stored on the OS file system
//!             EpubChapter::new("I").href("v2/c1.xhtml").xhtml(Path::new("path/to/c1.xhtml")),
//!             // Navigation-only entry linking to a fragment in another chapter
//!             EpubChapter::new("Section 1").href("v2/c1.xhtml#section-1"),
//!             // Resource included in spine/manifest but omitted from ToC
//!             EpubChapter::unlisted("v3extras.xhtml").xhtml(XHTML),
//!         ]),
//!     ])
//!     .write()
//!     .compression(0)
//!     // Save to disk or alternatively write to memory
//!     .save("doe_story.epub")
//! # }
//! # }
//! ```

#![warn(missing_docs)]
#![cfg_attr(docsrs, feature(doc_cfg))]

////////////////////////////////////////////////////////////////////////////////
// PRIVATE API
////////////////////////////////////////////////////////////////////////////////

mod parser;
mod util;
#[cfg(feature = "write")]
mod writer;

////////////////////////////////////////////////////////////////////////////////
// PUBLIC API
////////////////////////////////////////////////////////////////////////////////

pub mod ebook;
pub mod input;
pub mod reader;

#[doc(inline)]
pub use crate::ebook::epub;
pub use {ebook::Ebook, epub::Epub};

/// The rbook prelude for convenient imports of the core
/// [`ebook`] and [`reader`] **traits**.
///
/// This is a crate feature, `prelude`, that is enabled by default.
///
/// The prelude circumvents manually importing each trait and helps keep imports lean:
/// ```no_run
/// // Without the prelude (Verbose; manually importing each trait):
/// /*1*/ use rbook::Ebook;
/// /*2*/ use rbook::ebook::manifest::ManifestEntry;
/// /*3*/ use rbook::ebook::spine::{Spine, SpineEntry};
///
/// // With the prelude, lines 1, 2, and 3 can be consolidated into:
/// use rbook::prelude::*;
///
/// # fn main() -> rbook::ebook::errors::EbookResult<()> {
/// // Retrieve the manifest entry associated with a spine entry:
/// fn analyze<E: Ebook>(ebook: E) {
///     let spine_entry = ebook.spine().get(2).unwrap();
///     let manifest_entry = spine_entry.manifest_entry().unwrap();
///     let resource = manifest_entry.resource();
///
///     assert_eq!(2, spine_entry.order());
///     assert_eq!("xhtml", resource.kind().subtype());
///     assert_eq!("/EPUB/c1.xhtml", resource.key().value().unwrap());
/// }
/// # analyze(rbook::Epub::open("tests/ebooks/example_epub")?);
/// # Ok(())
/// # }
/// ```
/// The idea of libraries providing a prelude is subjective and may not be desirable.
/// As such, it is set as a default crate feature that can be disabled inside a
/// project's `Cargo.toml` file.
/// For example, omitting the `prelude` while retaining the `threadsafe` and `write` feature:
/// ```toml
/// [dependencies]
/// rbook = { version = "0.7.2", default-features = false, features = ["threadsafe", "write"] }
/// ```
#[cfg(feature = "prelude")]
pub mod prelude {
    pub use crate::ebook::{
        Ebook,
        manifest::{Manifest, ManifestEntry},
        metadata::{Contributor, Identifier, Language, MetaEntry, Metadata, Tag, Title},
        spine::{Spine, SpineEntry},
        toc::{Toc, TocEntry},
    };
    pub use crate::reader::{Reader, ReaderContent};
}
