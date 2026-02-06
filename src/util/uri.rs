use crate::util::borrow::CowExt;
use std::borrow::Cow;

pub const SEPARATOR: char = '/';
const SEPARATOR_STR: &str = "/";
const CURRENT_DIR: &str = ".";
const PARENT_DIR: &str = "..";
const EMPTY: &str = "";

/// Resolver to turn relative uris into absolute.
#[derive(Copy, Clone, Debug, PartialEq)]
pub(crate) struct UriResolver<'a>(
    /// The absolute path where relative paths are made absolute from.
    &'a str,
);

impl<'a> UriResolver<'a> {
    pub(crate) fn parent_of(absolute_path: &'a str) -> Self {
        Self(parent(absolute_path))
    }

    pub(crate) fn resolve(&self, href: &str) -> String {
        resolve(self.0, href).into_owned()
    }

    #[cfg(feature = "write")]
    pub(crate) fn relativize<'b>(&self, href: &'b str) -> Cow<'b, str> {
        relativize(self.0, href)
    }
}

pub(crate) fn into_absolute(mut path: String) -> String {
    if !path.starts_with(SEPARATOR) {
        path.insert(0, SEPARATOR);
    }
    path
}

pub(crate) fn parent(href: &str) -> &str {
    href.rfind(SEPARATOR).map_or(EMPTY, |index| {
        if index == 0 {
            SEPARATOR_STR
        } else {
            &href[..index]
        }
    })
}

pub(crate) fn path(href: &str) -> &str {
    href.find(['#', '?']).map_or(href, |index| &href[..index])
}

pub(crate) fn filename(href: &str) -> &str {
    path(href)
        .rsplit(SEPARATOR)
        .next()
        .expect("`rsplit` guarantees at least one entry")
}

pub(crate) fn file_extension(href: &str) -> Option<&str> {
    filename(href).rsplit_once('.').map(|(_, ext)| ext)
}

// This given href is assumed to be well-formed.
pub(crate) fn has_scheme(href: &str) -> bool {
    // The scheme must be ASCII
    let ascii = href.as_bytes();

    // Check if a colon exists
    let Some(colon_pos) = ascii.iter().position(|&c| c == b':') else {
        return false;
    };

    // The first byte must be an ASCII letter
    if !ascii[0].is_ascii_alphabetic() {
        return false;
    }

    ascii[1..colon_pos]
        .iter()
        // Return early if invalid characters are encountered
        .all(|c| c.is_ascii_alphanumeric() || matches!(*c, b'+' | b'.' | b'-'))
}

pub(crate) fn decode(encoded: &str) -> Cow<'_, str> {
    percent_encoding::percent_decode_str(encoded).decode_utf8_lossy()
}

pub(crate) fn encode(original: &str) -> Cow<'_, str> {
    const ASCII_SET: &percent_encoding::AsciiSet = &percent_encoding::NON_ALPHANUMERIC
        .remove(b'%') // Prevent double-encoding
        .remove(b'.') // File extensions
        .remove(b'/') // Directory separator
        .remove(b':') // Schemes
        .remove(b'#') // Fragments (Toc entries)
        .remove(b'?') // Query strings (Toc entries, although rare...)
        .remove(b'-')
        .remove(b'_')
        .remove(b'~')
        .remove(b'=')
        .remove(b'&');

    percent_encoding::percent_encode(original.as_bytes(), ASCII_SET).into()
}

/// Resolve a child path against its parent, normalizing if necessary.
pub(crate) fn resolve<'a>(parent_dir: &str, relative: &'a str) -> Cow<'a, str> {
    let (main_href, ext) = relative
        .find(['?', '#'])
        .map_or((relative, EMPTY), |position| {
            (&relative[..position], &relative[position..])
        });

    if main_href.starts_with(SEPARATOR) || has_scheme(main_href) {
        // If the path is absolute or has a scheme,
        // it is most likely resolved already.
        return Cow::Borrowed(relative);
    }

    let resolved_href = String::from(parent_dir) + SEPARATOR_STR + main_href + ext;

    Cow::Owned(
        normalize(&resolved_href)
            .take_owned()
            .unwrap_or(resolved_href),
    )
}

