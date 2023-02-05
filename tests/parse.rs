use rbook::xml::Find;
use rbook::Ebook;

#[test]
fn metadata_test() {
    let epub = rbook::Epub::new("tests/ebooks/childrens-literature.epub").unwrap();

    // epub specification required metadata
    let title = epub.metadata().title().unwrap();
    let identifier = epub.metadata().unique_identifier().unwrap();

    assert_eq!("title", title.name());
    assert_eq!("Children's Literature", title.value());
    assert_eq!("http://www.gutenberg.org/ebooks/25545", identifier.value());

    // Not epub specification required metadata
    let creators1 = epub.metadata().creators().unwrap();
    // Alternate way of retrieval
    let creators2 = epub.metadata().get("creator").unwrap(); // Namespace/prefix is not required

    assert_eq!(creators1, creators2);

    let creator1 = creators1.first().unwrap();
    let creator2 = creators1.last().unwrap();

    assert_eq!("Charles Madison Curry", creator1.value());
    assert_eq!("Erle Elsworth Clippinger", creator2.value());
}

#[test]
fn metadat_test_alt() {
    let epub = rbook::Epub::new("tests/ebooks/childrens-literature.epub").unwrap();

    // epub specification required metadata
    let title = epub.metadata().find_value("title").unwrap();
    let identifier = epub.metadata().find_value("identifier").unwrap();

    assert_eq!("Children's Literature", title);
    assert_eq!("http://www.gutenberg.org/ebooks/25545", identifier);

    // `get_value()` finds the first match
    // Find first `creator` element and return its value
    let creator1 = epub.metadata().find_value("creator").unwrap();
    // Find first `creator` element that contains an `id` attribute
    let creator1_alt = epub.metadata().find_value("creator[id]").unwrap();
    // Find first `creator` element that contains an `id` equal to a `curry`
    let creator1_alt2 = epub.metadata().find_value("creator[id=curry]").unwrap();

    assert_eq!(creator1, creator1_alt);
    assert_eq!(creator1_alt, creator1_alt2);

    let creator2 = epub
        .metadata()
        .find_value("creator[id=clippinger]")
        .unwrap();

    assert_eq!("Charles Madison Curry", creator1);
    assert_eq!("Erle Elsworth Clippinger", creator2);

    let creator1_role = epub
        .metadata()
        .find_value("creator[id=curry] > file-as")
        .unwrap();
    let creator2_role = epub
        .metadata()
        .find_value("creator[id=clippinger] > file-as[refines=#clippinger]")
        .unwrap();

    assert_eq!("Curry, Charles Madison", creator1_role);
    assert_eq!("Clippinger, Erle Elsworth", creator2_role);
}

#[test]
fn metadata_test2() {
    let epub = rbook::Epub::new("tests/ebooks/moby-dick.epub").unwrap();

    let creators = epub.metadata().creators().unwrap();
    let creator1 = creators.first().unwrap();

    assert_eq!("Herman Melville", creator1.value());

    // Refining (children) metadata and attributes
    let role = creator1.get_child("role").unwrap(); // Child metadata element
    let scheme = role.get_attribute("scheme").unwrap(); // Attribute of an element

    assert_eq!("aut", role.value());
    assert_eq!("marc:relators", scheme)
}

#[test]
fn manifest_test() {
    let epub = rbook::Epub::new("tests/ebooks/childrens-literature.epub").unwrap();

    // Retrieve manifest element by id
    let element = epub.manifest().by_id("s04").unwrap();
    let media_type = element.get_attribute("media-type").unwrap();

    assert_eq!("s04", element.name()); // id attribute value
    assert_eq!("s04.xhtml", element.value()); // href attribute value
    assert_eq!("application/xhtml+xml", media_type);

    // Retrieve file content using href
    let content = epub.read_file(element.value()).unwrap();

    assert!(content.starts_with("<?xml"));

    // Retrieve manifest element by property
    let element = epub.manifest().by_property("nav").unwrap();

    assert_eq!("nav.xhtml", element.value());

    // Retrieve manifest element by media type
    let element = epub
        .manifest()
        .by_media_type("application/x-dtbncx+xml")
        .unwrap();

    assert_eq!("toc.ncx", element.value());

    // Alternate way of retrieval
    let cover_element = epub.cover_image().unwrap();
    let content1 = epub.read_bytes_file(cover_element.value()).unwrap();
    // Provided paths are normalized
    let content2 = epub
        .read_bytes_file("EPUB//./images//primary/..///./cover.png")
        .unwrap();

    assert_eq!(content1, content2);
}

