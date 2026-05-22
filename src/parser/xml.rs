//! UTF-8 XML parsing

use crate::ebook::element::{Attribute, AttributesData};
use crate::ebook::errors::FormatError;
use crate::util::str::StringExt;
use quick_xml::encoding::EncodingError;
use quick_xml::escape;
use quick_xml::events::attributes::{Attribute as QAttribute, Attributes as QAttributes};
use quick_xml::events::{BytesCData, BytesEnd, BytesRef, BytesStart, BytesText, Event as QEvent};
use quick_xml::{Decoder, Error as QError};
use std::borrow::Cow;
use std::str::{self, Utf8Error};

pub(crate) type XmlResult<T> = Result<T, XmlError>;

#[derive(thiserror::Error, Debug)]
pub(crate) enum XmlError {
    /// An error propagated from [`quick_xml`].
    #[error(transparent)]
    QError(#[from] QError),
    #[error(transparent)]
    InvalidUtf8(#[from] Utf8Error),
}

impl From<EncodingError> for XmlError {
    fn from(error: EncodingError) -> Self {
        Self::QError(QError::Encoding(error))
    }
}

impl From<XmlError> for FormatError {
    fn from(error: XmlError) -> Self {
        // Erase `XmlError` when converting it into `Unparsable`
        Self::Unparsable(match error {
            XmlError::QError(error) => Box::new(error),
            XmlError::InvalidUtf8(error) => Box::new(error),
        })
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
    CharRef(XmlCharRef<'a>),
    Eof,
    /// Skipped events:
    /// - [`QEvent::Comment`]
    /// - [`QEvent::Decl`]
    /// - [`QEvent::PI`]
    /// - [`QEvent::DocType`]
    Skipped,
}

impl<'a> XmlEvent<'a> {
    fn new(ctx: XmlContext, event: QEvent<'a>) -> Self {
        match event {
            // `Start` and `Empty` are merged for convenience.
            // - `XmlStartElement::is_self_closing` indicates if the element is empty.
            QEvent::Start(el) => XmlEvent::Start(XmlStartElement::new(ctx, el, false)),
            QEvent::Empty(el) => XmlEvent::Start(XmlStartElement::new(ctx, el, true)),
            QEvent::End(el) => XmlEvent::End(el),
            QEvent::Text(text) => XmlEvent::Text(text),
            QEvent::CData(text) => XmlEvent::CData(text),
            QEvent::GeneralRef(char_ref) => XmlEvent::CharRef(XmlCharRef::new(ctx, char_ref)),
            QEvent::Eof => XmlEvent::Eof,
            _ => XmlEvent::Skipped,
        }
    }
}

#[derive(Copy, Clone, Debug)]
pub(crate) struct XmlContext {
    decoder: Decoder,
    config: XmlConfig,
}

impl XmlContext {
    fn unescape_value(&self, bytes: &[u8]) -> XmlResult<String> {
        let decoded = self.decoder.decode(bytes)?;

        match escape::unescape(&decoded) {
            Ok(unescaped) => Ok(unescaped.into_owned()),
            Err(error) if self.config.strict => Err(XmlError::QError(error.into())),
            Err(_) => Ok(decoded.into_owned()),
        }
    }
}

#[derive(Copy, Clone, Debug)]
pub(crate) struct XmlConfig {
    pub(crate) strict: bool,
}

pub(crate) struct XmlReader<'a> {
    reader: quick_xml::Reader<&'a [u8]>,
    config: XmlConfig,
}

impl<'a> XmlReader<'a> {
    pub(crate) fn from_bytes(config: XmlConfig, reader: &'a [u8]) -> Self {
        Self {
            reader: quick_xml::Reader::from_reader(reader),
            config,
        }
    }

    fn ctx(&self) -> XmlContext {
        XmlContext {
            decoder: self.reader.decoder(),
            config: self.config,
        }
    }

    /// If `event` is [`Some`], takes the [`Event`] and returns it,
    /// otherwise invokes [`Self::next`].
    ///
    /// After this call, `event` **will** have a value of [`None`].
    pub(crate) fn take_or_next(
        &mut self,
        event: &mut Option<XmlEvent<'a>>,
    ) -> Option<XmlResult<XmlEvent<'a>>> {
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
    ) -> XmlResult<(Option<XmlEvent<'a>>, String)> {
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
                XmlEvent::CharRef(char_ref) => char_ref.decode_into(&mut value)?,
                _ => {}
            }
        }
        Ok((consumed_event, value))
    }

