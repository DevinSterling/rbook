use rbook::Ebook;

#[test]
fn metadata_test() {
    let epub = rbook::Epub::new("example.epub").unwrap();

    // epub specification required metadata
    let title = epub.metadata().title();
    let identifier = epub.metadata().unique_identifier();

    assert_eq!("title", title.name());
    assert_eq!("Sword Art Online 1: Aincrad", title.value());
    assert_eq!("urn:uuid:3c6d9d4f-15c4-4261-a9d2-0c6bda3ad832", identifier.value());

    // Not epub specification required metadata
    let creators1 = epub.metadata().creators().unwrap();
    // Alternate way of retrieval
    let creators2 = epub.metadata().get("creator").unwrap(); // Namespace/prefix is not required
    let creator = creators1.first().unwrap();

    assert_eq!("Reki Kawahara and abec", creator.value());
    assert_eq!(creators1, creators2);

    // Refining (children) metadata and attributes
    let role = creator.get_child("role").unwrap(); // Child metadata element
    let scheme = role.get_attribute("scheme").unwrap(); // Attribute of an element

    assert_eq!("aut", role.value());
    assert_eq!("marc:relators", scheme.value())
}

#[test]
fn manifest_test() {
    let epub = rbook::Epub::new("example.epub").unwrap();

    // Retrieve manifest element by id
    let element = epub.manifest().by_id("chapter001").unwrap();
    let media_type = element.get_attribute("media-type").unwrap();

    assert_eq!("chapter001", element.name()); // id attribute value
    assert_eq!("chapter001.xhtml", element.value()); // href attribute value
    assert_eq!("application/xhtml+xml", media_type.value());

    // Retrieve file content using href
    let content = epub.read_file(element.value()).unwrap();

    assert!(content.starts_with("<html"));

    // Retrieve manifest element by property
    let element = epub.manifest().by_property("nav").unwrap();

    assert_eq!("toc.xhtml", element.value());

    // Retrieve manifest element by media type
    let element = epub.manifest().by_media_type("application/x-dtbncx+xml").unwrap();

    assert_eq!("toc.ncx", element.value());

    // Retrieve id of cover manifest element from optional cover metadata element
    let cover_id = epub.metadata().cover().unwrap().value();
    let cover_element1 = epub.manifest().by_id(cover_id).unwrap();
    // Alternate way of retrieval
    let cover_element2 = epub.cover_image().unwrap();

    assert_eq!(cover_element1, cover_element2);

    let content = epub.read_bytes_file(cover_element1.value());

    assert!(content.is_ok())
}

#[test]
fn spine_test() {
    let epub = rbook::Epub::new("example.epub").unwrap();

    // Get twelfth element in the spine
    let spine_element = epub.spine().elements().get(12).unwrap();

    // Get associated manifest element (name of a spine element is the value of the idref attribute)
    let manifest_element = epub.manifest().by_id(spine_element.name()).unwrap();

    // Compared value of "idref" and "id"
    assert_eq!(spine_element.name(), manifest_element.name());

    // Access spine attributes
    let direction = epub.spine().get_attribute("page-progression-direction").unwrap();

    assert_eq!("ltr", direction.value())
}

#[test]
fn guide_test() {
    let epub = rbook::Epub::new("example.epub").unwrap();

    assert_eq!(5, epub.guide().elements().len());

    let guide_element = epub.guide().by_type("copyright").unwrap();

    assert_eq!("copyright.xhtml", guide_element.value());

    let guide_element = epub.guide().by_type("toc").unwrap();
    let attribute = guide_element.get_attribute("type").unwrap();

    assert_eq!("Table of Contents", guide_element.name());
    assert_eq!("toc", attribute.value());
}

#[test]
fn toc_test() {
    let epub = rbook::Epub::new("example.epub").unwrap();

    let toc = epub.toc().elements();

    assert_eq!(32, toc.len());

    let toc_element = toc.get(30).unwrap();

    assert_eq!("Afterword", toc_element.name());
    assert_eq!("appendix001.xhtml", toc_element.value());

    let landmarks = epub.toc().landmarks().unwrap();

    assert_eq!(2, landmarks.len());

    let landmark_element = landmarks.get(1).unwrap();
    let attribute = landmark_element.get_attribute("type").unwrap();

    assert_eq!("Table of Contents", landmark_element.name());
    assert_eq!("toc", attribute.value());
}