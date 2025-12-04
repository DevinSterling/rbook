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
            let el = match event? {
                Event::Empty(el) | Event::Start(el) if el.is_local_name(consts::ROOT_FILE) => el,
                _ => continue,
            };
            // Although rare, multiple package.opf locations could exist.
            // Only accept the first path as it is the default
            let (Some(consts::bytes::PACKAGE_TYPE), Some(full_path)) = (
                el.get_attribute(consts::MEDIA_TYPE).as_deref(),
                el.get_attribute(consts::FULL_PATH),
            ) else {
                continue;
            };

            let mut package_file = String::from_utf8(full_path.to_vec())?;
            // Make location absolute
            if !package_file.starts_with('/') {
                package_file.insert(0, '/');
            }
            return self.require_encoded(package_file);
        }
        Err(EpubFormatError::NoOpfReference.into())
    }
}
