use std::borrow::Cow;
use std::fs::{File, Metadata};
use std::path::{Path, PathBuf};

use crate::formats::EbookError;

// Splits a string into two separate strings and excludes
// the split character
pub(crate) fn split_where(string: &str, character: char) -> Option<(&str, &str)> {
    string.find(character)
        .map(|index| string.split_at(index))
        .map(|(left,right)| (left, &right[1..]))
}

pub(crate) fn get_file<P: AsRef<Path>>(path: P) -> Result<File, EbookError> {
    File::open(&path).map_err(|error| EbookError::IO {
        cause: "Unable to open file".to_string(),
        description: format!("File path: '{:?}': {error}", path.as_ref())
    })
}

pub(crate) fn get_path_metadata<P: AsRef<Path>>(path: P) -> Result<Metadata, EbookError> {
    path.as_ref().metadata()
        .map_err(|error| EbookError::IO {
        cause: "Unable to access path metadata".to_string(),
        description: format!("Path: '{:?}': {error}", path.as_ref())
    })
}

pub(crate) fn get_parent_path<P: AsRef<Path>>(path: P) -> PathBuf {
    // Return an empty path if there is no parent
    path.as_ref().parent()
        .map_or_else(|| PathBuf::from(""), |parent| parent.to_path_buf())
}

// Support for UTF-16 by converting it to UTF-8
pub(crate) fn to_utf8(data: &[u8]) -> Cow<[u8]> {
    // Check if a utf-16 byte order mark (bom) exists
    if data.starts_with(b"\xFF\xFE") || data.starts_with(b"\xFE\xFF") {
        // Determine byte order for little endian (le) and big endian (be)
        let endian = if data.starts_with(b"\xFF") {
            u16::from_le_bytes
        } else {
            u16::from_be_bytes
        };

        let utf16_data: Vec<_> = data[2..].chunks_exact(2)
            .map(|chunk| endian([chunk[0], chunk[1]]))
            .collect();
        let utf8_data = String::from_utf16_lossy(&utf16_data);

        Cow::Owned(utf8_data.into_bytes())
    } else {
        Cow::Borrowed(data)
    }
}