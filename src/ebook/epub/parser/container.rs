use crate::ebook::epub::consts;
use crate::ebook::epub::errors::EpubFormatError;
use crate::ebook::epub::parser::EpubParser;
use crate::parser::ParserResult;
use crate::parser::xml::{XmlElement, XmlReader};
use quick_xml::Reader;
use quick_xml::events::Event;

impl EpubParser<'_> {
    /// Parses `META-INF/container.xml` and retrieves the package `.opf` file location.
    pub(super) fn parse_container(&self, data: &[u8]) -> ParserResult<String> {
        let mut reader = Reader::from_reader(data);

        while let Some(event) = reader.next() {
            if let Event::Empty(el) | Event::Start(el) = event? {
                if !el.is_local_name(consts::ROOT_FILE) {
                    continue;
                }
                // Although rare, multiple package.opf locations could exist.
                // Only accept the first path as it is the default
                if let (Some(media_type), Some(full_path)) = (
                    el.get_attribute(consts::MEDIA_TYPE),
                    el.get_attribute(consts::FULL_PATH),
                ) {
                    if media_type == consts::PACKAGE_TYPE.as_bytes() {
                        let mut s = String::from_utf8(full_path.to_vec())?;
                        // Make location absolute
                        if !s.starts_with('/') {
                            s.insert(0, '/');
                        }
                        return Ok(s);
                    }
                }
            }
        }
        Err(EpubFormatError::NoOpfReference.into())
    }
}
