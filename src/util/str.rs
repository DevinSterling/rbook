pub(crate) trait StringExt {
    fn trim_in_place(&mut self);
}

impl StringExt for String {
    fn trim_in_place(&mut self) {
        self.truncate(self.trim_end().len());

        let start = self.len() - self.trim_start().len();
        if start > 0 {
            self.drain(..start);
        }
    }
}

pub(crate) trait StrExt {
    fn starts_with_ignore_case(&self, start: &str) -> bool;
}

impl StrExt for str {
    fn starts_with_ignore_case(&self, start: &str) -> bool {
        self.len() >= start.len() && self[..start.len()].eq_ignore_ascii_case(start)
    }
}

#[cfg(feature = "write")]
pub(crate) fn prefix(prefix: &str, main: &str) -> String {
    let mut string = String::with_capacity(prefix.len() + main.len());
    string.push_str(prefix);
    string.push_str(main);
    string
}

#[cfg(feature = "write")]
pub(crate) fn suffix(suffix: &str, main: &str) -> String {
    let mut string = String::with_capacity(main.len() + suffix.len());
    string.push_str(main);
    string.push_str(suffix);
    string
}

#[cfg(feature = "write")]
pub(crate) fn slugify(input: &str) -> String {
    const LINES: &[char] = &['-', '_'];

    fn check_push(slug: &mut String, c: char) {
        // Avoid consecutive dashes
        if (c == '-' || c == '_') && (slug.is_empty() || slug.ends_with(LINES)) {
            return;
        }
        slug.push(c);
    }

    let mut slug = String::with_capacity(input.len());

    for char in input.chars() {
        let char = char.to_ascii_lowercase();

        match char {
            'a'..='z' | '0'..='9' | '-' => check_push(&mut slug, char),
            _ => check_push(&mut slug, '-'),
        }
    }

    if slug.ends_with(LINES) {
        slug.pop();
    }

    slug.shrink_to_fit();
    slug
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_trim_in_place() {
        #[rustfmt::skip]
        let expected = [
            ("a   b   c", "\n \r\t \n  a   b   c \r  \n\n\t"),
            ("", "  \r\n\t  \r \n"),
            ("", ""),
            ("%123", "%123"),
            ("abc", "abc "),
            ("xyz", "\txyz"),
        ];

        for (expected, original) in expected {
            let mut owned = original.to_owned();
            owned.trim_in_place();

            assert_eq!(expected, owned);
        }
    }

    #[test]
    #[cfg(feature = "write")]
    fn test_prefix() {
        #[rustfmt::skip]
        let expected = [
            ("#1", "#", "1"),
            ("prefixed", "prefix", "ed"),
            ("  a", "  ", "a"),
            ("", "", ""),
            ("/", "", "/"),
        ];

        for (expected, a, b) in expected {
            assert_eq!(expected, prefix(a, b));
        }
    }

    #[test]
    #[cfg(feature = "write")]
    fn test_suffix() {
        #[rustfmt::skip]
        let expected = [
            ("#1", "1", "#"),
            ("suffixed", "ed", "suffix"),
            ("file.xhtml", ".xhtml", "file"),
            ("", "", ""),
            ("/", "", "/"),
        ];

        for (expected, a, b) in expected {
            assert_eq!(expected, suffix(a, b));
        }
    }

    #[test]
    #[cfg(feature = "write")]
    fn test_slugify() {
        #[rustfmt::skip]
        let expected = [
            ("chapter-1-an-introduction", "Chapter #1: An Introduction?"),
            ("re-do-part-1", "_Re/do_-_-_-_Part_-_1-"),
            ("hi", "______---______-_-____-_hi___--__-_"),
            ("images-art1-png", "images/art1.png"),
        ];

        for (expected, original) in expected {
            assert_eq!(expected, slugify(original));
        }
    }
}
