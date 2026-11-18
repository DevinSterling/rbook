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
    fn starts_with_ignore_ascii_case(&self, start: &str) -> bool;
}

impl StrExt for str {
    fn starts_with_ignore_ascii_case(&self, start: &str) -> bool {
        self.as_bytes()
            .get(..start.len())
            .is_some_and(|prefix| prefix.eq_ignore_ascii_case(start.as_bytes()))
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
    fn starts_with_ascii_ignore_case() {
        #[rustfmt::skip]
        let expected = [
            (true,  "xyz", "XYZ"),
            (true,  "AbCxyz", "aBc"),
            (false, "XyZ", " xYz"),
            (true,  "", ""),
            (false, "", "abc"),
            (false, "aBc", "abCD"),
            (true,  "XⓎz", "xⓎZ"),
            (true,  "å/B/ć/ḋ/ê", "å/b/ć"),
            (false, "Å/ƀ/ć", "å/ƀ/ć"),
            (false, "éabc", "e"),
            (false, "😀abc", "abc"),
            (true,  "日本語", "日本"),
            (false, "日本語", "xyz"),
        ];

        for (expected, a, b) in expected {
            assert_eq!(
                expected,
                a.starts_with_ignore_ascii_case(b),
                "Expected {expected}, a=`{a}`, b=`{b}`",
            );
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
