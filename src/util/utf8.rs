use std::borrow::Cow;
use thiserror::Error;

#[derive(Error, Debug)]
pub(crate) enum UtfError {
    #[error("UTF-16 data needs to contain an even amount of bytes")]
    UnevenByteCount,
}

// Support for UTF-16 by converting it to UTF-8
pub(crate) fn to_utf8(data: &[u8]) -> Result<Cow<[u8]>, UtfError> {
    Ok(match get_utf16_data(data)? {
        Some(converted) => Cow::Owned(converted.into_bytes()),
        _ => Cow::Borrowed(data),
    })
}

pub(crate) fn to_utf8_str(data: &[u8]) -> Result<Cow<str>, UtfError> {
    Ok(match get_utf16_data(data)? {
        Some(converted) => Cow::Owned(converted),
        _ => String::from_utf8_lossy(data),
    })
}

fn get_utf16_data(data: &[u8]) -> Result<Option<String>, UtfError> {
    // Check if an utf-16 byte order mark (bom) exists
    if data.starts_with(b"\xFF\xFE") || data.starts_with(b"\xFE\xFF") {
        // Determine byte order for little endian (le) and big endian (be)
        let endian = if data.starts_with(b"\xFF") {
            u16::from_le_bytes
        } else {
            u16::from_be_bytes
        };

        data[2..]
            .chunks_exact(2)
            .map(|chunk| chunk.try_into().map(endian))
            .collect::<Result<Vec<_>, _>>()
            .map(|utf16| Some(String::from_utf16_lossy(&utf16)))
            .map_err(|_| UtfError::UnevenByteCount)
    } else {
        Ok(None)
    }
}
