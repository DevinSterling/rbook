use crate::ebook::errors::FormatError::Unparsable;
use crate::ebook::errors::{EbookError, EbookResult};
use crate::epub::rewrite::{EpubRewriteConfig, PathRewrite};
use crate::util::uri::{self, UriResolver};
use quick_xml::events::attributes::Attribute;
use quick_xml::events::{BytesEnd, BytesStart, Event};
use quick_xml::{Reader, Writer};
use std::borrow::Cow;
use std::error::Error;
use std::io;
use std::io::Write;

pub(crate) struct ContentRewriter<'ebook> {
    config: &'ebook EpubRewriteConfig,
    resolver: UriResolver<'ebook>,
    writer: Writer<Vec<u8>>,
}

impl<'ebook> ContentRewriter<'ebook> {
    pub(crate) fn new(
        config: &'ebook EpubRewriteConfig,
        content_file_location: &'ebook str,
    ) -> Self {
        Self {
            writer: Writer::new(Vec::new()),
            resolver: UriResolver::parent_of(content_file_location),
            config,
        }
    }

    pub(crate) fn rewrite(mut self, original: &[u8]) -> EbookResult<String> {
        let mut reader = Reader::from_reader(original);

        loop {
            let event = reader
                .read_event()
                .map_err(|err| EbookError::Format(Unparsable(Box::new(err))))?;

            match event {
                Event::Eof => break,
                Event::Start(mut start) => {
                    // Provide and retrieve the modified start element
                    start = self.rewrite_start_element(start)?;
                    self.writer
                        .write_event(Event::Start(start))
                        .map_err(to_ebook_error)?;
                }
                Event::Empty(mut start) => {
                    start = self.rewrite_start_element(start)?;
                    self.writer
                        .write_event(Event::Empty(start))
                        .map_err(to_ebook_error)?;
                }
                Event::End(ref end) if self.check_inject_stylesheet(end) => {
                    self.inject_css().map_err(to_ebook_error)?;
                    self.writer.write_event(event).map_err(to_ebook_error)?;
                }
                event => self.writer.write_event(event).map_err(to_ebook_error)?,
            }
        }

        // This should always work since quick-xml should generate valid UTF-8 XML
        String::from_utf8(self.writer.into_inner()).map_err(to_ebook_error)
    }

    fn check_inject_stylesheet(&self, end: &BytesEnd) -> bool {
        self.config.inject_css.is_some() && end.name().0 == b"head"
    }

    fn rewrite_start_element<'a>(&mut self, start: BytesStart<'a>) -> EbookResult<BytesStart<'a>> {
        match start.name().0 {
            _ if self.config.path_rewrite.is_prefix() => self.check_rewrite_path(start),
            _ => Ok(start),
        }
    }

    fn check_rewrite_path<'a>(&mut self, start: BytesStart<'a>) -> EbookResult<BytesStart<'a>> {
        match start.name().0 {
            b"object" => self.rewrite_path(start, |a| matches!(a.key.0, b"data")),
            b"source" => self.rewrite_path(start, |a| matches!(a.key.0, b"src" | b"srcset")),
            b"link" => self.rewrite_path(start, |a| matches!(a.key.0, b"href")),
            b"image" | b"use" => {
                self.rewrite_path(start, |a| matches!(a.key.0, b"href" | b"xlink:href"))
            }
            b"iframe" | b"script" | b"img" | b"video" | b"audio" | b"track" | b"input" => {
                self.rewrite_path(start, |a| matches!(a.key.0, b"src"))
            }
            b"a" => self.rewrite_path(start, |a| {
                // Special case: do not match against if an anchor/fragment is present
                matches!(a.key.0, b"href") && !a.value.starts_with(b"#")
            }),
            _ => Ok(start),
        }
    }

    fn rewrite_path<'a>(
        &mut self,
        start: BytesStart<'a>,
        matcher: impl Fn(&Attribute) -> bool,
    ) -> EbookResult<BytesStart<'a>> {
        let mut el = start.clone();
        el.clear_attributes();

        for attribute_result in start.attributes() {
            let mut attribute = attribute_result.map_err(to_ebook_error)?;

            if matcher(&attribute) && !uri::has_scheme_bytes(&attribute.value) {
                let path = self.rewrite_path_data(&attribute)?;
                attribute.value = Cow::Borrowed(path.as_bytes());
                el.push_attribute(attribute);
            } else {
                el.push_attribute(attribute);
            }
        }

        Ok(el)
    }

    fn rewrite_path_data(&self, attribute: &Attribute) -> EbookResult<String> {
        let relative = str::from_utf8(&attribute.value).map_err(to_ebook_error)?;
        let mut path = self.resolver.resolve(relative);

        if let PathRewrite::Prefix(prefix) = &self.config.path_rewrite
            && prefix != "/"
        {
            // For greater flexibility, remove the leading slash given by the `resolve` method
            path.replace_range(..1, prefix);
        }
        Ok(path)
    }

    fn inject_css(&mut self) -> io::Result<()> {
        if let Some(css) = &self.config.inject_css {
            // Wrap with CDATA to ensure any injected CSS is parsed correctly on all platforms
            let buffer = self.writer.get_mut();
            buffer.write_all(b"<style>/*<![CDATA[*/")?;
            buffer.write_all(css.as_bytes())?;
            buffer.write_all(b"/*]]>*/</style>")?;
        }
        Ok(())
    }
}

fn to_ebook_error(err: impl Error + Send + Sync + 'static) -> EbookError {
    EbookError::Format(Unparsable(Box::from(err)))
}
