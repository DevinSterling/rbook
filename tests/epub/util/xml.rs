use quick_xml::Reader;
use quick_xml::events::Event;

pub fn extract_attributes(xml: &str, keys: &[&str]) -> impl Iterator<Item = String> {
    let mut reader = Reader::from_str(xml);

    std::iter::from_fn(move || {
        loop {
            match reader.read_event() {
                Ok(Event::Eof) => break,
                Ok(Event::Start(el) | Event::Empty(el)) => {
                    for attr in el.attributes() {
                        let attr = match attr {
                            Ok(attr) => attr,
                            Err(e) => {
                                panic!("Attribute error at {}: {e:?}", reader.buffer_position())
                            }
                        };

                        if keys.contains(&attr.key.0) {
                            return Some(attr.value.into_owned());
                        }
                    }
                }
                Err(e) => panic!("Unexpected error at {}: {e:?}", reader.error_position()),
                _ => {}
            }
        }
        None
    })
}
