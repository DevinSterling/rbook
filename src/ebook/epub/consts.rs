// Shared general constants //
// Elements
const _HEAD: &str = "head";
const _META: &str = "meta";
const _LINK: &str = "link";
// Attribute keys
const _VERSION: &str = "version";
const _NAME: &str = "name";
const _CONTENT: &str = "content";
const _MEDIA_TYPE: &str = "media-type";
const _HREF: &str = "href";
const _REL: &str = "rel";
// General
const _TITLE: &str = "title";

pub(crate) mod xml {
    #[cfg(feature = "write")]
    pub(crate) use write::*;

    pub(crate) const ID: &str = "id";
    pub(crate) const LANG: &str = "xml:lang";

    #[cfg(feature = "write")]
    pub(crate) mod write {
        pub(crate) const XMLNS: &str = "xmlns";
    }
}

pub(crate) mod epub {
    #[cfg(feature = "write")]
    pub(crate) use write::*;

    // Attribute keys
    pub(crate) const TYPE: &str = "epub:type";

    #[cfg(feature = "write")]
    mod write {
        pub(crate) const XMLNS: &str = "xmlns:epub";
        pub(crate) const EPUB_NS: &str = "http://www.idpf.org/2007/ops";

        // Attribute values
        /// Value of [`TYPE`] attribute.
        pub(crate) const TITLE: &str = super::super::_TITLE;
    }
}

pub(crate) mod dc {
    #[cfg(feature = "write")]
    pub(crate) use write::*;

    // Elements
    pub(crate) const PREFIX: &str = "dc";
    pub(crate) const TITLE: &str = "dc:title";
    pub(crate) const LANGUAGE: &str = "dc:language";
    pub(crate) const IDENTIFIER: &str = "dc:identifier";
    pub(crate) const CREATOR: &str = "dc:creator";
    pub(crate) const CONTRIBUTOR: &str = "dc:contributor";
    pub(crate) const DATE: &str = "dc:date";
    pub(crate) const DESCRIPTION: &str = "dc:description";
    pub(crate) const PUBLISHER: &str = "dc:publisher";
    pub(crate) const SUBJECT: &str = "dc:subject";
    pub(crate) const MODIFIED: &str = "dcterms:modified";

    #[cfg(feature = "write")]
    mod write {
        pub(crate) const XMLNS_DC: &str = "xmlns:dc";
        pub(crate) const DUBLIN_CORE_NS: &str = "http://purl.org/dc/elements/1.1/";
        pub(crate) const NAMESPACE: &str = "dc:";

        // Elements
        pub(crate) const RIGHTS: &str = "dc:rights";
    }
}

pub(crate) mod ocf {
    #[cfg(feature = "write")]
    pub(crate) use write::*;

    // Paths
    pub(crate) const CONTAINER_PATH: &str = "META-INF/container.xml";

    // Elements
    pub(crate) const ROOT_FILE: &str = "rootfile";

    // Rootfile attribute keys
    pub(crate) const FULL_PATH: &str = "full-path";
    pub(crate) const MEDIA_TYPE: &str = super::_MEDIA_TYPE;

    #[cfg(feature = "write")]
    mod write {
        pub(crate) const CONTAINER_NS: &str = "urn:oasis:names:tc:opendocument:xmlns:container";
        pub(crate) const CONTAINER_VERSION: &str = "1.0";

        // Elements
        pub(crate) const CONTAINER: &str = "container";
        pub(crate) const ROOT_FILES: &str = "rootfiles";

        // Container attribute keys
        pub(crate) const VERSION: &str = super::super::_VERSION;
    }
}

pub(crate) mod opf {
    #[cfg(feature = "write")]
    pub(crate) use write::*;

    // Elements
    pub(crate) const PACKAGE: &str = "package";
    pub(crate) const METADATA: &str = "metadata";
    pub(crate) const META: &str = super::_META;
    pub(crate) const LINK: &str = super::_LINK;
    pub(crate) const MANIFEST: &str = "manifest";
    pub(crate) const SPINE: &str = "spine";
    pub(crate) const GUIDE: &str = "guide";
    pub(crate) const ITEM: &str = "item";
    pub(crate) const ITEMREF: &str = "itemref";
    pub(crate) const REFERENCE: &str = "reference";

    // Package attribute keys
    pub(crate) const VERSION: &str = super::_VERSION;
    pub(crate) const UNIQUE_ID: &str = "unique-identifier";
    pub(crate) const PREFIX: &str = "prefix";

