use std::borrow::Cow;
use std::io::{Read, Seek};
use std::path::{Path, PathBuf};
use thiserror::Error;
use zip::{self, read};

use crate::formats::EbookError;
use crate::utility;

pub trait Archive {
    fn read_file(&mut self, path: &Path) -> Result<String, ArchiveError>;
    fn read_bytes_file(&mut self, path: &Path) -> Result<Vec<u8>, ArchiveError>;
}

/// Possible errors for an Archive:
/// - **[InvalidPath](Self::InvalidPath)**
/// - **[CannotRead](Self::CannotRead)**
/// - **[InvalidEncoding](Self::InvalidEncoding)**
#[derive(Error, Debug)]
pub enum ArchiveError {
    /// When a given path does not point to a valid location.
    #[error("[InvalidPath][{cause}]: {description}")]
    InvalidPath { cause: String, description: String },
    /// When the contents of a requested file cannot be read.
    #[error("[CannotRead][{cause}]: {description}")]
    CannotRead { cause: String, description: String },
    /// When a path contains characters from an unsupported encoding.
    #[error("[InvalidEncoding][{cause}]: {description}")]
    InvalidEncoding { cause: String, description: String },
}

// Wrapper struct
pub struct ZipArchive<T>(zip::ZipArchive<T>);

impl<T: Read + Seek> ZipArchive<T> {
    pub fn new(zip: T) -> Result<Self, EbookError> {
        match zip::ZipArchive::new(zip) {
            Ok(zip) => Ok(Self(zip)),
            Err(error) => Err(EbookError::IO {
                cause: "Unable to access zip archive".to_string(),
                description: error.to_string(),
            }),
        }
    }

    fn get_file<P: AsRef<Path>>(&mut self, path: P) -> Result<ZipFile, ArchiveError> {
        let normalized_path = utility::normalize_path(&path);

        let mut path_str = normalized_path
            .to_str()
            .ok_or_else(|| ArchiveError::InvalidEncoding {
                cause: "Non UTF-8 encoded path".to_string(),
                description: format!(
                    "The provided path does not contain valid utf-8: '{:?}'",
                    path.as_ref()
                ),
            })?
            .to_string();

        // Paths on windows contain backslashes. However, paths to files
        // in a zip archive requires only forward slashes.
        if cfg!(windows) {
            path_str = path_str.replace('\\', "/");
        }

        match self.0.by_name(&path_str) {
            Ok(zip_file) => Ok(ZipFile(zip_file)),
            Err(error) => Err(ArchiveError::InvalidPath {
                cause: "Unable to access zip file".to_string(),
                description: format!(
                    "Unable to retrieve file '{path_str}' from zip archive: {error}"
                ),
            }),
        }
    }
}

impl<T: Read + Seek> Archive for ZipArchive<T> {
    fn read_file(&mut self, path: &Path) -> Result<String, ArchiveError> {
        let mut zip_file = self.get_file(path)?;
        let content = zip_file.read()?;

        Ok(content)
    }

    fn read_bytes_file(&mut self, path: &Path) -> Result<Vec<u8>, ArchiveError> {
        let mut zip_file = self.get_file(path)?;
        let content = zip_file.read_bytes()?;

        Ok(content)
    }
}

// Wrapper struct
pub struct ZipFile<'a>(read::ZipFile<'a>);

impl ZipFile<'_> {
    pub fn read(&mut self) -> Result<String, ArchiveError> {
        let bytes = self.read_bytes()?;
        let data = utility::to_utf8(&bytes);

        let bytes = match data {
            Cow::Owned(_) => data.into_owned(), // Return new byte data
            _ => bytes,                         // Return original byte data
        };

        match String::from_utf8(bytes) {
            Ok(string) => Ok(string),
            Err(err) => Err(ArchiveError::CannotRead {
                cause: "Cannot read zip file contents to string".to_string(),
                description: err.to_string(),
            }),
        }
    }

    pub fn read_bytes(&mut self) -> Result<Vec<u8>, ArchiveError> {
        let mut buf = Vec::new();

        match &self.0.read_to_end(&mut buf) {
            Ok(_) => Ok(buf),
            Err(err) => Err(ArchiveError::CannotRead {
                cause: "Cannot read zip file contents to bytes vector".to_string(),
                description: err.to_string(),
            }),
        }
    }
}

pub struct DirArchive(PathBuf);

impl DirArchive {
    pub fn new<P: AsRef<Path>>(path: P) -> Result<Self, EbookError> {
        let path_buf = path.as_ref().to_path_buf();

        match path_buf.try_exists() {
            Ok(_) => Ok(Self(path_buf)),
            Err(error) => Err(EbookError::IO {
                cause: "Provided path is inaccessible".to_string(),
                description: format!("Path '{:?}': {error}", path.as_ref()),
            }),
        }
    }

    pub fn get_path<P: AsRef<Path>>(&self, path: P) -> Result<PathBuf, ArchiveError> {
        let join_path = self.0.join(&path);
        let normalized_path = utility::normalize_path(join_path);

        // Path traversal mitigation
        if normalized_path.starts_with(&self.0) && normalized_path.is_file() {
            Ok(normalized_path)
        } else {
            Err(ArchiveError::InvalidPath {
                cause: "Provided path is inaccessible or not a file".to_string(),
                description: format!("Please ensure the file exists: '{:?}'", path.as_ref()),
            })
        }
    }
}

impl Archive for DirArchive {
    fn read_file(&mut self, path: &Path) -> Result<String, ArchiveError> {
        let bytes = self.read_bytes_file(path)?;
        let data = utility::to_utf8(&bytes);

        let bytes = match data {
            Cow::Owned(_) => data.into_owned(), // Return new byte data
            _ => bytes,                         // Return original byte data
        };

        match String::from_utf8(bytes) {
            Ok(content) => Ok(content),
            Err(error) => Err(ArchiveError::CannotRead {
                cause: "Cannot read file contents to string".to_string(),
                description: format!("Path: '{path:?}': {error}"),
            }),
        }
    }

    fn read_bytes_file(&mut self, path: &Path) -> Result<Vec<u8>, ArchiveError> {
        let path = self.get_path(path)?;

        match std::fs::read(&path) {
            Ok(content) => Ok(content),
            Err(error) => Err(ArchiveError::CannotRead {
                cause: "Cannot read file contents to bytes vector".to_string(),
                description: format!("Path: '{path:?}': {error}"),
            }),
        }
    }
}
