use crate::ebook::errors::UtfError;

// Support for UTF-16 by converting it to UTF-8
pub(crate) fn into_utf8(data: Vec<u8>) -> Result<Vec<u8>, UtfError> {
    if is_utf16(&data) {
        from_utf16(&data).map(String::into_bytes)
    } else {
        Ok(data)
    }
}

pub(crate) fn into_utf8_str(data: Vec<u8>) -> Result<String, UtfError> {
    if is_utf16(&data) {
        from_utf16(&data)
    } else {
        String::from_utf8(data).map_err(UtfError::InvalidUtf8)
    }
}

/// Checks if a UTF-16 byte order mark (BOM) exists
fn is_utf16(data: &[u8]) -> bool {
    data.starts_with(b"\xFF\xFE") || data.starts_with(b"\xFE\xFF")
}

fn from_utf16(data: &[u8]) -> Result<String, UtfError> {
    let (chunks, []) = data.as_chunks::<2>() else {
        return Err(UtfError::UnevenByteCount(data.len()));
    };
    let (endian, start): (fn([u8; 2]) -> u16, _) = match chunks.first() {
        // Determine byte order for little endian (le) and big endian (be)
        Some(b"\xFF\xFE") => (u16::from_le_bytes, 1),
        Some(b"\xFE\xFF") => (u16::from_be_bytes, 1),
        // No BOM
        _ => (u16::from_be_bytes, 0),
    };
    let utf16 = chunks[start..].iter().copied().map(endian);

    char::decode_utf16(utf16)
        .collect::<Result<String, _>>()
        .map_err(UtfError::UndecodableUtf16)
}

#[cfg(test)]
mod tests {
    const UTF_8: &str = "UTF-8";
    // rbook does not convert from UTF-16 without a BOM
    const UTF_16_LE: &[u8] = b"\xFF\xFE\x55\x00\x54\x00\x46\x00\x2D\x00\x38\x00";
    const UTF_16_BE: &[u8] = b"\xFE\xFF\x00\x55\x00\x54\x00\x46\x00\x2D\x00\x38";
    // Unsupported UTF-16; no BOM available for conversion
    const UTF_16_NO_BOM: &[u8] = b"\x55\x00\x54\x00\x46\x00\x2D\x00\x38\x00";
    // Malformed UTF-16
    const UTF_16_UNEVEN_BYTES: &[u8] = b"\xFF\xFE\x55";
    const UTF_16_UNPAIRED_SURROGATE: &[u8] = b"\xFF\xFE\x00\xD8\x41\x00";

    #[test]
    fn test_is_utf16() {
        assert!(super::is_utf16(UTF_16_LE));
        assert!(super::is_utf16(UTF_16_BE));
        assert!(super::is_utf16(UTF_16_UNEVEN_BYTES));
        assert!(!super::is_utf16(UTF_16_NO_BOM));
        assert!(!super::is_utf16(UTF_8.as_bytes()));
        assert!(!super::is_utf16(b""));
        assert!(!super::is_utf16(b"\xFF"));
        assert!(!super::is_utf16(b"\xFE"));
    }

    #[test]
    fn test_to_utf8() {
        let utf8_bytes = UTF_8.as_bytes();

        assert_eq!(utf8_bytes, super::into_utf8(utf8_bytes.to_vec()).unwrap());
        assert_eq!(utf8_bytes, super::into_utf8(UTF_16_LE.to_vec()).unwrap());
        assert_eq!(utf8_bytes, super::into_utf8(UTF_16_BE.to_vec()).unwrap());
        // No change; remains the same
        assert_eq!(
            UTF_16_NO_BOM,
            super::into_utf8(UTF_16_NO_BOM.to_vec()).unwrap()
        );
        assert!(super::into_utf8(UTF_16_UNEVEN_BYTES.to_vec()).is_err());
        assert!(super::into_utf8(UTF_16_UNPAIRED_SURROGATE.to_vec()).is_err());
    }

    #[test]
    fn test_to_utf8_str() {
        assert_eq!(UTF_8, super::into_utf8_str(UTF_8.into()).unwrap());
        assert_eq!(UTF_8, super::into_utf8_str(UTF_16_LE.to_vec()).unwrap());
        assert_eq!(UTF_8, super::into_utf8_str(UTF_16_BE.to_vec()).unwrap());

        // `x00` is a valid UTF8 character
        assert_eq!(
            "U\x00T\x00F\x00-\x008\x00",
            super::into_utf8_str(UTF_16_NO_BOM.to_vec()).unwrap(),
        );
        assert!(super::into_utf8_str(UTF_16_UNEVEN_BYTES.to_vec()).is_err());
    }

    #[test]
    fn test_from_utf16() {
        assert_eq!(UTF_8, super::from_utf16(UTF_16_LE).unwrap());
        assert_eq!(UTF_8, super::from_utf16(UTF_16_BE).unwrap());

        assert!(super::from_utf16(UTF_16_UNEVEN_BYTES).is_err());

        // Lack of a BOM means improper handling of endian byte order.
        // This scenario will never occur in the public API as conversion
        // from UTF-16 to UTF-8 is guarded by `is_utf16()` which checks
        // if there is a BOM before calling `from_utf16(...)`.
        assert!(super::from_utf16(UTF_16_NO_BOM).is_ok());
    }
}
