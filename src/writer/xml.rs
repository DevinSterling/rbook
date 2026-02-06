use crate::writer::WriterResult;
use quick_xml::events::attributes::Attribute;
use quick_xml::events::{BytesDecl, BytesEnd, BytesStart, BytesText, Event};
use std::borrow::Cow;
use std::io::Write;

pub(crate) struct XmlWriter<'a, W> {
    writer: quick_xml::Writer<W>,
    start_element: Option<BytesStart<'a>>,
}

impl<'a, W: Write> XmlWriter<'a, W> {
    pub(crate) fn new(writer: W) -> Self {
        Self {
            writer: quick_xml::Writer::new_with_indent(writer, b' ', 2),
            start_element: None,
        }
    }

    pub(crate) fn write_utf8_declaration(&mut self) -> WriterResult<&mut Self> {
        const XML_VERSION: &str = "1.0";
        const XML_ENCODING: &str = "UTF-8";

        self.writer.write_event(Event::Decl(BytesDecl::new(
            XML_VERSION,
            Some(XML_ENCODING),
            None,
        )))?;

        Ok(self)
    }

    /// Start an element: `<tag`
    pub(crate) fn start_element(&mut self, tag: &'a str) -> WriterResult<&mut Self> {
        // For ergonomics, close the previous element.
        // This is useful when creating an inner element within a parent.
        // (e.g., `<parent><nested`)
        self.finish_start_element()?;

        self.start_element = Some(BytesStart::new(tag));
        Ok(self)
    }

    /// Append an attribute to the [started](Self::start_element) element: `<tag name="value"`
    pub(crate) fn add_attribute<'b>(
        &mut self,
        name: &str,
        value: impl Into<Option<&'b str>>,
    ) -> &mut Self {
        if let (Some(element), Some(value)) = (&mut self.start_element, value.into()) {
            element.push_attribute(new_escaped_attribute(name, value));
        }
        self
    }

    pub(crate) fn add_attributes<'b>(
        &mut self,
        iter: impl IntoIterator<Item = (&'b str, &'b str)>,
    ) -> &mut Self {
        if let Some(element) = &mut self.start_element {
            element.extend_attributes(
                iter.into_iter()
                    .map(|(name, value)| new_escaped_attribute(name, value)),
            );
        }
        self
    }

    // End states
    /// Finish writing a start element: **`<parent>`**
    ///
    /// See [`Self::finish_end_element`] to write the closing tag.
    pub(crate) fn finish_start_element(&mut self) -> WriterResult<()> {
        if let Some(element) = self.start_element.take() {
            self.writer.write_event(Event::Start(element))?;
        }
        Ok(())
    }

    /// Finish writing a parent element: **`<parent><inner/></parent>`**
    pub(crate) fn finish_end_element(&mut self, tag: &str) -> WriterResult<()> {
        // Ensure the start element is closed with `>` (e.g., <start → <start>)
        // If `finish_start_element` isn't called before this method, the created
        // element is similar to a self-closing element: (<start></start> & <start />)
        self.finish_start_element()?;

        self.writer.write_event(Event::End(BytesEnd::new(tag)))?;
        Ok(())
    }

    /// The given `text` is **unescaped**.
    ///
    /// Finish writing a text element: **`<elem>text</elem>`**
    pub(crate) fn finish_text_element(&mut self, text: &str) -> WriterResult<()> {
        if let Some(element) = self.start_element.take() {
            let text = BytesText::from_escaped(escape(text));
            self.writer.write_event(Event::Start(element.borrow()))?;
            self.writer.write_event(Event::Text(text))?;
            self.writer.write_event(Event::End(element.to_end()))?;
        }
        Ok(())
    }

    /// Finish writing a self-closing element: **`<elem/>`**
    pub(crate) fn finish_empty_element(&mut self) -> WriterResult<()> {
        if let Some(element) = self.start_element.take() {
            self.writer.write_event(Event::Empty(element))?;
        }
        Ok(())
    }
}

fn new_escaped_attribute<'a>(name: &'a str, value: &'a str) -> Attribute<'a> {
    Attribute {
        key: quick_xml::name::QName(name.as_bytes()),
        value: match escape(value.trim()) {
            Cow::Borrowed(borrowed) => Cow::Borrowed(borrowed.as_bytes()),
            Cow::Owned(owned) => Cow::Owned(owned.into_bytes()),
        },
    }
}

