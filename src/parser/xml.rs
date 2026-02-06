//! UTF-8 XML parsing

use crate::ebook::element::{Attribute, AttributesData};
use crate::ebook::errors::FormatError;
use crate::parser::ParserResult;
use crate::util::str::StringExt;
use quick_xml::Decoder;
use quick_xml::encoding::EncodingError;
use quick_xml::escape;
use quick_xml::events::attributes::Attribute as QuickXmlAttribute;
use quick_xml::events::{BytesCData, BytesEnd, BytesRef, BytesStart, BytesText, Event};
use std::borrow::Cow;
use std::str;

#[doc(hidden)]
impl From<EncodingError> for FormatError {
    fn from(error: EncodingError) -> Self {
        Self::Unparsable(Box::new(error))
    }
}

pub(crate) enum XmlEvent<'a> {
    /// Represent a start element:
    /// - `<start x="y"></start>`
    /// - `<start x="y"/>`
    Start(XmlStartElement<'a>),
    End(BytesEnd<'a>),
    Text(BytesText<'a>),
    CData(BytesCData<'a>),
    GeneralRef(BytesRef<'a>),
    Eof,
    /// Skipped events:
    /// - [`quick_xml::events::Comment`]
    /// - [`quick_xml::events::Decl`]
    /// - [`quick_xml::events::PI`]
    /// - [`quick_xml::events::DocType`]
    Skipped,
}

impl<'a> XmlEvent<'a> {
    fn new(ctx: XmlReaderContext, event: Event<'a>) -> Self {
        match event {
            // `Start` and `Empty` are merged for convenience.
            // - `XmlStartElement::is_self_closing` indicates if the element is empty.
            Event::Start(e) => XmlEvent::Start(XmlStartElement::new(ctx, e, false)),
            Event::Empty(e) => XmlEvent::Start(XmlStartElement::new(ctx, e, true)),
            Event::End(e) => XmlEvent::End(e),
            Event::Text(e) => XmlEvent::Text(e),
            Event::CData(e) => XmlEvent::CData(e),
            Event::GeneralRef(e) => XmlEvent::GeneralRef(e),
            Event::Eof => XmlEvent::Eof,
            _ => XmlEvent::Skipped,
        }
    }
}

#[derive(Copy, Clone, Debug)]
pub(crate) struct XmlReaderContext {
    decoder: Decoder,
    strict: bool,
}

impl XmlReaderContext {
    fn unescape_value(&self, bytes: &[u8]) -> ParserResult<String> {
        let decoded = self.decoder.decode(bytes)?;

        match escape::unescape(&decoded) {
            Ok(unescaped) => Ok(unescaped.into_owned()),
            Err(error) if self.strict => Err(FormatError::Unparsable(Box::new(error))),
            Err(_) => Ok(decoded.into_owned()),
        }
    }
}

pub(crate) struct XmlReader<'a> {
    reader: quick_xml::Reader<&'a [u8]>,
    strict: bool,
}

impl<'a> XmlReader<'a> {
    pub(crate) fn from_bytes(strict: bool, reader: &'a [u8]) -> Self {
        Self {
            reader: quick_xml::Reader::from_reader(reader),
            strict,
        }
    }

    fn ctx(&self) -> XmlReaderContext {
        XmlReaderContext {
            decoder: self.reader.decoder(),
            strict: self.strict,
        }
    }

    /// Iterator-like method to read the next [`Event`].
    pub(crate) fn next(&mut self) -> Option<ParserResult<XmlEvent<'a>>> {
        match self
            .reader
            .read_event()
            .map(|event| XmlEvent::new(self.ctx(), event))
        {
            Ok(XmlEvent::Eof) => None,
            result => Some(result.map_err(|error| FormatError::Unparsable(Box::new(error)))),
        }
    }

    /// If `event` is [`Some`], takes the [`Event`] and returns it,
    /// otherwise invokes [`Self::next`].
    ///
    /// After this call, `event` **will** have a value of [`None`].
    pub(crate) fn take_or_next(
        &mut self,
        event: &mut Option<XmlEvent<'a>>,
    ) -> Option<ParserResult<XmlEvent<'a>>> {
        event.take().map(Ok).or_else(|| self.next())
    }

    /// Retrieve all text until a designated stopping point.
    /// During the process, when the stopping point is reached,
    /// the event that caused the stop is consumed from the reader,
    /// which may need to also be read by the caller.
    ///
    /// As such, returns the [`Event`] that caused the stop and text.
    fn get_text(
        &mut self,
        mut is_stop: impl FnMut(&XmlEvent) -> bool,
    ) -> ParserResult<(Option<XmlEvent<'a>>, String)> {
        let mut value = String::new();
        let mut consumed_event = None;

        while let Some(result) = self.next() {
            let event = result?;

            if is_stop(&event) {
                value.trim_in_place();
                consumed_event.replace(event);
                break;
            }
            match event {
                XmlEvent::Text(mut text) => Self::handle_text(&mut value, &mut text)?,
                XmlEvent::CData(cdata) => Self::handle_cdata(&mut value, &cdata)?,
                XmlEvent::GeneralRef(general_ref) => {
                    self.handle_general_ref(&mut value, &general_ref)?;
                }
                _ => {}
            }
        }
        Ok((consumed_event, value))
    }

    /// Retrieve consolidated text for a specified element up to its end tag.
    pub(crate) fn get_element_text(&mut self, start: &XmlStartElement<'_>) -> ParserResult<String> {
        self.get_text(|event| matches!(event, XmlEvent::End(el) if el.name().0 == start.name()))
            .map(|(_, text)| text)
    }

    /// See [`Self::get_text`]
    pub(crate) fn get_text_till_either(
        &mut self,
        start: &[u8],
        till: &[u8],
    ) -> ParserResult<(Option<XmlEvent<'a>>, String)> {
        self.get_text(|event| {
            let predicate = |el| el == start || el == till;

            match event {
                XmlEvent::Start(el) if predicate(el.name()) => true,
                XmlEvent::End(el) if predicate(el.name().0) => true,
                _ => false,
            }
        })
    }

    fn handle_general_ref(&self, value: &mut String, general_ref: &BytesRef) -> ParserResult<()> {
        fn push_unsupported(value: &mut String, reference: &str) {
            // Unsupported custom entity/character reference
            // - This is a rare scenario if there are non-standard entities/char refs.
            // - NOTE: Despite this being a safe option when parsing,
            //   when writing back, the unresolved entity/ref will be double-escaped.
            value.push('&');
            value.push_str(reference);
            value.push(';');
        }

        if general_ref.is_char_ref() {
            match general_ref.resolve_char_ref() {
                Ok(Some(resolved)) => value.push(resolved),
                // The `None` case should never happen as
                // `is_char_ref` was called before resolving
                Ok(None) => (),
                // An invalid char ref is given
                Err(quick_xml::Error::Escape(_)) if !self.strict => {
                    push_unsupported(value, &general_ref.decode()?);
                }
                Err(error) => return Err(FormatError::Unparsable(Box::new(error))),
            }
        } else {
            let decoded = general_ref.decode()?;

            // Resolve xml/html entity
            match escape::resolve_predefined_entity(&decoded) {
                Some(resolved) => value.push_str(resolved),
                None => push_unsupported(value, &decoded),
            }
        }
        Ok(())
    }

    fn handle_cdata(value: &mut String, cdata: &BytesCData) -> ParserResult<()> {
        value.push_str(cdata.decode()?.trim());
        Ok(())
    }

    fn handle_text(value: &mut String, text: &mut BytesText) -> ParserResult<()> {
        // Determine when to add spacing
        let has_padding_start = text.try_trim_start();
        let has_padding_end = text.try_trim_end();
        let last_char = value.chars().last().unwrap_or_default();

        // Check "start" spacing
        if (text.is_empty() || has_padding_start) && last_char != ' ' {
            // Only add spacing if there's content
            if !value.is_empty() {
                // Add a space to ensure all text doesn't squeeze together
                value.push(' ');
            }
            // Return early if there is no text to process
            if text.is_empty() {
                return Ok(());
            }
        }
        let text = text.decode()?;

        // Consolidate into a single paragraph
        for text in text.lines().map(str::trim).filter(|s| !s.is_empty()) {
            value.push_str(text);
            value.push(' ');
        }
        // If there should be no end spacing, remove the last space added by the loop
        if !has_padding_end {
            value.pop();
        }
        Ok(())
    }
}

pub(crate) struct XmlStartElement<'a> {
    ctx: XmlReaderContext,
    element: BytesStart<'a>,
    is_self_closing: bool,
}

