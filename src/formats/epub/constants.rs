// Paths
pub(crate) const META_INF: &str = "META-INF"; // Used to identify the root `.opf` file
pub(crate) const CONTAINER: &str = "META-INF/container.xml"; // Used to identify container

// Elements
pub(crate) const PACKAGE: &str = "package";
pub(crate) const SPINE: &str = "spine";

// Metadata elements
pub(crate) const TITLE: &str = "title"; // Also used for Guide title attribute
pub(crate) const LANGUAGE: &str = "language";
pub(crate) const IDENTIFIER: &str = "identifier";
pub(crate) const MODIFIED: &str = "modified";
pub(crate) const CREATOR: &str = "creator";
pub(crate) const CONTRIBUTOR: &str = "contributor";
pub(crate) const DATE: &str = "date";
pub(crate) const DESCRIPTION: &str = "description";
pub(crate) const PUBLISHER: &str = "publisher";
pub(crate) const SUBJECT: &str = "subject";
pub(crate) const TYPE: &str = "type"; // Also used for Guide type attribute
pub(crate) const COVER: &str = "cover";

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

// Spine attributes
pub(crate) const IDREF: &str = "idref";

// Toc attributes
pub(crate) const TOC_TYPE: &str = "epub:type";
pub(crate) const TOC: &str = "toc";
pub(crate) const LANDMARKS: &str = "landmarks";
pub(crate) const PAGE_LIST2: &str = "pageList"; // epub2
pub(crate) const PAGE_LIST3: &str = "page-list"; // epub3
pub(crate) const PLAY_ORDER: &str = "playOrder"; // epub2 only

// Properties
pub(crate) const PROPERTIES: &str = "properties";
pub(crate) const COVER_PROPERTY: &str = "cover-image";
pub(crate) const NAV_PROPERTY: &str = "nav";

// Media types
pub(crate) const MEDIA_TYPE: &str = "media-type";
pub(crate) const PACKAGE_TYPE: &str = "application/oebps-package+xml";
pub(crate) const NCX_TYPE: &str = "application/x-dtbncx+xml";

// rbook specific
// Used to indicate and differentiate between non-legacy and legacy
// features if not possible otherwise.
pub(crate) const LEGACY_FEATURE: &str = "_epub_legacy_feature";
pub(crate) const LEGACY_META: &str = "OPF2 meta";
