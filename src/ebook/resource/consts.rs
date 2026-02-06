pub(crate) mod mime {
    #[cfg(feature = "write")]
    pub(crate) use write::*;

    pub(crate) const XHTML: &str = "application/xhtml+xml";
    pub(crate) const OEBPS_PACKAGE: &str = "application/oebps-package+xml";
    pub(crate) const HTML: &str = "text/html";
    pub(crate) const JAVASCRIPT: &str = "application/javascript";
    pub(crate) const ECMASCRIPT: &str = "application/ecmascript";
    pub(crate) const JAVASCRIPT_TEXT: &str = "text/javascript";
    pub(crate) const CSS: &str = "text/css";

    // constants where calling str.as_bytes() is not possible
    pub(crate) mod bytes {
        pub(crate) const OEBPS_PACKAGE: &[u8] = super::OEBPS_PACKAGE.as_bytes();
    }

    #[cfg(feature = "write")]
    mod write {
        pub(crate) const NCX: &str = "application/x-dtbncx+xml";
    }
}