#[test]
fn spine_test() {
    let epub = rbook::Epub::new("tests/ebooks/childrens-literature.epub").unwrap();

    let spine_elements = epub.spine().elements();

    // Get twelfth element in the spine
    let spine_element12 = spine_elements.get(1).unwrap();

    // Get associated manifest element (name of a spine element is the value of the idref attribute)
    let manifest_element = epub.manifest().by_id(spine_element12.name()).unwrap();

    // Compared value of "idref" and "id"
    assert_eq!(spine_element12.name(), manifest_element.name());

    // Access spine attributes
    let direction = epub.spine().get_attribute("toc").unwrap();

    assert_eq!("ncx", direction)
}

// #[test]
// fn guide_test() {
//     let epub = rbook::Epub::new("tests/ebooks/example.epub").unwrap();
//
//     assert_eq!(5, epub.guide().elements().len());
//
//     let guide_element = epub.guide().by_type("copyright").unwrap();
//
//     assert_eq!("copyright.xhtml", guide_element.value());
//
//     let guide_element = epub.guide().by_type("toc").unwrap();
//     let attribute = guide_element.get_attribute("type").unwrap();
//
//     assert_eq!("Table of Contents", guide_element.name());
//     assert_eq!("toc", attribute.value());
// }

#[test]
fn toc_test() {
    let epub = rbook::Epub::new("tests/ebooks/moby-dick.epub").unwrap();

    let toc = epub.toc().elements();

    assert_eq!(141, toc.len());

    let toc_element1 = toc.get(30).unwrap();
    let toc_element2 = toc.get(140).unwrap();

    assert_eq!("Chapter 27. Knights and Squires.", toc_element1.name());
    assert_eq!("chapter_027.xhtml", toc_element1.value());
    assert_eq!("Copyright Page", toc_element2.name());
    assert_eq!("copyright.xhtml", toc_element2.value());

    let landmarks = epub.toc().landmarks().unwrap();

    assert_eq!(3, landmarks.len());

    let landmark_element = landmarks.get(1).unwrap();
    let attribute = landmark_element.get_attribute("type").unwrap();

    assert_eq!("Begin Reading", landmark_element.name());
    assert_eq!("bodymatter", attribute);
}

#[test]
fn directory_test() {
    let epub = rbook::Epub::new("tests/ebooks/example_epub").unwrap();

    let title = epub.metadata().title().unwrap();
    assert_eq!("Directory Example", title.value());

    let creators = epub.metadata().creators().unwrap();
    let creator = creators.first().unwrap();
    assert_eq!("Devin Sterling", creator.value());

    let role = creator.get_child("role").unwrap();
    assert_eq!("aut", role.value());

    let sources = epub.metadata().get("source").unwrap();
    let source = sources.first().unwrap();
    assert_eq!("rbook", source.value());

    let manifest_element = epub.manifest().by_id("c2").unwrap();
    assert_eq!("c2.xhtml", manifest_element.value());
    assert_eq!(4, epub.spine().elements().len());

    let toc_elements = epub.toc().elements();
    let toc_element1 = toc_elements.get(1).unwrap();

    assert_eq!("rbook c1", toc_element1.name());
    assert_eq!("EPUB/c1.xhtml", toc_element1.value());

    assert_eq!(None, epub.cover_image());
}
