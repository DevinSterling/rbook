mod directory;
pub(crate) mod errors;
pub(crate) mod zip;

// Write-only modules
#[cfg(feature = "write")]
pub(crate) mod empty;
#[cfg(feature = "write")]
pub(crate) mod write;

use crate::ebook::archive::directory::DirectoryArchive;
use crate::ebook::archive::errors::{ArchiveError, ArchiveResult};
use crate::ebook::archive::zip::ZipArchive;
use crate::ebook::resource::{Resource, ResourceKey};
use crate::util;
use std::fmt::Debug;
use std::fs;
use std::io::{self, Write};

#[cfg(feature = "write")]
use {
    crate::ebook::resource::ResourceContent,
    std::borrow::Cow,
    write::{ArchiveOverlay, OverlayResource, ResourceKeySet},
};

pub(crate) trait Archive: util::sync::SendAndSync {
    fn copy_resource(&self, resource: &Resource, writer: &mut dyn Write) -> ArchiveResult<u64>;

    fn read_resource_as_utf8_bytes(&self, resource: &Resource) -> ArchiveResult<Vec<u8>> {
        let mut bytes = Vec::new();

        self.copy_resource(resource, &mut bytes)?;
        util::utf::into_utf8(bytes).map_err(|source| ArchiveError::InvalidUtf8Resource {
            resource: resource.as_static(),
            source,
        })
    }

    /// Returns all resource keys for data retrieval.
    /// Primarily only used when writing an archive to a destination.
    ///
    /// This ensures all resources are retrieved from an archive,
    /// which are then selectively copied.
    ///
    /// # Note
    /// In a future update, this method may not be feature-gated behind `write`.
    #[cfg(feature = "write")]
    fn resources(&self) -> ArchiveResult<ResourceKeySet<'_>>;
}

impl Archive for Box<dyn Archive> {
    fn copy_resource(&self, resource: &Resource, writer: &mut dyn Write) -> ArchiveResult<u64> {
        (**self).copy_resource(resource, writer)
    }

    #[cfg(feature = "write")]
    fn resources(&self) -> ArchiveResult<ResourceKeySet<'_>> {
        (**self).resources()
    }
}

/// A decorator over an [`Archive`] that enables write overlays.
pub(crate) struct ResourceArchive<A> {
    /// The original state of an ebook
    base: A,
    /// Files/content to add or overwrite from `base`
    #[cfg(feature = "write")]
    overlay: ArchiveOverlay,
}

impl<A: Archive> ResourceArchive<A> {
    pub(crate) fn new(base: A) -> Self {
        Self {
            base,
            #[cfg(feature = "write")]
            overlay: ArchiveOverlay::new(),
        }
    }
}

impl<A: Archive> Archive for ResourceArchive<A> {
    fn copy_resource(&self, resource: &Resource, writer: &mut dyn Write) -> ArchiveResult<u64> {
        // `overlay` takes precedence even if the resource exists in `base`
        #[cfg(feature = "write")]
        match self.overlay.get(resource.key()) {
            Some(OverlayResource::Content(content)) => {
                return content.copy_bytes(resource, writer);
            }
            Some(OverlayResource::Relocated(original_location)) => {
                return self.base.copy_resource(&original_location.into(), writer);
            }
            _ => {}
        }
        self.base.copy_resource(resource, writer)
    }

    #[cfg(feature = "write")]
    fn resources(&self) -> ArchiveResult<ResourceKeySet<'_>> {
        let mut resources = self.base.resources()?;

        for (key, content) in &self.overlay {
            if let OverlayResource::Relocated(original_location) = content {
                // Original `base` resource locations do not appear in the returned set
                resources.remove(original_location);
            }
            resources.insert(Cow::Borrowed(&key.0));
        }
        Ok(resources)
    }
}

impl<A> Debug for ResourceArchive<A> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let mut f = f.debug_struct("ResourceArchive");
        #[cfg(feature = "write")]
        f.field("overlay", &self.overlay);
        f.finish_non_exhaustive()
    }
}

#[derive(Copy, Clone)]
pub(crate) enum ResourceProvider<'ebook> {
    Archive(&'ebook dyn Archive),
    #[cfg(feature = "write")]
    Single(&'ebook ResourceContent),
    #[cfg(feature = "write")]
    Empty,
}

impl ResourceProvider<'_> {
    pub(crate) fn copy_bytes<W: Write>(
        &self,
        resource: &Resource<'_>,
        mut writer: W,
    ) -> ArchiveResult<u64> {
        match self {
            Self::Archive(archive) => archive.copy_resource(resource, &mut writer),
            #[cfg(feature = "write")]
            Self::Single(content) => content.copy_bytes(resource, &mut writer),
            #[cfg(feature = "write")]
            Self::Empty => Err(ArchiveError::InvalidResource {
                source: io::Error::new(
                    io::ErrorKind::Unsupported,
                    "Content retrieval is unsupported.",
                ),
                resource: resource.as_static(),
            }),
        }
    }
}

////////////////////////////////////////////////////////////////////////////////
// HELPER FUNCTIONS
////////////////////////////////////////////////////////////////////////////////

/// Unzip the file if it is not a directory.
///
/// If it is, the contents can be accessed directly,
/// which makes using a zip file unnecessary.
pub(crate) fn get_archive(path: &std::path::Path) -> ArchiveResult<Box<dyn Archive>> {
    Ok(if path.is_file() {
        let file = fs::File::open(path).map_err(|error| ArchiveError::UnreadableArchive {
            source: error,
            path: Some(path.to_path_buf()),
        })?;
        Box::new(ZipArchive::new(io::BufReader::new(file), Some(path))?)
    } else {
        Box::new(DirectoryArchive::new(path)?)
    })
}

/// Helper method for archives that support resolving against paths.
fn extract_resource_path<'a>(resource: &'a Resource<'a>) -> ArchiveResult<&'a str> {
    match resource.key() {
        // Make the file "relative" otherwise retrieving the file will not work.
        // ZipArchive and DirectoryArchive only support relative paths.
        //
        // `/path/to/chapter%202.xhtml` -> `path/to/chapter 2.xhtml`
        ResourceKey::Value(value) => Ok(value.trim_start_matches('/')),
        ResourceKey::Position(_) => Err(ArchiveError::InvalidResource {
            source: io::Error::from(io::ErrorKind::InvalidFilename),
            resource: resource.as_static(),
        }),
    }
}

pub(crate) fn into_utf8_string(resource: &Resource, bytes: Vec<u8>) -> ArchiveResult<String> {
    util::utf::into_utf8_str(bytes).map_err(|source| ArchiveError::InvalidUtf8Resource {
        resource: resource.as_static(),
        source,
    })
}
