// General
pub(crate) const ID: &str = "id";
pub(crate) const HREF: &str = "href";
pub(crate) const SRC: &str = "src";
pub(crate) const LANG: &str = "xml:lang";
pub(crate) const DIR: &str = "dir";

// Paths
pub(crate) const CONTAINER: &str = "META-INF/container.xml"; // Used to identify container

// Elements
pub(crate) const ROOT_FILE: &str = "rootfile";
pub(crate) const PACKAGE: &str = "package";
pub(crate) const MANIFEST: &str = "manifest";
pub(crate) const SPINE: &str = "spine";

// Metadata elements
pub(crate) const TITLE: &str = "dc:title";
pub(crate) const LANGUAGE: &str = "dc:language";
pub(crate) const IDENTIFIER: &str = "dc:identifier";
pub(crate) const MODIFIED: &str = "dcterms:modified";
pub(crate) const CREATOR: &str = "dc:creator";
pub(crate) const CONTRIBUTOR: &str = "dc:contributor";
pub(crate) const DATE: &str = "dc:date";
pub(crate) const DESCRIPTION: &str = "dc:description";
pub(crate) const PUBLISHER: &str = "dc:publisher";
pub(crate) const SUBJECT: &str = "dc:subject";
pub(crate) const DC_NAMESPACE: &str = "dc";
pub(crate) const META: &str = "meta";

// Meta attribute keys
pub(crate) const TITLE_TYPE: &str = "title-type";
pub(crate) const IDENTIFIER_TYPE: &str = "identifier-type";
pub(crate) const COVER: &str = "cover";
pub(crate) const FILE_AS: &str = "file-as";
pub(crate) const ROLE: &str = "role";
pub(crate) const AUTHORITY: &str = "authority";
pub(crate) const TERM: &str = "term";
pub(crate) const ALTERNATE_SCRIPT: &str = "alternate-script";
pub(crate) const DISPLAY_SEQ: &str = "display-seq";
pub(crate) const SCHEME: &str = "scheme";

// Legacy Meta attribute keys
pub(crate) const OPF_ROLE: &str = "opf:role";
pub(crate) const OPF_FILE_AS: &str = "opf:file-as";
pub(crate) const OPF_SCHEME: &str = "opf:scheme";
pub(crate) const OPF_AUTHORITY: &str = "opf:authority";
pub(crate) const OPF_TERM: &str = "opf:term";
pub(crate) const OPF_ALT_REP: &str = "opf:alt-rep";
pub(crate) const OPF_ALT_REP_LANG: &str = "opf:alt-rep-lang";

// Meta attribute value
pub(crate) const MAIN_TITLE_TYPE: &str = "main";

// Container attributes
pub(crate) const FULL_PATH: &str = "full-path";

// Package attributes
pub(crate) const VERSION: &str = "version";
pub(crate) const UNIQUE_ID: &str = "unique-identifier";

// Metadata attributes
pub(crate) const PROPERTY: &str = "property";
pub(crate) const NAME: &str = "name";
pub(crate) const CONTENT: &str = "content";
pub(crate) const REFINES: &str = "refines";

// Manifest attributes
pub(crate) const FALLBACK: &str = "fallback";
pub(crate) const MEDIA_OVERLAY: &str = "media-overlay";

// Manifest item properties
pub(crate) const PROPERTIES: &str = "properties";
pub(crate) const COVER_IMAGE: &str = "cover-image";
pub(crate) const NAV_PROPERTY: &str = "nav";

// Spine attributes
pub(crate) const IDREF: &str = "idref";
pub(crate) const LINEAR: &str = "linear";
pub(crate) const PAGE_PROGRESSION_DIRECTION: &str = "page-progression-direction";

// Guide attributes
pub(crate) const GUIDE_TITLE: &str = "title";
pub(crate) const GUIDE_TYPE: &str = "type";

// Toc attributes
pub(crate) const EPUB_TYPE: &str = "epub:type";
pub(crate) const PLAY_ORDER: &str = "playOrder"; // epub2 only

// Media types
pub(crate) const MEDIA_TYPE: &str = "media-type";
pub(crate) const PACKAGE_TYPE: &str = "application/oebps-package+xml";
pub(crate) const NCX_TYPE: &str = "application/x-dtbncx+xml";

// constants where calling str.as_bytes() is not possible
pub(crate) mod bytes {
    pub(crate) const PACKAGE: &[u8] = super::PACKAGE.as_bytes();
    pub(crate) const METADATA: &[u8] = b"metadata";
    pub(crate) const MANIFEST: &[u8] = super::MANIFEST.as_bytes();
    pub(crate) const SPINE: &[u8] = super::SPINE.as_bytes();
    pub(crate) const GUIDE: &[u8] = b"guide";

    pub(crate) const ITEM: &[u8] = b"item";
    pub(crate) const ITEMREF: &[u8] = b"itemref";
    pub(crate) const REFERENCE: &[u8] = b"reference";

    pub(crate) const NAV: &[u8] = b"nav";
    pub(crate) const NAV_MAP: &[u8] = b"navMap"; // NCX
    pub(crate) const PAGE_LIST: &[u8] = b"pageList";

    pub(crate) const LIST_ITEM: &[u8] = b"li";
    pub(crate) const NAV_POINT: &[u8] = b"navPoint"; // NCX
    pub(crate) const PAGE_TARGET: &[u8] = b"pageTarget";

    pub(crate) const NAV_LABEL: &[u8] = b"navLabel"; // NCX
    pub(crate) const NAV_CONTENT: &[u8] = b"content"; // NCX

    pub(crate) const ANCHOR: &[u8] = b"a";
}
