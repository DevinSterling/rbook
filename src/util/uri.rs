use std::borrow::Cow;
use std::path::{Component, Path, PathBuf};

pub(crate) fn parent(href: &str) -> &str {
    href.rfind('/')
        .map_or("", |index| if index == 0 { "/" } else { &href[..index] })
}

pub(crate) fn decode(encoded: &str) -> Cow<str> {
    percent_encoding::percent_decode_str(encoded).decode_utf8_lossy()
}

pub(crate) fn normalize(href: &str) -> String {
    let mut buf = PathBuf::from(href);
    normalize_href_path(&mut buf);

    // 1: `buf` is UTF-8 as its data derives from `href`.
    // 2: Ensure separators are forward slashes.
    buf.to_string_lossy().replace('\\', "/")
}

/// Resolve a child path against its parent, normalizing if necessary.
pub(crate) fn resolve<'a>(parent_dir: &str, relative: &'a str) -> Cow<'a, str> {
    let (main_href, frag) = relative
        .find(['?', '#'])
        .map(|position| (&relative[..position], &relative[position..]))
        .unwrap_or((relative, ""));

    if main_href.starts_with('/') || has_scheme(main_href) {
        // If the path is absolute or has a scheme,
        // it is most likely resolved already.
        return Cow::Borrowed(relative);
    }

    let mut buf = Path::new(parent_dir).join(main_href);
    normalize_href_path(&mut buf);

    // 1: `buf` is UTF-8 as its data derives from `absolute_dir` and `href`.
    // 2: Ensure separators are forward slashes.
    Cow::Owned(buf.to_string_lossy().replace('\\', "/") + frag)
}

fn normalize_href_path(original: &mut PathBuf) {
    let mut stack = Vec::new();

    for component in original.components() {
        match component {
            Component::ParentDir => {
                if stack
                    .last()
                    // If the component is the root, disallow popping.
                    // No content must come before the root when present.
                    .is_some_and(|component| !matches!(component, Component::RootDir))
                {
                    stack.pop();
                }
            }
            Component::CurDir => {}
            _ => {
                stack.push(component);
            }
        }
    }

    // Most if not all, hrefs are not normalized
    *original = PathBuf::from_iter(stack);
}

/// The provided `href` must not contain a `fragment`
/// and `query` when passed to this method.
fn has_scheme(href: &str) -> bool {
    // Check if a scheme is provided
    href.contains(':')
}

#[cfg(test)]
mod tests {
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
            assert_eq!(expect_href, super::parent(href));
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
            assert_eq!(expect_href, super::resolve(absolute_dir, relative_href));
        }
    }
}
