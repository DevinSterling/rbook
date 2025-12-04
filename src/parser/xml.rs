use crate::ebook::element::AttributeData;
use crate::ebook::errors::FormatError;
use crate::parser::ParserResult;
use crate::util::StringExt;
use quick_xml::Reader;
use quick_xml::encoding::EncodingError;
use quick_xml::escape;
use quick_xml::events::attributes::Attribute;
use quick_xml::events::{BytesCData, BytesRef, BytesStart, BytesText, Event};
use std::borrow::Cow;
use std::str;
use std::string::FromUtf8Error;

pub(crate) type ByteReader<'a> = Reader<&'a [u8]>;

impl From<quick_xml::Error> for FormatError {
    fn from(error: quick_xml::Error) -> Self {
        Self::Unparsable(Box::new(error))
    }
}

impl From<EncodingError> for FormatError {
    fn from(error: EncodingError) -> Self {
        Self::Unparsable(Box::new(error))
    }
}

pub(crate) trait XmlReader<'a> {
    /// Iterator-like method to read the next [`Event`].
    fn next(&mut self) -> Option<ParserResult<Event<'a>>>;

    /// If `event` is [`Some`], takes the [`Event`] and returns it,
    /// otherwise invokes [`Self::next`].
    ///
    /// After this call, `event` **will** have a value of [`None`].
    fn take_or_next(&mut self, event: &mut Option<Event<'a>>) -> Option<ParserResult<Event<'a>>> {
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
        mut is_stop: impl FnMut(&Event) -> bool,
    ) -> ParserResult<(Option<Event<'a>>, String)> {
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
                Event::Text(mut text) => handle_text(&mut value, &mut text)?,
                Event::CData(cdata) => handle_cdata(&mut value, &cdata)?,
                Event::GeneralRef(general_ref) => handle_general_ref(&mut value, &general_ref)?,
                _ => {}
            }
        }
        Ok((consumed_event, value))
    }

    /// Retrieve consolidated text for a specified element up to its end tag.
    fn get_text_simple(&mut self, start: &BytesStart) -> ParserResult<String> {
        self.get_text(|event| matches!(event, Event::End(el) if el.name() == start.name()))
            .map(|x| x.1)
    }

    /// See [`Self::get_text`]
    fn get_text_till_either(
        &mut self,
        start: &BytesStart,
        till: &BytesStart,
    ) -> ParserResult<(Option<Event<'a>>, String)> {
        self.get_text(|event| {
            let predicate = |el| el == start.name() || el == till.name();

            match event {
                Event::Start(el) | Event::Empty(el) if predicate(el.name()) => true,
                Event::End(el) if predicate(el.name()) => true,
                _ => false,
            }
        })
    }
}

impl<'a> XmlReader<'a> for ByteReader<'a> {
    fn next(&mut self) -> Option<ParserResult<Event<'a>>> {
        match self.read_event() {
            Ok(Event::Eof) => None,
            result => Some(result.map_err(|error| FormatError::Unparsable(Box::new(error)))),
        }
    }
}

pub(crate) trait XmlElement<'a> {
    fn is_local_name(&self, local_name: impl AsRef<[u8]>) -> bool;

    fn is_prefix(&self, prefix: impl AsRef<[u8]>) -> bool;

    fn get_attribute(&self, key: impl AsRef<[u8]>) -> Option<Cow<'_, [u8]>>;

    fn has_attribute(&self, key: impl AsRef<[u8]>) -> bool;

    fn bytes_attributes(&self) -> BytesAttributes<'_>;
}

impl<'a> XmlElement<'a> for BytesStart<'a> {
    fn is_local_name(&self, target_local_name: impl AsRef<[u8]>) -> bool {
        self.local_name().as_ref() == target_local_name.as_ref()
    }

    fn is_prefix(&self, target_prefix: impl AsRef<[u8]>) -> bool {
        self.name()
            .prefix()
            .is_some_and(|p| p.as_ref() == target_prefix.as_ref())
    }

    fn get_attribute(&self, key: impl AsRef<[u8]>) -> Option<Cow<'_, [u8]>> {
        match self.try_get_attribute(key) {
            Ok(option) => option.map(|attribute| attribute.value),
            Err(_) => None,
        }
    }

    fn has_attribute(&self, key: impl AsRef<[u8]>) -> bool {
        self.try_get_attribute(key).ok().flatten().is_some()
    }

    fn bytes_attributes(&self) -> BytesAttributes<'_> {
        BytesAttributes(self.attributes().filter_map(Result::ok).collect())
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

pub(crate) struct BytesAttributes<'a>(Vec<Attribute<'a>>);

impl BytesAttributes<'_> {
    /// Removes and returns the value of the attribute by `name`.
    pub(crate) fn remove(
        &mut self,
        name: impl AsRef<[u8]>,
    ) -> Result<Option<String>, FromUtf8Error> {
        let name = name.as_ref();

        self.0
            .iter()
            .position(|attribute| attribute.key.as_ref() == name)
            .map(|i| String::from_utf8(self.0.swap_remove(i).value.into_owned()))
            .transpose()
    }
}

impl TryFrom<BytesAttributes<'_>> for Vec<AttributeData> {
    type Error = FromUtf8Error;

    fn try_from(attributes: BytesAttributes<'_>) -> Result<Self, Self::Error> {
        attributes
            .0
            .into_iter()
            .map(AttributeData::try_from)
            .collect()
    }
}

impl TryFrom<Attribute<'_>> for AttributeData {
    type Error = FromUtf8Error;

    fn try_from(attribute: Attribute<'_>) -> Result<Self, Self::Error> {
        let name = attribute.key.0;
        let value = attribute.value.into_owned();

        Ok(Self::new(
            str::from_utf8(name)
                .map(Cow::Borrowed)
                // fallback; this generally should never occur
                .or_else(|_| String::from_utf8(name.to_vec()).map(Cow::Owned))?,
            String::from_utf8(value)?,
        ))
    }
}

// Helper methods
fn handle_general_ref(value: &mut String, general_ref: &BytesRef) -> ParserResult<()> {
    if general_ref.is_char_ref() {
        if let Some(resolved) = general_ref.resolve_char_ref()? {
            value.push(resolved);
        }
    } else {
        let decoded = general_ref.decode()?;

        match escape::resolve_xml_entity(decoded.as_ref()) {
            Some(resolved) => value.push_str(resolved.as_ref()),
            // Unsupported entity
            None => value.push_str(format!("&{decoded};").as_ref()),
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

#[cfg(test)]
mod tests {
    use quick_xml::events::BytesText;

    #[test]
    fn test_text_to_str() {
        let mut s = String::new();
        super::handle_text(
            &mut s,
            &mut BytesText::from_escaped(" \n \t \r  data1 &amp; data2  \n\r \n  \t "),
        )
        .unwrap();
        // If there is whitespace at the end, a single `space` must replace it all.
        assert_eq!("data1 &amp; data2 ", s);

        super::handle_text(&mut s, &mut BytesText::from_escaped("data3 ")).unwrap();
        assert_eq!("data1 &amp; data2 data3 ", s);

        super::handle_text(&mut s, &mut BytesText::from_escaped("  data4")).unwrap();
        assert_eq!("data1 &amp; data2 data3 data4", s);

        super::handle_text(&mut s, &mut BytesText::from_escaped("data5")).unwrap();
        assert_eq!("data1 &amp; data2 data3 data4data5", s);
    }
}
