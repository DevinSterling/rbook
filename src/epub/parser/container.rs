use crate::ebook::resource::consts::mime;
use crate::epub::consts::{ocf, ocf::bytes};
use crate::epub::errors::EpubError;
use crate::epub::parser::{EpubParser, EpubParserValidator};
use crate::parser::ParserResult;
use crate::parser::xml::{XmlEvent, XmlReader, extract_attributes};
use crate::util::uri;

impl EpubParser<'_> {
    /// Parses `META-INF/container.xml` and retrieves the package `.opf` file location.
    pub(super) fn parse_container(&self, data: &[u8]) -> ParserResult<String> {
        let mut reader = XmlReader::from_bytes(self.xml_config(), data);

        while let Some(event) = reader.next() {
            let el = match event? {
                XmlEvent::Start(el) if el.is_local_name(ocf::ROOT_FILE) => el,
                _ => continue,
            };
            extract_attributes! {
                el.attributes(),
                bytes::MEDIA_TYPE => media_type as |attr| attr.into_value(),
                bytes::FULL_PATH => full_path,
            }

            // Although rare, multiple package.opf locations could exist.
            // Only accept the first path as it is the default
            let (Some(mime::bytes::OEBPS_PACKAGE), Some(package_file)) =
                (media_type.as_deref(), full_path)
            else {
                continue;
            };

            // Make location absolute
            return self.require_href(uri::into_absolute(package_file));
        }
        Err(EpubError::NoOpfReference.into())
    }
}
