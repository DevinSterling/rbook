use crate::ebook::archive::errors::ArchiveResult;
use crate::ebook::archive::{self, Archive, ArchiveError};
use crate::ebook::resource::Resource;
use std::path::{Path, PathBuf};
use std::{fs, io};

#[derive(Debug)]
pub(crate) struct DirectoryArchive(PathBuf);

impl DirectoryArchive {
    pub(crate) fn new(file: &Path) -> ArchiveResult<Self> {
        match file.canonicalize() {
            Ok(dir) if dir.is_dir() => Ok(Self(dir)),
            Ok(_) => Err(ArchiveError::UnreadableArchive {
                path: Some(file.to_path_buf()),
                source: io::Error::from(io::ErrorKind::NotADirectory),
            }),
            Err(source) => Err(ArchiveError::UnreadableArchive {
                path: Some(file.to_path_buf()),
                source,
            }),
        }
    }

    fn get_path(&self, resource: &Resource) -> Result<PathBuf, ArchiveError> {
        let path = self.0.join(archive::extract_resource_key(resource)?);
        let resolved = path
            .canonicalize()
            .map_err(|source| ArchiveError::CannotRead {
                resource: resource.as_static(),
                source,
            })?;

        // Path traversal mitigation
        if resolved.starts_with(&self.0) && resolved.is_file() {
            Ok(resolved)
        } else {
            Err(ArchiveError::InvalidResource {
                source: io::Error::new(
                    io::ErrorKind::NotFound,
                    "Provided path is inaccessible or not a file",
                ),
                resource: resource.as_static(),
            })
        }
    }
}

impl Archive for DirectoryArchive {
    fn read_resource_bytes(&self, resource: &Resource) -> Result<Vec<u8>, ArchiveError> {
        let path = self.get_path(resource)?;

        fs::read(&path).map_err(|error| ArchiveError::CannotRead {
            source: error,
            resource: resource.as_static(),
        })
    }
}
