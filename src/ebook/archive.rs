pub(super) mod directory;
pub(super) mod errors;
pub(super) mod zip;

use crate::ebook::archive::directory::DirectoryArchive;
use crate::ebook::archive::errors::{ArchiveError, ArchiveResult};
use crate::ebook::archive::zip::ZipArchive;
use crate::ebook::resource::{Resource, ResourceKey};
use crate::util;
use crate::util::sync::SendAndSync;
use std::fmt::{Debug, Formatter};
use std::fs::File;
use std::io::{self, BufReader};
use std::path::Path;

pub(super) trait Archive: SendAndSync {
    fn read_resource_bytes(&self, resource: &Resource) -> ArchiveResult<Vec<u8>>;

    fn read_resource_bytes_utf8(&self, resource: &Resource) -> ArchiveResult<Vec<u8>> {
        util::utf::into_utf8(self.read_resource_bytes(resource)?)
            .map_err(|_| ArchiveError::InvalidUtf8Resource(resource.as_static()))
    }

    fn read_resource_str(&self, resource: &Resource) -> ArchiveResult<String> {
        // Retrieve converted bytes
        util::utf::into_utf8_str(self.read_resource_bytes(resource)?)
            .map_err(|_| ArchiveError::InvalidUtf8Resource(resource.as_static()))
    }
}

/// This is a wrapper currently.
/// > When the write/modify API is released,
/// > additional fields will be added to this struct.
pub(super) struct ResourceArchive {
    /// The original state of an ebook
    base: Box<dyn Archive>,
}

impl ResourceArchive {
    pub(super) fn new(base: Box<dyn Archive>) -> Self {
        Self { base }
    }
}

impl Archive for ResourceArchive {
    fn read_resource_bytes(&self, resource: &Resource) -> ArchiveResult<Vec<u8>> {
        self.base.read_resource_bytes(resource)
    }
}

impl Debug for ResourceArchive {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("ResourceArchive")
            .field("base", &std::any::type_name_of_val(&self.base))
            .finish()
    }
}

/// Helper method for archives that support resolving against paths.
fn extract_resource_key<'a>(resource: &'a Resource<'a>) -> ArchiveResult<&'a str> {
    match resource.key() {
        ResourceKey::Value(value) => {
            // Make the file "relative" otherwise retrieving the file will not work.
            // ZipArchive and DirectoryArchive only support relative paths.
            //
            // `/EPUB/OEBPS/toc.xhtml` -> `EPUB/OEBPS/toc.xhtml`
            Ok(value.strip_prefix('/').unwrap_or(value))
        }
        ResourceKey::Position(_) => Err(ArchiveError::InvalidResource {
            source: io::Error::from(io::ErrorKind::InvalidFilename),
            resource: resource.as_static(),
        }),
    }
}

/// Unzip the file if it is not a directory.
///
/// If it is, the contents can be accessed directly,
/// which makes using a zip file unnecessary.
pub(super) fn get_archive(path: &Path) -> ArchiveResult<Box<dyn Archive>> {
    Ok(if path.is_file() {
        let file = File::open(path).map_err(|error| ArchiveError::UnreadableArchive {
            source: error,
            path: Some(path.to_path_buf()),
        })?;
        Box::new(ZipArchive::new(BufReader::new(file), Some(path))?)
    } else {
        Box::new(DirectoryArchive::new(path)?)
    })
}