    /// Retrieve consolidated text for a specified element up to its end tag.
    pub(crate) fn get_element_text(&mut self, start: &XmlStartElement<'_>) -> XmlResult<String> {
        self.get_text(|event| matches!(event, XmlEvent::End(el) if el.name().0 == start.name()))
            .map(|(_, text)| text)
    }

    /// See [`Self::get_text`]
    pub(crate) fn get_text_till_either(
        &mut self,
        start: &[u8],
        till: &[u8],
    ) -> XmlResult<(Option<XmlEvent<'a>>, String)> {
        self.get_text(|event| {
            let predicate = |el| el == start || el == till;

            match event {
                XmlEvent::Start(el) if predicate(el.name()) => true,
                XmlEvent::End(el) if predicate(el.name().0) => true,
                _ => false,
            }
        })
    }

    fn handle_cdata(value: &mut String, cdata: &BytesCData) -> XmlResult<()> {
        value.push_str(cdata.decode()?.trim());
        Ok(())
    }

    fn handle_text(value: &mut String, text: &mut BytesText) -> XmlResult<()> {
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

impl<'a> Iterator for XmlReader<'a> {
    type Item = XmlResult<XmlEvent<'a>>;

    fn next(&mut self) -> Option<Self::Item> {
        match self
            .reader
            .read_event()
            .map(|event| XmlEvent::new(self.ctx(), event))
        {
            Ok(XmlEvent::Eof) => None,
            result => Some(result.map_err(XmlError::QError)),
        }
    }
}

pub(crate) struct XmlStartElement<'a> {
    ctx: XmlContext,
    element: BytesStart<'a>,
    is_self_closing: bool,
}

impl<'a> XmlStartElement<'a> {
    fn new(ctx: XmlContext, element: BytesStart<'a>, is_self_closing: bool) -> Self {
        Self {
            ctx,
            element,
            is_self_closing,
        }
    }

    pub(crate) fn name(&self) -> &[u8] {
        self.element.name().0
    }

    pub(crate) fn name_decoded(&self) -> XmlResult<Cow<'_, str>> {
        self.ctx
            .decoder
            .decode(self.name())
            .map_err(|error| XmlError::QError(error.into()))
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
    ) -> XmlResult<Option<Cow<'_, [u8]>>> {
        match self.element.try_get_attribute(key) {
            Ok(option) => Ok(option.map(|attribute| attribute.value)),
            Err(error) if self.ctx.config.strict => Err(XmlError::QError(error.into())),
            Err(_) => Ok(None),
        }
    }

    pub(crate) fn get_attribute(&self, key: impl AsRef<[u8]>) -> XmlResult<Option<String>> {
        self.get_attribute_raw(key).and_then(|value| match value {
            Some(value) => self.ctx.unescape_value(&value).map(Some),
            None => Ok(None),
        })
    }

    pub(crate) fn has_attribute(&self, key: impl AsRef<[u8]>) -> XmlResult<bool> {
        match self.element.try_get_attribute(key) {
            Ok(attribute) => Ok(attribute.is_some()),
            Err(error) if self.ctx.config.strict => Err(XmlError::QError(error.into())),
            Err(_) => Ok(false),
        }
    }

    pub(crate) fn attributes(&self) -> XmlAttributes<'_> {
        let mut attributes = self.element.attributes();
        attributes.with_checks(self.ctx.config.strict);

        XmlAttributes {
            ctx: self.ctx,
            attributes,
        }
    }
}

pub(crate) struct XmlAttribute<'a> {
    ctx: XmlContext,
    attribute: QAttribute<'a>,
}

impl<'a> XmlAttribute<'a> {
    pub(crate) fn name(&self) -> &[u8] {
        self.attribute.key.as_ref()
    }

