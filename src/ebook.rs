//! Core format-agnostic [`Ebook`] module and implementations.
//!
//! # Overview
//! This module defines [`Ebook`] (the common interface for ebook formats),
//! associated traits, and supporting modules.
//!
//! ## Supported Formats
//! - [`epub`]: EPUB 2 and 3
//!
//! ## Core Components
//! - [`errors`]: Ebook-related error types.
//! - [`manifest`]: Resources contained within an ebook.
//! - [`metadata`]: Metadata details (title, authors, identifiers, version).
//! - [`spine`]: The canonical reading-order sequence.
//! - [`toc`]: Table of contents navigation.
//!
//! ## Supporting Components
//! - [`resource`]: Resource identification and handles.
//! - [`element`]: Access to XML-related types.

pub(super) mod archive;
pub mod element;
pub mod epub;
pub mod errors;
pub mod manifest;
pub mod metadata;
pub mod resource;
pub mod spine;
pub mod toc;

use crate::ebook::errors::EbookResult;
use crate::ebook::manifest::Manifest;
use crate::ebook::metadata::Metadata;
use crate::ebook::resource::Resource;
use crate::ebook::spine::Spine;
use crate::ebook::toc::Toc;
use crate::reader::Reader;

/// Trait that represents the core properties of an ebook.
///
/// Provides access to the following main contents:
/// - [`Metadata`]: Metadata details (title, language, identifiers)
/// - [`Manifest`]: Manifest resources (HTML, images, CSS)
/// - [`Spine`]: Canonical reading order
/// - [`Toc`]: Table of contents
///
/// # Supported ebook formats:
/// - [EPUB 2 and 3](epub::Epub)
///
/// # Lifetime
/// All views, such as [`Reader`], [`Manifest`], [`Metadata`], [`Spine`], [`Toc`], etc. are
/// tied to the lifetime of the owned [`Ebook`] instance (`'ebook`).
pub trait Ebook {
    /// Returns a new [`Reader`] to sequentially read over the [`Spine`] contents of an ebook.
    fn reader(&self) -> impl Reader<'_>;

    /// Attributes associated with an ebook, such as title and author information.
    fn metadata(&self) -> impl Metadata<'_>;

    /// The [`Manifest`], encompassing the [`resources`](Resource) contained within an ebook.
    fn manifest(&self) -> impl Manifest<'_>;

    /// The [`Spine`], encompassing the canonical reading-order sequence.
    ///
    /// # See Also
    /// - [`Self::reader`] to sequentially read spine content with greater control.
    fn spine(&self) -> impl Spine<'_>;

    /// The table of contents ([`Toc`]), encompassing navigation points.
    fn toc(&self) -> impl Toc<'_>;

    /// Returns the specified [`Resource`] in the form of a string.
    ///
    /// # Errors
    /// [`ArchiveError`](errors::ArchiveError): When retrieval of the specified [`Resource`] fails.
    ///
    /// # See Also
    /// - [`ManifestEntry::read_str`](manifest::ManifestEntry::read_str)
    ///   to alternatively retrieve the data from a manifest entry.
    fn read_resource_str<'a>(&self, resource: impl Into<Resource<'a>>) -> EbookResult<String>;

    /// Returns the specified [`Resource`] in the form of bytes.
    ///
    /// # Errors
    /// [`ArchiveError`](errors::ArchiveError): When retrieval of the specified [`Resource`] fails.
    ///
    /// # See Also
    /// - [`ManifestEntry::read_bytes`](manifest::ManifestEntry::read_bytes)
    ///   to alternatively retrieve the data from a manifest entry.
    fn read_resource_bytes<'a>(&self, resource: impl Into<Resource<'a>>) -> EbookResult<Vec<u8>>;
}
