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
use std::io::{self, Read, Write};

#[cfg(feature = "write")]
use {
    crate::ebook::resource::ResourceContent,
    std::borrow::Cow,
    write::{ArchiveOverlay, OverlayResource, ResourceKeySet},
};

#[derive(Copy, Clone, Debug)]
pub(crate) struct ArchiveConfig {
    /// Maximum number of bytes allowed per resource.
    /// Exceeding the limit results in an error.
    pub(crate) max_resource_bytes: u64,
}

impl Default for ArchiveConfig {
    fn default() -> Self {
        Self {
            max_resource_bytes: u64::MAX,
        }
    }
}

pub(crate) trait Archive: util::sync::SendAndSync {
    fn contains_resource(&self, resource: &Resource) -> ArchiveResult<bool>;

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
    fn contains_resource(&self, resource: &Resource) -> ArchiveResult<bool> {
        (**self).contains_resource(resource)
    }

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
    fn contains_resource(&self, resource: &Resource) -> ArchiveResult<bool> {
        // `overlay` takes precedence even if the resource exists in `base`
        #[cfg(feature = "write")]
        if self.overlay.contains_key(resource.key()) {
            return Ok(true);
        }
        self.base.contains_resource(resource)
    }

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
pub(crate) fn get_archive(
    config: ArchiveConfig,
    path: &std::path::Path,
) -> ArchiveResult<Box<dyn Archive>> {
    Ok(if path.is_file() {
        let file = fs::File::open(path).map_err(|error| ArchiveError::UnreadableArchive {
            source: error,
            path: Some(path.to_path_buf()),
        })?;
        Box::new(ZipArchive::new(
            config,
            io::BufReader::new(file),
            Some(path),
        )?)
    } else {
        Box::new(DirectoryArchive::new(config, path)?)
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

/// Helper to copy a resource with a limit.
fn copy_resource_with_limit(
    limit: u64,
    resource: &Resource<'_>,
    reader: &mut impl Read,
    writer: &mut dyn Write,
) -> ArchiveResult<u64> {
    let map_err = |error| io_to_cannot_read_error(error, resource);

    if limit == u64::MAX {
        io::copy(reader, writer).map_err(map_err)
    } else {
        let copied = io::copy(&mut reader.take(limit), writer).map_err(map_err)?;

        if copied == limit
            // Check an extra byte to see if the limit has been exceeded
            && io::copy(&mut reader.take(1), &mut io::sink()).map_err(map_err)? > 0
        {
            Err(ArchiveError::MaxBytesExceeded {
                resource: resource.as_static(),
                limit,
            })
        } else {
            Ok(copied)
        }
    }
}

fn io_to_cannot_read_error(source: io::Error, resource: &Resource<'_>) -> ArchiveError {
    ArchiveError::CannotRead {
        resource: resource.as_static(),
        source,
    }
}

pub(crate) fn into_utf8_string(resource: &Resource, bytes: Vec<u8>) -> ArchiveResult<String> {
    util::utf::into_utf8_str(bytes).map_err(|source| ArchiveError::InvalidUtf8Resource {
        resource: resource.as_static(),
        source,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Cursor;

    #[test]
    fn copy_resource_limit_allows() {
        let resource = Resource::from("test");
        let large = vec![b'a'; 10000];
        #[rustfmt::skip]
        let cases: &[(&[u8], u64)] = &[
            (b"", 0),
            (b"Hello", 5),
            (b"Hello", 6),
            (b"Hello", u64::MAX),
            // Exceeds the internal 8 KiB buffer of `std::io::copy` (Multiple reads)
            (&large, 10000),
            (&large, 10001),
        ];

        for (buffer, max_limit) in cases {
            let mut reader = Cursor::new(buffer);
            let mut writer = Vec::new();
            let copied =
                copy_resource_with_limit(*max_limit, &resource, &mut reader, &mut writer).unwrap();

            assert_eq!(buffer.len() as u64, copied);
            assert_eq!(*buffer, writer);
        }
    }

    #[test]
    fn copy_resource_limit_rejects() {
        let expected_resource = Resource::from("test");
        let large = vec![b'a'; 10_000];
        #[rustfmt::skip]
        let cases: &[(&[u8], u64)] = &[
            (b"Hello", 0),
            (b"Hello", 1),
            (b"Hello", 4),
            // Exceeds the internal 8 KiB buffer of `std::io::copy` (Multiple reads)
            (&large, 8192),
            (&large, 9999),
        ];

        for (buffer, max_limit) in cases {
            let mut reader = Cursor::new(buffer);
            let mut writer = Vec::new();

            let Err(ArchiveError::MaxBytesExceeded { limit, resource }) =
                copy_resource_with_limit(*max_limit, &expected_resource, &mut reader, &mut writer)
            else {
                panic!("expected MaxBytesExceeded");
            };

            assert_eq!(*max_limit, limit);
            assert_eq!(expected_resource, resource);

            // Writer must never go beyond the max limit
            assert!(writer.len() <= *max_limit as usize);
        }
    }
}
