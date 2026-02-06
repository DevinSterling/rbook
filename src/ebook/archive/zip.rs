use crate::ebook::archive::errors::ArchiveResult;
use crate::ebook::archive::{self, Archive, ArchiveError};
use crate::ebook::resource::Resource;
use crate::util::sync::Lock;
use std::io::{self, Read, Seek, Write};
use std::path::Path;
use zip::ZipArchive as Zip;
use zip::read::ZipFile;

#[cfg(feature = "write")]
use {super::ResourceKeySet, crate::ebook::resource::ResourceKey, crate::util, std::borrow::Cow};

pub(crate) struct ZipArchive<R>(Lock<Zip<R>>);

impl<R: Read + Seek> ZipArchive<R> {
    /// `reader` (and optional `path` for a more descriptive error message).
    pub(crate) fn new(reader: R, path: Option<&Path>) -> ArchiveResult<Self> {
        Zip::new(reader)
            .map(|zip| Self(Lock::new(zip)))
            .map_err(|error| ArchiveError::UnreadableArchive {
                source: io::Error::from(error),
                path: path.map(Path::to_path_buf),
            })
    }

    fn get_file<'a>(archive: &'a mut Zip<R>, resource: &Resource) -> ArchiveResult<ZipFile<'a, R>> {
        archive
            .by_name(archive::extract_resource_path(resource)?)
            .map_err(|error| ArchiveError::InvalidResource {
                source: io::Error::from(error),
                resource: resource.as_static(),
            })
    }
}

impl<#[cfg(feature = "threadsafe")] R: Send + Sync, #[cfg(not(feature = "threadsafe"))] R> Archive
    for ZipArchive<R>
where
    R: Read + Seek + 'static,
{
    fn copy_resource(&self, resource: &Resource, writer: &mut dyn Write) -> ArchiveResult<u64> {
        let mut lock = self.0.lock().map_err(|_| ArchiveError::UnreadableArchive {
            source: io::Error::other("Poisoned ZipArchive"),
            path: None,
        })?;
        let mut zip_file = Self::get_file(&mut lock, resource)?;

        std::io::copy(&mut zip_file, writer).map_err(|error| ArchiveError::CannotRead {
            source: error,
            resource: resource.as_static(),
        })
    }

    #[cfg(feature = "write")]
    fn resources(&self) -> ArchiveResult<ResourceKeySet<'_>> {
        let lock = self.0.lock().map_err(|_| ArchiveError::UnreadableArchive {
            source: io::Error::other("Poisoned ZipArchive"),
            path: None,
        })?;

        Ok(lock
            .file_names()
            // Ignore directories; paths that ends with a separator
            .filter(|path| !path.ends_with('/'))
            // Owning is necessary due to the lock
            // - The path is made absolute to maintain consistency throughout the API
            .map(|path| Cow::Owned(ResourceKey::from(util::str::prefix("/", path))))
            .collect())
    }
}
