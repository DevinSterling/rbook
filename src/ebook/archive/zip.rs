use crate::ebook::archive::errors::ArchiveResult;
use crate::ebook::archive::{self, Archive, ArchiveError};
use crate::ebook::resource::Resource;
use crate::util::sync::Lock;
use std::io;
use std::io::{Read, Seek};
use std::path::Path;
use zip::ZipArchive as Zip;
use zip::read::ZipFile;

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
            .by_name(archive::extract_resource_key(resource)?)
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
    fn read_resource_bytes(&self, resource: &Resource) -> Result<Vec<u8>, ArchiveError> {
        let mut lock = acquire_archive_lock(&self.0)?;
        let mut zip_file = Self::get_file(&mut lock, resource)?;
        let mut buf = Vec::new();

        zip_file
            .read_to_end(&mut buf)
            .map(|_| buf)
            .map_err(|error| ArchiveError::CannotRead {
                source: error,
                resource: resource.as_static(),
            })
    }
}

#[cfg(feature = "threadsafe")]
fn acquire_archive_lock<T>(lock: &Lock<T>) -> ArchiveResult<std::sync::MutexGuard<T>> {
    lock.lock().map_err(|_| ArchiveError::UnreadableArchive {
        source: io::Error::other("Poisoned ZipArchive"),
        path: None,
    })
}
#[cfg(not(feature = "threadsafe"))]
fn acquire_archive_lock<T>(lock: &Lock<T>) -> Result<std::cell::RefMut<T>, ArchiveError> {
    Ok(lock.borrow_mut())
}
