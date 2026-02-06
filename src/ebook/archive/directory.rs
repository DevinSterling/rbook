use crate::ebook::archive::errors::ArchiveResult;
use crate::ebook::archive::{self, Archive, ArchiveError};
use crate::ebook::resource::Resource;
use std::fs::File;
use std::io::{self, Write};
use std::path::{Path, PathBuf};

#[cfg(feature = "write")]
use {super::ResourceKeySet, crate::ebook::resource::ResourceKey, crate::util, std::borrow::Cow};

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
        let path = self.0.join(archive::extract_resource_path(resource)?);
        let resolved = path
            .canonicalize()
            .map_err(|source| ArchiveError::InvalidResource {
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
    fn copy_resource(&self, resource: &Resource, writer: &mut dyn Write) -> ArchiveResult<u64> {
        let path = self.get_path(resource)?;
        let file = File::open(&path);

        match file {
            Ok(mut file) => io::copy(&mut file, writer),
            Err(error) => Err(error),
        }
        .map_err(|error| ArchiveError::CannotRead {
            source: error,
            resource: resource.as_static(),
        })
    }

    #[cfg(feature = "write")]
    fn resources(&self) -> ArchiveResult<ResourceKeySet<'_>> {
        fn traverse(set: &mut ResourceKeySet<'_>, prefix: &Path, path: &Path) -> ArchiveResult<()> {
            fn unreadable(source: io::Error, path: PathBuf) -> ArchiveError {
                ArchiveError::UnreadableArchive {
                    path: Some(path),
                    source,
                }
            }

            let read_dir = path
                .read_dir()
                .map_err(|err| unreadable(err, path.to_path_buf()))?;

            for entry in read_dir {
                let entry = entry.map_err(|err| unreadable(err, path.to_path_buf()))?;
                let metadata = entry
                    .metadata()
                    .map_err(|err| unreadable(err, entry.path()))?;

                // Symlinks are not supported currently
                // (to avoid potential path traversal vulnerabilities)
                if metadata.is_symlink() {
                    continue;
                }

                let path = entry.path();
                if metadata.is_dir() {
                    traverse(set, prefix, &path)?;
                } else if let Ok(path) = path.strip_prefix(prefix)
                    // Only UTF8 paths are supported.
                    && let Some(utf8_path) = path.to_str()
                {
                    // The path is made absolute to maintain consistency throughout the API
                    let value = if cfg!(windows) {
                        // Enforce forward slashes
                        util::str::prefix("/", &utf8_path.replace("\\", "/"))
                    } else {
                        util::str::prefix("/", utf8_path)
                    };

                    let key = ResourceKey::Value(Cow::Owned(value));

                    set.insert(Cow::Owned(key));
                }
            }
            Ok(())
        }

        let mut set = ResourceKeySet::new();
        traverse(&mut set, &self.0, &self.0)?;
        Ok(set)
    }
}
