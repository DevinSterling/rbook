use crate::ebook::element::AttributeData;
use crate::ebook::errors::FormatError;
use crate::parser::ParserResult;
use crate::util::StringExt;
use quick_xml::Reader;
use quick_xml::events::attributes::Attribute;
use quick_xml::events::{BytesCData, BytesStart, BytesText, Event};
use std::borrow::Cow;
use std::str;
use std::string::FromUtf8Error;

pub(crate) type ByteReader<'a> = Reader<&'a [u8]>;

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
    /// As such, upon return, `last_event` is set to the event that caused the stop.
    fn get_text(
        &mut self,
        last_event: &mut Option<Event<'a>>,
        mut stop: impl FnMut(&Event) -> bool,
    ) -> ParserResult<String> {
        let mut value = String::new();

        while let Some(result) = self.next() {
            let event = result?;

            if stop(&event) {
                value.trim_in_place();
                last_event.replace(event);
                break;
            }
            match event {
                Event::Text(mut text) => text_to_str(&mut value, &mut text),
                Event::CData(cdata) => cdata_to_str(&mut value, &cdata),
                _ => {}
            }
        }
        Ok(value)
    }

    /// Retrieve consolidated text for a specified element up to its end tag.
    fn get_text_simple(&mut self, start: &BytesStart) -> ParserResult<String> {
        self.get_text(
            &mut None,
            |event| matches!(event, Event::End(el) if el.name() == start.name()),
        )
    }

    /// See [`Self::get_text`]
    fn get_text_till_either(
        &mut self,
        last_event: &mut Option<Event<'a>>,
        start: &BytesStart,
        till: &BytesStart,
    ) -> ParserResult<String> {
        self.get_text(last_event, |event| {
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

    fn get_attribute(&self, key: impl AsRef<[u8]>) -> Option<Cow<[u8]>>;

    fn bytes_attributes(&self) -> BytesAttributes;
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

    fn get_attribute(&self, key: impl AsRef<[u8]>) -> Option<Cow<[u8]>> {
        match self.try_get_attribute(key) {
            Ok(option) => option.map(|attribute| attribute.value),
            Err(_) => None,
        }
    }

    fn bytes_attributes(&self) -> BytesAttributes {
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
    fn take_attribute(&mut self, name: &[u8]) -> Option<Result<String, FromUtf8Error>> {
        self.0
            .iter()
            .position(|attribute| attribute.key.as_ref() == name)
            .map(|i| String::from_utf8(self.0.swap_remove(i).value.into_owned()))
    }

    /// Removes and returns the value of the attribute by `name`.
    pub(crate) fn take_attribute_value(
        &mut self,
        name: impl AsRef<[u8]>,
    ) -> Result<Option<String>, FromUtf8Error> {
        self.take_attribute(name.as_ref()).transpose()
    }

    /// Removes and returns the first attribute value matching any of `names`.
    pub(crate) fn take_attribute_value_any(
        &mut self,
        names: impl IntoIterator<Item = impl AsRef<[u8]>>,
    ) -> Result<Option<String>, FromUtf8Error> {
        names
            .into_iter()
            .find_map(|name| self.take_attribute(name.as_ref()))
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
fn cdata_to_str(value: &mut String, cdata: &BytesCData) {
    let text = cdata
        .decode()
        .unwrap_or_else(|_| String::from_utf8_lossy(cdata.as_ref()));

    value.push_str(text.trim());
}

fn text_to_str(value: &mut String, text: &mut BytesText) {
    // Determine when to add spacing
    let has_padding_start = text.try_trim_start();
    let has_padding_end = text.try_trim_end();
    let last_char = value.chars().last().unwrap_or_default();

    // Check "start" spacing
    if (text.is_empty() || has_padding_start) && last_char != ' ' {
        // Only add spacing if there's content
        if !value.is_empty() {
            // Add a space to ensure that text doesn't squeeze together
            value.push(' ');
        }
        // Return early if there is no text to process
        if text.is_empty() {
            return;
        }
    }
    let text = text
        .unescape()
        .unwrap_or_else(|_| String::from_utf8_lossy(text.as_ref()));

    // Consolidate into a single paragraph
    for text in text.lines().map(str::trim).filter(|s| !s.is_empty()) {
        value.push_str(text);
        value.push(' ');
    }
    // If there should be no end spacing,
    // get rid of the last space from loop
    if !has_padding_end {
        value.pop();
    }
}

#[cfg(test)]
mod tests {
    use quick_xml::events::BytesText;

    #[test]
    fn test_text_to_str() {
        let mut s = String::new();
        super::text_to_str(
            &mut s,
            &mut BytesText::from_escaped(" \n \t \r  data1 &amp; data2  \n\r \n  \t "),
        );
        // If there is whitespace at the end, a single `space` must replace it all.
        assert_eq!("data1 & data2 ", s);

        super::text_to_str(&mut s, &mut BytesText::from_escaped("data3 "));
        assert_eq!("data1 & data2 data3 ", s);

        super::text_to_str(&mut s, &mut BytesText::from_escaped("  data4"));
        assert_eq!("data1 & data2 data3 data4", s);

        super::text_to_str(&mut s, &mut BytesText::from_escaped("data5"));
        assert_eq!("data1 & data2 data3 data4data5", s);
    }
}