fn escape(input: &str) -> Cow<'_, str> {
    macro_rules! escape_chars {
        {$($char:literal => $entity:literal,)+} => {
            const ESCAPE_CHARS: &'static [char] = &[$($char),+];

            /// Only characters contained within [`ESCAPE_CHARS`] must be given.
            fn get_entity(c: char) -> &'static str {
                match c {
                    $($char => $entity,)+
                    _ => unreachable!("only characters in `ESCAPE_CHARS` are matched"),
                }
            }
        };
    }

    escape_chars! {
        '<'  => "&lt;",
        '>'  => "&gt;",
        '"'  => "&quot;",
        '&'  => "&amp;",
        '\'' => "&apos;",
        // Encode whitespace
        '\t' => "&#9;",
        '\n' => "&#10;",
        '\r' => "&#13;",
        '\u{00A0}' => "&#160;", // Non-breaking space (&nbsp;)
    }

    let mut escaped = None;
    let mut last_pos = 0;

    for (i, matched) in input.match_indices(ESCAPE_CHARS) {
        let out = escaped.get_or_insert_with(|| String::with_capacity(input.len() + 16));
        // 'matched' is exactly one char because of the `ESCAPE_CHARS` pattern.
        let c = matched
            .chars()
            .next()
            .expect("Should not be an empty string");

        // Push everything from the last match up to current index
        out.push_str(&input[last_pos..i]);
        out.push_str(get_entity(c));
        last_pos = i + matched.len();
    }

    match escaped {
        None => Cow::Borrowed(input),
        // Finish and return the owned String
        Some(mut s) => {
            s.push_str(&input[last_pos..]);
            Cow::Owned(s)
        }
    }
}

macro_rules! write_element {
    // Empty (self-closing) element
    (writer: $w:expr, tag: $t:expr, $(attributes: $attrs:tt)?) => {
        $crate::writer::xml::write_element!(@helper $w, $t, $($attrs)?)
        .finish_empty_element()
    };
    // Text element
    (writer: $w:expr, tag: $t:expr, text: $text:expr, $(attributes: $attrs:tt)?) => {
        $crate::writer::xml::write_element!(@helper $w, $t, $($attrs)?)
        .finish_text_element($text)
    };
    // Parent element with inner content
    (writer: $w:expr, tag: $t:expr, $(attributes: $attrs:tt)? inner_content: $inner:block) => {{
        let tag = $t;
        $crate::writer::xml::write_element!(@helper $w, tag, $($attrs)?);
        $w.finish_start_element()?;
        $inner
        $w.finish_end_element(tag)
    }};

    //////////////////////////////////
    // HELPERS
    //////////////////////////////////

    (@helper $w:expr, $t:expr, { $($name:path $(where $cond:expr)? => $val:expr,)* ..$iter:expr, }) => {
        write_element!(@helper $w, $t, { $($name $(where $cond)? => $val,)* })
        // Avoid duplicate attributes
        .add_attributes($iter.filter(|(name, _)| match *name {
            $($name)|* => false,
            _ => true,
        }))
    };
    (@helper $w:expr, $t:expr, { $($name:path $(where $cond:expr)? => $val:expr,)* }) => {{
        let mut element = $w.start_element($t)?;
        $(
        $(if $cond)? {
            element = element.add_attribute($name, $val);
        }
        )*
        element
    }};
    // Fallback for when 'attributes' block is missing entirely
    (@helper $w:expr, $t:expr,) => {
        $w.start_element($t)?
    };
}

pub(crate) use write_element;

#[cfg(test)]
mod tests {
    #[test]
    fn test_escape() {
        #[rustfmt::skip]
        let expected = [
            ("&lt;&gt;&apos;&quot;&amp;&#13;&#10;&#9;&#160;", "<>'\"&\r\n\t\u{00A0}"),
            ("abc xyz", "abc xyz"),
            ("1 &lt; 2 &amp; 3", "1 < 2 & 3"),
            ("3 &gt; 1 &amp; 2", "3 > 1 & 2"),
            ("&quot;&apos;quoted&apos;&quot;", "\"'quoted'\""),
            ("line1&#10;line2&#13;&#10;line3", "line1\nline2\r\nline3"),
            ("&#9;&#9;With&#160;non breaking&#160;space", "\t\tWith\u{00A0}non breaking\u{00A0}space"),
            ("esc&lt;aped&amp;attr&gt;ibute&quot;value&apos;", "esc<aped&attr>ibute\"value'"),
       ];

        for (expected_escaped, original) in expected {
            assert_eq!(expected_escaped, super::escape(original));
        }
    }
}
