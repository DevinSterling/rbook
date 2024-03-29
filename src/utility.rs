use std::borrow::Cow;
use std::fs::{File, Metadata};
use std::path::{Path, PathBuf};

use crate::formats::EbookError;
use crate::result::EbookResult;

#[cfg(feature = "multi-thread")]
pub use std::sync::{Arc as Shared, Mutex as Lock, Weak};
#[cfg(not(feature = "multi-thread"))]
pub use std::{
    cell::RefCell as Lock,
    rc::{Rc as Shared, Weak},
};

// Splits a string into two separate strings and excludes
// the split character
pub(crate) fn split_where(string: &str, character: char) -> Option<(&str, &str)> {
    string
        .find(character)
        .map(|index| string.split_at(index))
        .map(|(left, right)| (left, &right[1..]))
}

pub(crate) fn get_file<P: AsRef<Path>>(path: P) -> EbookResult<File> {
    File::open(&path).map_err(|error| EbookError::IO {
        cause: "Unable to open file".to_string(),
        description: format!("File path: '{:?}': {error}", path.as_ref()),
    })
}

pub(crate) fn get_path_metadata<P: AsRef<Path>>(path: P) -> EbookResult<Metadata> {
    path.as_ref().metadata().map_err(|error| EbookError::IO {
        cause: "Unable to access path metadata".to_string(),
        description: format!("Path: '{:?}': {error}", path.as_ref()),
    })
}

pub(crate) fn get_parent_path<P: AsRef<Path>>(path: &P) -> &Path {
    let path = path.as_ref();
    // Return `path` itself if there is no parent
    path.parent().unwrap_or(path)
}

// Function to normalize paths. ex: `EPUB//.//OPS/../../toc.ncx` -> `toc.ncx`
pub(crate) fn normalize_path<P: AsRef<Path>>(path: &P) -> Cow<Path> {
    let mut stack = Vec::new();
    let mut is_normalized = true;

    for component in path.as_ref().components() {
        let slice = component.as_os_str();

        if slice == ".." {
            is_normalized = false;
            stack.pop();
        } else {
            stack.push(slice);
        }
    }

    if is_normalized {
        Cow::Borrowed(path.as_ref())
    } else {
        let mut normalized_path = PathBuf::new();

        for component in stack {
            normalized_path.push(component);
        }

        Cow::Owned(normalized_path)
    }
}

// Support for UTF-16 by converting it to UTF-8
pub(crate) fn to_utf8(data: &[u8]) -> Cow<[u8]> {
    // Check if a utf-16 byte order mark (bom) exists
    if data.starts_with(b"\xFF\xFE") || data.starts_with(b"\xFE\xFF") {
        // Determine byte order for little endian (le) and big endian (be)
        let endian = match data.starts_with(b"\xFF") {
            true => u16::from_le_bytes,
            false => u16::from_be_bytes,
        };

        let utf16_data: Vec<_> = data[2..]
            .chunks_exact(2)
            .map(|chunk| endian([chunk[0], chunk[1]]))
            .collect();
        let utf8_data = String::from_utf16_lossy(&utf16_data);

        Cow::Owned(utf8_data.into_bytes())
    } else {
        Cow::Borrowed(data)
    }
}

pub(crate) fn split_exclude<'a>(input: &'a str, excluded: &[char]) -> Vec<&'a str> {
    input
        .split(|c| excluded.iter().any(|exclude| c == *exclude))
        .filter(|s| excluded.iter().any(|exclude| s != &exclude.to_string()))
        .collect()
}
