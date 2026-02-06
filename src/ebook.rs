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

use crate::ebook::archive::errors::ArchiveResult;
use crate::ebook::errors::EbookResult;
use crate::ebook::manifest::Manifest;
use crate::ebook::metadata::Metadata;
use crate::ebook::resource::Resource;
use crate::ebook::spine::Spine;
use crate::ebook::toc::Toc;
use crate::reader::Reader;
use crate::util::Sealed;
use std::io::Write;

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
pub trait Ebook: Sealed {
    /// Returns a new [`Reader`] to sequentially read over the [`Spine`] contents of an ebook.
    fn reader(&self) -> impl Reader<'_>;

    /// Data associated with an ebook, such as
    /// [title](Metadata::title) and [author](Metadata::creators) information.
    fn metadata(&self) -> impl Metadata<'_>;

    /// The [`Manifest`], encompassing the publication [`resources`](Resource)
    /// contained within an ebook.
    fn manifest(&self) -> impl Manifest<'_>;

    /// The [`Spine`], encompassing the canonical reading-order sequence.
    ///
    /// # See Also
    /// - [`Self::reader`] to sequentially read spine content with greater control.
    fn spine(&self) -> impl Spine<'_>;

    /// The table of contents ([`Toc`]), encompassing navigation points.
    fn toc(&self) -> impl Toc<'_>;

    /// Copies the content of a [`Resource`] into the given `writer`,
    /// returning the total number of bytes written on success.
    ///
    /// # Errors
    /// [`ArchiveError`](errors::ArchiveError): When copying the content of a [`Resource`] fails.
    ///
    /// # See Also
    /// - [`ManifestEntry::copy_bytes`](manifest::ManifestEntry::copy_bytes)
    ///   to copy the content directly from a manifest entry.
    fn copy_resource<'a>(
        &self,
        resource: impl Into<Resource<'a>>,
        writer: &mut impl Write,
    ) -> ArchiveResult<u64>;

    /// Returns the content of a [`Resource`] as a string.
    ///
    /// # Errors
    /// [`ArchiveError`](errors::ArchiveError): When retrieval of the specified [`Resource`] fails.
    ///
    /// # See Also
    /// - [`ManifestEntry::read_str`](manifest::ManifestEntry::read_str)
    ///   to retrieve the content directly from a manifest entry.
    fn read_resource_str<'a>(&self, resource: impl Into<Resource<'a>>) -> ArchiveResult<String> {
        let resource = resource.into();

        archive::into_utf8_string(&resource, self.read_resource_bytes(&resource)?)
    }

    /// Returns the content of a [`Resource`] as bytes.
    ///
    /// # Errors
    /// [`ArchiveError`](errors::ArchiveError): When retrieval of the specified [`Resource`] fails.
    ///
    /// # See Also
    /// - [`ManifestEntry::read_bytes`](manifest::ManifestEntry::read_bytes)
    ///   to retrieve the content directly from a manifest entry.
    fn read_resource_bytes<'a>(&self, resource: impl Into<Resource<'a>>) -> ArchiveResult<Vec<u8>> {
        let mut vec = Vec::new();
        self.copy_resource(resource, &mut vec)?;
        Ok(vec)
    }
}