impl<'a> XmlStartElement<'a> {
    fn new(ctx: XmlReaderContext, element: BytesStart<'a>, is_self_closing: bool) -> Self {
        Self {
            ctx,
            element,
            is_self_closing,
        }
    }

    pub(crate) fn name(&self) -> &[u8] {
        self.element.name().0
    }

    pub(crate) fn name_decoded(&self) -> ParserResult<Cow<'_, str>> {
        self.ctx
            .decoder
            .decode(self.name())
            .map_err(|error| FormatError::Unparsable(Box::new(error)))
    }

    pub(crate) fn local_name(&self) -> &[u8] {
        self.element.local_name().into_inner()
    }

    pub(crate) fn is_local_name(&self, target_local_name: impl AsRef<[u8]>) -> bool {
        self.local_name() == target_local_name.as_ref()
    }

    pub(crate) fn is_prefix(&self, target_prefix: impl AsRef<[u8]>) -> bool {
        self.element
            .name()
            .prefix()
            .is_some_and(|p| p.as_ref() == target_prefix.as_ref())
    }

    pub(crate) fn is_self_closing(&self) -> bool {
        self.is_self_closing
    }

    /// Returns the raw attribute value
    pub(crate) fn get_attribute_raw(
        &self,
        key: impl AsRef<[u8]>,
    ) -> ParserResult<Option<Cow<'_, [u8]>>> {
        match self.element.try_get_attribute(key) {
            Ok(option) => Ok(option.map(|attribute| attribute.value)),
            Err(error) if self.ctx.strict => Err(FormatError::Unparsable(Box::new(error))),
            Err(_) => Ok(None),
        }
    }

    pub(crate) fn get_attribute(&self, key: impl AsRef<[u8]>) -> ParserResult<Option<String>> {
        self.get_attribute_raw(key).and_then(|value| match value {
            Some(value) => self.ctx.unescape_value(&value).map(Some),
            None => Ok(None),
        })
    }

    pub(crate) fn has_attribute(&self, key: impl AsRef<[u8]>) -> ParserResult<bool> {
        match self.element.try_get_attribute(key) {
            Ok(attribute) => Ok(attribute.is_some()),
            Err(error) if self.ctx.strict => Err(FormatError::Unparsable(Box::new(error))),
            Err(_) => Ok(false),
        }
    }

    pub(crate) fn attributes(&self) -> ParserResult<XmlAttributes<'_>> {
        let attributes = self
            .element
            .attributes()
            .filter_map(|result| match result {
                Ok(_) => Some(result),
                Err(_) if self.ctx.strict => Some(result),
                // Skip the attribute if not `strict`
                Err(_) => None,
            })
            .collect::<Result<Vec<_>, _>>()
            .map_err(|error| FormatError::Unparsable(Box::new(error)))?;

        Ok(XmlAttributes {
            ctx: self.ctx,
            attributes,
        })
    }
}