pub(crate) fn normalize(original: &str) -> Cow<'_, str> {
    // First check if normalization is required
    let mut components = original.split(SEPARATOR);
    // If absolute (`/a/b/c`), the first split is always empty.
    if original.starts_with(SEPARATOR) {
        components.next();
    }
    // Normalization is not required if the following are not found:
    // "."  => Current dir
    // ".." => Parent dir
    // ""   => Empty component (e.g., double slashes)
    if !components.any(|c| matches!(c, EMPTY | CURRENT_DIR | PARENT_DIR)) {
        return Cow::Borrowed(original);
    }

    // Normalize
    let mut stack = Vec::new();

    for component in original.split(SEPARATOR) {
        match component {
            EMPTY | CURRENT_DIR => {}
            PARENT_DIR => {
                stack.pop();
            }
            _ => stack.push(component),
        }
    }

    // Calculate `capacity` to avoid reallocations when appending to `path`
    let capacity = stack.iter().map(|s| s.len()).sum::<usize>() + stack.len();
    let mut path = String::with_capacity(capacity);
    let mut components = stack.into_iter();

    // Re-add the root directory if there was one originally
    if original.starts_with(SEPARATOR) {
        path.push(SEPARATOR);
    }
    if let Some(component) = components.next() {
        path.push_str(component);
    }
    for component in components {
        path.push(SEPARATOR);
        path.push_str(component);
    }
    Cow::Owned(path)
}

/// The given `base` and `path` arguments must be absolute.
#[cfg(feature = "write")]
fn relativize<'a>(base: &str, path: &'a str) -> Cow<'a, str> {
    // First check if allocation is required
    if let Some(relative) = path.strip_prefix(base) {
        // Ensure `relative` is not fragmented
        // (e.g., base="img" and path="img2" are different)
        if relative.starts_with(SEPARATOR) {
            return Cow::Borrowed(&relative[1..]); // Strip leading slash
        } else if relative.is_empty() {
            return Cow::Borrowed(CURRENT_DIR); // Identical paths
        }
    }

    // Splits and skips certain components:
    // "."  => Current dir
    // ""   => Empty component (e.g., double slashes)
    fn split_components(s: &str) -> impl Iterator<Item = &str> {
        s.split(SEPARATOR)
            .filter(|&c| !matches!(c, EMPTY | CURRENT_DIR))
    }
    let mut base_it = split_components(base);
    let mut path_it = split_components(path);
    let mut stack = Vec::new();

    // Implementation based on:
    // https://github.com/rust-lang/rust/blob/e1d0de82cc40b666b88d4a6d2c9dcbc81d7ed27f/src/librustc_back/rpath.rs#L116-L158
    loop {
        match (base_it.next(), path_it.next()) {
            (None, None) => break,
            (None, Some(component)) => {
                stack.push(component);
                stack.extend(path_it);
                break;
            }
            (_, None) => stack.push(PARENT_DIR),
            // Continue; iterate to where there is no common prefix
            (Some(base_c), Some(path_c)) if stack.is_empty() && base_c == path_c => (),
            (Some(_), Some(component)) => {
                stack.push(PARENT_DIR);
                stack.extend(base_it.map(|_| PARENT_DIR));
                stack.push(component);
                stack.extend(path_it);
                break;
            }
        }
    }
    Cow::Owned(stack.join(SEPARATOR_STR))
}