    // Legacy EPUB 2 metadata attribute keys
    pub(crate) const NAME: &str = super::_NAME;
    pub(crate) const CONTENT: &str = super::_CONTENT;

    // Metadata attribute keys
    pub(crate) const PROPERTY: &str = "property";
    pub(crate) const REFINES: &str = "refines";
    pub(crate) const TEXT_DIR: &str = "dir";

    // Link attribute keys
    pub(crate) const REL: &str = super::_REL; // link only
    pub(crate) const HREFLANG: &str = "hreflang";

    // Metadata refinements
    pub(crate) const TITLE_TYPE: &str = "title-type";
    pub(crate) const IDENTIFIER_TYPE: &str = "identifier-type";
    pub(crate) const FILE_AS: &str = "file-as";
    pub(crate) const ROLE: &str = "role";
    pub(crate) const AUTHORITY: &str = "authority";
    pub(crate) const TERM: &str = "term";
    pub(crate) const ALTERNATE_SCRIPT: &str = "alternate-script";
    pub(crate) const DISPLAY_SEQ: &str = "display-seq";
    pub(crate) const SCHEME: &str = "scheme";

    // Metadata attribute values
    /// Legacy EPUB 2 cover image reference.
    pub(crate) const COVER: &str = "cover";
    pub(crate) const GENERATOR: &str = "generator";
    /// Value of [`OPF_EVENT`] attribute.
    pub(crate) const MODIFICATION: &str = "modification";
    /// Value of [`OPF_EVENT`] attribute.
    pub(crate) const PUBLICATION: &str = "publication";
    /// Value of [`TITLE_TYPE`] refinement.
    pub(crate) const MAIN_TITLE_TYPE: &str = "main";

    // Manifest item properties
    pub(crate) const COVER_IMAGE: &str = "cover-image";
    pub(crate) const NAV_PROPERTY: &str = "nav";

    // Manifest attribute keys
    pub(crate) const FALLBACK: &str = "fallback";
    pub(crate) const MEDIA_OVERLAY: &str = "media-overlay";

    // Spine attribute keys
    pub(crate) const TOC: &str = "toc";
    pub(crate) const PAGE_PROGRESSION_DIRECTION: &str = "page-progression-direction";
    pub(crate) const IDREF: &str = "idref";
    pub(crate) const LINEAR: &str = "linear";

    // Spine attribute values
    pub(crate) const YES: &str = "yes";

    // Guide attribute keys
    pub(crate) const TITLE: &str = super::_TITLE;
    pub(crate) const TYPE: &str = "type";

    // Attribute keys
    pub(crate) const MEDIA_TYPE: &str = super::_MEDIA_TYPE;
    pub(crate) const PROPERTIES: &str = "properties";
    pub(crate) const HREF: &str = super::_HREF;

    // Legacy Meta attribute keys
    pub(crate) const OPF_ROLE: &str = "opf:role";
    pub(crate) const OPF_FILE_AS: &str = "opf:file-as";
    pub(crate) const OPF_SCHEME: &str = "opf:scheme";
    pub(crate) const OPF_AUTHORITY: &str = "opf:authority";
    pub(crate) const OPF_TERM: &str = "opf:term";
    pub(crate) const OPF_ALT_REP: &str = "opf:alt-rep";
    pub(crate) const OPF_ALT_REP_LANG: &str = "opf:alt-rep-lang";
    pub(crate) const OPF_EVENT: &str = "opf:event";

    pub(crate) mod bytes {
        pub(crate) const PACKAGE: &[u8] = super::PACKAGE.as_bytes();
        pub(crate) const METADATA: &[u8] = super::METADATA.as_bytes();
        pub(crate) const MANIFEST: &[u8] = super::MANIFEST.as_bytes();
        pub(crate) const SPINE: &[u8] = super::SPINE.as_bytes();
        pub(crate) const GUIDE: &[u8] = super::GUIDE.as_bytes();
        pub(crate) const ITEM: &[u8] = super::ITEM.as_bytes();
        pub(crate) const ITEMREF: &[u8] = super::ITEMREF.as_bytes();
        pub(crate) const REFERENCE: &[u8] = super::REFERENCE.as_bytes();
    }

    #[cfg(feature = "write")]
    mod write {
        pub(crate) const XMLNS_OPF: &str = "xmlns:opf";
        pub(crate) const OPF_NS: &str = "http://www.idpf.org/2007/opf";

        // Meta attribute values
        pub(crate) const MARC_RELATORS: &str = "marc:relators";

        // Spine attribute values
        pub(crate) const NO: &str = "no";
    }
}

