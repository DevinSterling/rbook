use crate::ebook::epub::consts::ocf;
use crate::ebook::epub::errors::EpubError;
use crate::ebook::epub::parser::{EpubParser, EpubParserValidator};
use crate::ebook::resource::consts::mime;
use crate::parser::ParserResult;
use crate::parser::xml::{XmlEvent, XmlReader};
use crate::util::uri;

impl EpubParser<'_> {
    /// Parses `META-INF/container.xml` and retrieves the package `.opf` file location.
    pub(super) fn parse_container(&self, data: &[u8]) -> ParserResult<String> {
        let mut reader = XmlReader::from_bytes(self.is_strict(), data);

        while let Some(event) = reader.next() {
            let el = match event? {
                XmlEvent::Start(el) if el.is_local_name(ocf::ROOT_FILE) => el,
                _ => continue,
            };
            // Although rare, multiple package.opf locations could exist.
            // Only accept the first path as it is the default
            let (Some(mime::bytes::OEBPS_PACKAGE), Some(package_file)) = (
                el.get_attribute_raw(ocf::MEDIA_TYPE)?.as_deref(),
                el.get_attribute(ocf::FULL_PATH)?,
            ) else {
                continue;
            };

            // Make location absolute
            return self.require_href(uri::into_absolute(package_file));
        }
        Err(EpubError::NoOpfReference.into())
    }
}