#[cfg(feature = "write")]
pub(crate) fn join(left: &str, right: &str) -> String {
    let mut joined = String::with_capacity(left.len() + right.len() + 1);
    joined.push_str(left.trim_end_matches(SEPARATOR));
    joined.push(SEPARATOR);
    joined.push_str(right.trim_start_matches(SEPARATOR));
    joined
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parent_href() {
        #[rustfmt::skip]
        let expected = [
            ("OPS/content/toc", "OPS/content/toc/toc.xhtml?q=1#start"),
            ("OPS/content", "OPS/content/toc"),
            ("OPS/content", "OPS/content/c1.xhtml"),
            ("OPS", "OPS/c5.xhtml?q=1"),
            ("", "OPS"),
            ("/", "/OPS"),
            ("/", "/"),
            ("", ""),
        ];

        for (expect_href, href) in expected {
            assert_eq!(expect_href, parent(href));
        }
    }

    #[test]
    fn test_as_absolute_href() {
        #[rustfmt::skip]
        let expected = [
            ("/c3.xhtml", "OPS/content", "/c3.xhtml"),
            ("content/c3.xhtml", "./content", "c3.xhtml"),
            ("OPS/content/toc/toc.xhtml", "OPS/content/toc", "toc.xhtml"),
            ("OPS/content/toc/toc.xhtml", "OPS/content/toc", "./toc.xhtml",),
            ("OPS/content/toc/toc.xhtml", "OPS/content/toc", "./././././////./toc.xhtml",),
            ("OPS/content/c1.xhtml", "OPS/content/toc", "../c1.xhtml"),
            ("OPS/c1.xhtml?q=1", "OPS/content/toc", "../../c1.xhtml?q=1"),
            ("c1.xhtml#part-2", "OPS/content/toc", "../../../c1.xhtml#part-2"),
            ("c1.xhtml?q=1#part-1", "OPS/content/toc", "../../../../c1.xhtml?q=1#part-1"),
            ("OPS/a/toc.ncx", "OPS/a/b/c/d/e", "../../../../toc.ncx"),
        ];

        for (expect_href, absolute_dir, relative_href) in expected {
            assert_eq!(expect_href, resolve(absolute_dir, relative_href));
        }
    }

    #[cfg(feature = "write")]
    #[test]
    fn test_diff_href() {
        #[rustfmt::skip]
        let expected = [
            ("images/1.png", "/OEBPS", "/OEBPS/images/1.png", ),
            ("../chapters/c1.xhtml", "/data/content", "/data/chapters/c1.xhtml"),
            ("../images_other/1.png", "/images", "/images_other/1.png"),
            ("b/c/d.png", "/a", "/a/b/c/d.png"),
            ("../../../../a.png#part-2", "/a/b/c/d", "/a.png#part-2"),
            ("../../e/z.png?q=1#part-1", "/a/b/c", "/a/e/z.png?q=1#part-1"),
        ];

        for (expect_href, absolute_dir, other_dir) in expected {
            assert_eq!(expect_href, relativize(absolute_dir, other_dir));
        }
    }

    #[cfg(feature = "write")]
    #[test]
    fn test_join() {
        #[rustfmt::skip]
        let expected = [
            ("/path/to/file", "/", "path/to/file", ),
            ("data/content/c1.xhtml", "data/content", "c1.xhtml"),
            ("/images/1.png", "/images/", "/1.png"),
            ("/a/a/b/c/d.png", "/a", "/a/b/c/d.png"),
            ("////.//a//b/./../../a.png#part-2", "////.//a//b/.//", "/../../a.png#part-2"),
            ("/a/b/c/a/e/z.png?q=1#part-1", "/a/b/c", "/a/e/z.png?q=1#part-1"),
        ];

        for (expect_join, left, right) in expected {
            assert_eq!(expect_join, join(left, right));
        }
    }

    #[test]
    fn test_has_scheme() {
        assert!(has_scheme("https://ab.c"));
        assert!(has_scheme("mailto:a@b.c"));
        assert!(has_scheme("a:link"));
        assert!(has_scheme("x.y.z+a+b+c-1-2-3:123"));
        assert!(!has_scheme("1https://ab.c"));
        assert!(!has_scheme(":abc"));
        assert!(!has_scheme(":"));
        assert!(!has_scheme(""));
        assert!(!has_scheme("not a scheme:..."));
    }
}