pub(crate) struct XmlAttributes<'a> {
    ctx: XmlReaderContext,
    attributes: Vec<QuickXmlAttribute<'a>>,
}

impl XmlAttributes<'_> {
    /// Removes and returns the value of the attribute by `name`.
    pub(crate) fn remove(&mut self, name: impl AsRef<[u8]>) -> ParserResult<Option<String>> {
        let name = name.as_ref();

        self.attributes
            .iter()
            .position(|attribute| attribute.key.as_ref() == name)
            .map(|i| {
                let raw = &self.attributes.swap_remove(i).value;
                self.ctx.unescape_value(raw)
            })
            .transpose()
    }
}

impl TryFrom<XmlAttributes<'_>> for AttributesData {
    type Error = FormatError;

    fn try_from(attributes: XmlAttributes<'_>) -> Result<Self, Self::Error> {
        attributes
            .attributes
            .iter()
            .map(|attribute| {
                let name = str::from_utf8(attribute.key.as_ref())
                    .map_err(|err| FormatError::Unparsable(Box::new(err)))?
                    .to_owned();
                let value = attributes.ctx.unescape_value(&attribute.value)?;
                Ok(Attribute::create(name, value))
            })
            .collect::<Result<Vec<_>, _>>()
            .map(AttributesData::from)
    }
}

pub(crate) trait XmlText {
    /// Returns `true` if the start was trimmed.
    fn try_trim_start(&mut self) -> bool;

    /// Returns `true` if the end was trimmed.
    fn try_trim_end(&mut self) -> bool;
}

impl XmlText for BytesText<'_> {
    fn try_trim_start(&mut self) -> bool {
        let before = self.len();
        self.inplace_trim_start();
        self.len() != before
    }

    fn try_trim_end(&mut self) -> bool {
        let before = self.len();
        self.inplace_trim_end();
        self.len() != before
    }
}

#[cfg(test)]
mod tests {
    use super::XmlReader;
    use quick_xml::events::BytesText;

    #[test]
    fn test_text_to_str() {
        let mut s = String::new();
        XmlReader::handle_text(
            &mut s,
            &mut BytesText::from_escaped(" \n \t \r  data1 &amp; data2  \n\r \n  \t "),
        )
        .unwrap();
        // If there is whitespace at the end, a single `space` must replace it all.
        assert_eq!("data1 &amp; data2 ", s);

        XmlReader::handle_text(&mut s, &mut BytesText::from_escaped("data3 ")).unwrap();
        assert_eq!("data1 &amp; data2 data3 ", s);

        XmlReader::handle_text(&mut s, &mut BytesText::from_escaped("  data4")).unwrap();
        assert_eq!("data1 &amp; data2 data3 data4", s);

        XmlReader::handle_text(&mut s, &mut BytesText::from_escaped("data5")).unwrap();
        assert_eq!("data1 &amp; data2 data3 data4data5", s);
    }
}