pub(crate) mod xhtml {
    #[cfg(feature = "write")]
    pub(crate) use write::*;

    // Elements
    pub(crate) const ANCHOR: &str = "a";
    pub(crate) const LIST_ITEM: &str = "li";
    pub(crate) const NAV: &str = "nav";
    pub(crate) const ORDERED_LIST: &str = "ol";

    // Attribute keys
    pub(crate) const HREF: &str = super::_HREF;

    pub(crate) mod bytes {
        pub(crate) const ANCHOR: &[u8] = super::ANCHOR.as_bytes();
        pub(crate) const LIST_ITEM: &[u8] = super::LIST_ITEM.as_bytes();
        pub(crate) const NAV: &[u8] = super::NAV.as_bytes();
    }

    #[cfg(feature = "write")]
    mod write {
        pub(crate) const XHTML_NS: &str = "http://www.w3.org/1999/xhtml";

        // Elements
        pub(crate) const HTML: &str = "html";
        pub(crate) const HEAD: &str = super::super::_HEAD;
        pub(crate) const BODY: &str = "body";
        pub(crate) const TITLE: &str = super::super::_TITLE;
        pub(crate) const H2: &str = "h2";
        pub(crate) const SPAN: &str = "span";
        pub(crate) const LINK: &str = super::super::_LINK;

        // Attribute keys
        pub(crate) const REL: &str = super::super::_REL;

        // Attribute key/value
        pub(crate) const HIDDEN: &str = "hidden";

        // Attribute value
        pub(crate) const STYLESHEET: &str = "stylesheet";
    }
}

pub(crate) mod ncx {
    #[cfg(feature = "write")]
    pub(crate) use write::*;

    // Elements
    pub(crate) const DOC_TITLE: &str = "docTitle";
    pub(crate) const NAV_MAP: &str = "navMap";
    pub(crate) const PAGE_LIST: &str = "pageList";
    pub(crate) const NAV_POINT: &str = "navPoint";
    pub(crate) const PAGE_TARGET: &str = "pageTarget";
    pub(crate) const NAV_LABEL: &str = "navLabel";
    pub(crate) const CONTENT: &str = super::_CONTENT;

    // pageList attribute keys
    pub(crate) const TYPE: &str = "type";
    pub(crate) const SRC: &str = "src";

    // constants where calling str.as_bytes() is not possible
    pub(crate) mod bytes {
        pub(crate) const DOC_TITLE: &[u8] = super::DOC_TITLE.as_bytes();
        pub(crate) const NAV_MAP: &[u8] = super::NAV_MAP.as_bytes();
        pub(crate) const PAGE_LIST: &[u8] = super::PAGE_LIST.as_bytes();
        pub(crate) const NAV_POINT: &[u8] = super::NAV_POINT.as_bytes();
        pub(crate) const PAGE_TARGET: &[u8] = super::PAGE_TARGET.as_bytes();
        pub(crate) const NAV_LABEL: &[u8] = super::NAV_LABEL.as_bytes();
        pub(crate) const CONTENT: &[u8] = super::CONTENT.as_bytes();
    }

    #[cfg(feature = "write")]
    mod write {
        pub(crate) const NCX_NS: &str = "http://www.daisy.org/z3986/2005/ncx/";
        pub(crate) const NCX_VERSION: &str = "2005-1";

        // Elements
        pub(crate) const NCX: &str = "ncx";
        pub(crate) const HEAD: &str = super::super::_HEAD;
        pub(crate) const TEXT: &str = "text";
        pub(crate) const META: &str = super::super::_META;

        // `ncx` element attribute keys
        pub(crate) const VERSION: &str = super::super::_VERSION;

        // NCX metadata types
        pub(crate) const DTB_UID: &str = "dtb:uid";
        pub(crate) const DTB_DEPTH: &str = "dtb:depth";
        pub(crate) const DTB_TOTAL_PAGE_COUNT: &str = "dtb:totalPageCount";
        pub(crate) const DTB_MAX_PAGE_NUMBER: &str = "dtb:maxPageNumber";

        // Attribute keys
        pub(crate) const PLAY_ORDER: &str = "playOrder";
        pub(crate) const CLASS: &str = "class";
        pub(crate) const VALUE: &str = "value";
        pub(crate) const NAME: &str = super::super::_NAME;

        // pageTarget type attribute value enumerations
        pub(crate) const FRONT: &str = "front";
        pub(crate) const NORMAL: &str = "normal";
        pub(crate) const SPECIAL: &str = "special";
    }
}
