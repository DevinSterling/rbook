use crate::ebook::archive::errors::ArchiveResult;
use crate::ebook::archive::{self, Archive, ArchiveConfig, ArchiveError, copy_resource_with_limit};
use crate::ebook::resource::Resource;
use crate::util::sync::Lock;
use std::io::{self, Read, Seek, Write};
use std::path::Path;
use zip::ZipArchive as Zip;

#[cfg(feature = "write")]
use {super::ResourceKeySet, crate::ebook::resource::ResourceKey, crate::util, std::borrow::Cow};

pub(crate) struct ZipArchive<R> {
    config: ArchiveConfig,
    zip: Lock<Zip<R>>,
}

impl<R: Read + Seek> ZipArchive<R> {
    /// `reader` (and optional `path` for a more descriptive error message).
    pub(crate) fn new(
        config: ArchiveConfig,
        reader: R,
        path: Option<&Path>,
    ) -> ArchiveResult<Self> {
        Zip::new(reader)
            .map(|zip| Self {
                zip: Lock::new(zip),
                config,
            })
            .map_err(|error| ArchiveError::UnreadableArchive {
                source: io::Error::from(error),
                path: path.map(Path::to_path_buf),
            })
    }
}

impl<#[cfg(feature = "threadsafe")] R: Send + Sync, #[cfg(not(feature = "threadsafe"))] R> Archive
    for ZipArchive<R>
where
    R: Read + Seek + 'static,
{
    fn contains_resource(&self, resource: &Resource) -> ArchiveResult<bool> {
        let path = archive::extract_resource_path(resource)?;
        let lock = self.zip.lock().map_err(|_| create_poisoned_error())?;

        Ok(lock.index_for_name(path).is_some())
    }

    fn copy_resource(&self, resource: &Resource, writer: &mut dyn Write) -> ArchiveResult<u64> {
        let mut lock = self.zip.lock().map_err(|_| create_poisoned_error())?;
        let mut zip_file = lock
            .by_name(archive::extract_resource_path(resource)?)
            .map_err(|error| ArchiveError::InvalidResource {
                source: io::Error::from(error),
                resource: resource.as_static(),
            })?;

        copy_resource_with_limit(
            self.config.max_resource_bytes,
            resource,
            &mut zip_file,
            writer,
        )
    }

    #[cfg(feature = "write")]
    fn resources(&self) -> ArchiveResult<ResourceKeySet<'_>> {
        let lock = self.zip.lock().map_err(|_| create_poisoned_error())?;

        Ok(lock
            .file_names()
            // An error can be thrown here...
            // However, that means resource retrieval becomes impossible if a single
            // entry has a malformed/non-UTF8 name.
            // (NOTE: EPUBs should only contain UTF-8 filenames)
            .filter_map(|result| result.ok())
            // Ignore directories; paths that ends with a separator
            .filter(|path| !path.ends_with('/'))
            // Owning is necessary due to the lock
            // - The path is made absolute to maintain consistency throughout the API
            .map(|path| Cow::Owned(ResourceKey::from(util::str::prefix("/", &path))))
            .collect())
    }
}

fn create_poisoned_error() -> ArchiveError {
    ArchiveError::UnreadableArchive {
        source: io::Error::other("Poisoned ZipArchive"),
        path: None,
    }
}