    pub(crate) fn value_decoded(&self) -> XmlResult<String> {
        self.ctx.unescape_value(self.value())
    }

    pub(crate) fn value(&self) -> &[u8] {
        &self.attribute.value
    }

    pub(crate) fn into_value(self) -> Cow<'a, [u8]> {
        self.attribute.value
    }
}

impl TryFrom<XmlAttribute<'_>> for Attribute {
    type Error = XmlError;

    fn try_from(attribute: XmlAttribute) -> Result<Self, Self::Error> {
        let name = str::from_utf8(attribute.name())
            .map_err(XmlError::from)?
            .to_owned();
        let value = attribute.value_decoded()?;
        Ok(Attribute::create(name, value))
    }
}

pub(crate) struct XmlAttributes<'a> {
    ctx: XmlContext,
    attributes: QAttributes<'a>,
}

impl<'a> Iterator for XmlAttributes<'a> {
    type Item = XmlResult<XmlAttribute<'a>>;

    fn next(&mut self) -> Option<Self::Item> {
        self.attributes.next().map(|result| {
            result
                .map(|attribute| XmlAttribute {
                    ctx: self.ctx,
                    attribute,
                })
                .map_err(|error| XmlError::QError(error.into()))
        })
    }
}

impl TryFrom<XmlAttributes<'_>> for AttributesData {
    type Error = XmlError;

    fn try_from(attributes: XmlAttributes<'_>) -> Result<Self, Self::Error> {
        attributes
            .map(|result| result.and_then(TryInto::try_into))
            .collect::<XmlResult<Vec<Attribute>>>()
            .map(AttributesData::from)
    }
}

pub(crate) struct XmlCharRef<'a> {
    ctx: XmlContext,
    reference: BytesRef<'a>,
}

impl<'a> XmlCharRef<'a> {
    pub(crate) fn new(ctx: XmlContext, reference: BytesRef<'a>) -> Self {
        Self { ctx, reference }
    }

    pub(crate) fn decode_into(&self, buffer: &mut String) -> XmlResult<()> {
        fn push_unsupported(value: &mut String, reference: &str) {
            // Unsupported custom entity/character reference
            // - This is a rare scenario if there are non-standard entities/char refs.
            // - NOTE: Despite this being a safe option when parsing,
            //   when writing back, the unresolved entity/ref will be double-escaped.
            value.push('&');
            value.push_str(reference);
            value.push(';');
        }

        if self.reference.is_char_ref() {
            match self.reference.resolve_char_ref() {
                Ok(Some(resolved)) => buffer.push(resolved),
                // The `None` case should never happen as
                // `is_char_ref` was called before resolving
                Ok(None) => {}
                // An invalid char ref is given
                Err(QError::Escape(_)) if !self.ctx.config.strict => {
                    push_unsupported(buffer, &self.reference.decode()?);
                }
                Err(error) => return Err(XmlError::QError(error)),
            }
        } else {
            let decoded = self.reference.decode()?;

            // Resolve xml/html entity
            match escape::resolve_predefined_entity(&decoded) {
                Some(resolved) => buffer.push_str(resolved),
                None => push_unsupported(buffer, &decoded),
            }
        }
        Ok(())
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

macro_rules! extract_attributes {
    {
        $attributes:expr,
        $($mapping:path $(where $cond:expr)? => $var:ident $(as |$attr:ident| $map:expr)?,)*
        $(..$remaining:ident,)?
    } => {
        $(let mut $remaining = Vec::new();)?
        $(let mut $var = None;)*

        for result in $attributes {
            let attribute = result?;

            match attribute.name() {
                $(
                $mapping $(if $cond)? => $var = Some(
                    extract_attributes!(@value_helper attribute $( $attr $map )?)
                ),
                )*
                _ => {
                    $($remaining.push(attribute.try_into()?);)?
                }
            }
        }
    };
    (@value_helper $attribute:ident $attr:ident $map:expr) => {{
        let $attr = $attribute;
        $map
    }};
    (@value_helper $attribute:ident) => {{
        $attribute.value_decoded()?
    }};
}

pub(crate) use extract_attributes;

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
