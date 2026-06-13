//! Content rewriting for XHTML content retrieved from an [`Epub`](crate::Epub).
//!
//! Rewriting is a retrieval-time modification layer and
//! never mutates the underlying [`Epub`](crate::Epub).

use crate::input::IntoOption;
use std::borrow::Cow;

pub(super) mod rewriter;

/// Indicates how to write resource paths within XHTML content.
///
/// # Examples
/// - Rewriting paths as [root relative](Self::root_relative)
///   for an [`EpubReader`](crate::epub::reader::EpubReader):
/// ```
/// # use rbook::Epub;
/// use rbook::epub::rewrite::{EpubRewriteOptions, PathRewrite};
/// # fn main() -> rbook::ebook::errors::EbookResult<()> {
///
/// let epub = Epub::open("tests/ebooks/example_epub")?;
/// let root_relative = PathRewrite::root_relative();
/// let options = EpubRewriteOptions::default()
///     .rewrite_paths(root_relative);
///
/// let mut reader = epub
///     .reader_builder()
///     .rewrite(options)
///     .create();
/// # Ok(())
/// # }
/// ```
///
/// Default: [`Self::None`]
#[non_exhaustive]
#[derive(Clone, Debug, Default, PartialEq)]
pub enum PathRewrite {
    /// Paths remain in their original representation.
    #[default]
    None,
    /// Paths are resolved to root-relative form with a prefix.
    ///
    /// # Appearance
    /// Relative to root-relative path with the prefix set to `example://`:
    /// > `../images/1.png` → `example://opf/data/images/1.png`
    ///
    /// # See Also
    /// - [`Self::root_relative`] for `/`-prefixed paths.
    /// - [`Self::prefix`] to conveniently provide a prefix.
    Prefix(Cow<'static, str>),
}

impl PathRewrite {
    /// Resolves paths to root-relative form.
    ///
    /// # Appearance
    /// Equivalent to [`Self::Prefix`] with the prefix set to `/`:
    /// > `../images/1.png` → `/opf/data/images/1.png`
    pub fn root_relative() -> Self {
        Self::prefix("/")
    }

    /// Resolves paths to root-relative form with the given `prefix`.
    ///
    /// # Appearance
    /// - With the prefix set to `http://localhost:8080/`:
    /// > `../images/1.png` → `http://localhost:8080/opf/data/images/1.png`
    ///
    /// # See Also
    /// - [`Self::root_relative`] for `/`-prefixed paths.
    pub fn prefix(prefix: impl Into<Cow<'static, str>>) -> Self {
        Self::Prefix(prefix.into())
    }

    /// Returns `true` if no path rewriting is set.
    pub fn is_none(&self) -> bool {
        matches!(self, Self::None)
    }

    /// Returns `true` if a prefix is set.
    pub fn is_prefix(&self) -> bool {
        matches!(self, Self::Prefix(_))
    }
}

#[derive(Clone, Debug, Default, PartialEq)]
pub(super) struct EpubRewriteConfig {
    /// See [`EpubRewriteOptions::rewrite_paths`]
    path_rewrite: PathRewrite,
    /// See [`EpubRewriteOptions::inject_css`]
    inject_css: Option<Cow<'static, str>>,
}

impl EpubRewriteConfig {
    pub(super) fn content_requires_modification(&self) -> bool {
        self.path_rewrite.is_prefix() || self.inject_css.is_some()
    }
}

/// Configuration for rewriting XHTML content retrieved from an [`Epub`](crate::Epub).
///
/// # Options
/// - [`rewrite_paths`](Self::rewrite_paths) (Default: [`PathRewrite::None`])
/// - [`inject_css`](Self::inject_css) (Default: [`None`])
///
/// # See Also
/// - [`EpubReaderOptions::rewrite`](crate::epub::reader::EpubReaderOptions::rewrite)
///   for reader-wide rewriting.
/// - [`Epub::read_resource_str_with`](crate::epub::Epub::read_resource_str_with)
///   for resource-scoped rewriting.
/// - [`EpubManifestEntry::read_str_with`](crate::epub::manifest::EpubManifestEntry::read_str_with)
///   for manifest entry-scoped rewriting.
///
/// # Examples
/// - Reading content with rewrite options:
/// ```
/// # use rbook::Epub;
/// use rbook::epub::rewrite::{EpubRewriteOptions, PathRewrite};
///
/// # fn main() -> rbook::ebook::errors::EbookResult<()> {
/// let epub = Epub::open("tests/ebooks/example_epub")?;
/// let rewrite = EpubRewriteOptions::new()
///     .rewrite_paths(PathRewrite::prefix("ebook://"))
///     .inject_css("body { color: blue; font-family: sans-serif; }");
///
/// let manifest_entry = epub.manifest().by_id("toc").unwrap();
/// let xhtml = manifest_entry.read_str_with(&rewrite)?;
///
/// assert!(xhtml.contains("ebook://"));
/// assert!(xhtml.contains("body { color: blue; font-family: sans-serif; }"));
/// # Ok(())
/// # }
/// ```
#[derive(Clone, Debug, Default, PartialEq)]
pub struct EpubRewriteOptions(pub(super) EpubRewriteConfig);

impl EpubRewriteOptions {
    /// Creates a new builder with default values.
    pub fn new() -> Self {
        Self::default()
    }

    /// Injects the given `css` as-is into XHTML content.
    ///
    /// Passing [`None`] clears the injected CSS.
    ///
    /// # Note
    /// - The CSS is injected as the last `style` element within the `head` section.
    ///
    /// # Examples
    /// - Injecting CSS to override anchor colors for an [`EpubReader`](crate::epub::reader::EpubReader):
    /// ```
    /// # use rbook::Epub;
    /// # use rbook::epub::rewrite::EpubRewriteOptions;
    /// # fn main() -> rbook::ebook::errors::EbookResult<()> {
    /// let epub = Epub::open("tests/ebooks/example_epub")?;
    /// let rewrite = EpubRewriteOptions::default()
    ///     .inject_css("ol > a[href$='.xhtml'] { color: red; }");
    ///
    /// let mut reader = epub
    ///     .reader_builder()
    ///     .rewrite(rewrite)
    ///     .create();
    /// # Ok(())
    /// # }
    /// ```
    pub fn inject_css(mut self, css: impl IntoOption<Cow<'static, str>>) -> Self {
        self.0.inject_css = css.into_option();
        self
    }

    /// Rewrites resource paths (e.g., `src`, `href`) found within XHTML content.
    ///
    /// Useful for rendering pipelines that require absolute or prefixed resource paths
    /// (`../img1.png` -> `ebook://OEBPS/data/img1.png`).
    ///
    /// Default: [`PathRewrite::None`]
    pub fn rewrite_paths(mut self, rewrite: PathRewrite) -> Self {
        self.0.path_rewrite = rewrite;
        self
    }
}
