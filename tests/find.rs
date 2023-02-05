use rbook::xml::Find;
use rbook::Ebook;

#[test]
fn metadata_find_test() {
    let epub = rbook::Epub::new("tests/ebooks/moby-dick.epub").unwrap();

    // Find the first `title` element
    let title = epub.metadata().find_value("title").unwrap();
    assert_eq!("Moby-Dick", title);

    // Find the first `creator` element
    let creator = epub.metadata().find("creator").unwrap();
    // Find the first element with where its `id` attribute equals `creator`
    // Note: it is coincidence here that the name of the element and id are the same
    let creator_alt = epub.metadata().find_value("*[id=creator]").unwrap();
    assert_eq!(creator.value(), creator_alt);

    // Retrieve all creators of an epub
    let creators = epub.metadata().creators().unwrap();
    let creators2 = epub.metadata().find_all("creator").unwrap();
    assert_eq!(creators, creators2);

    // Find the first `creator` element that has a child `file-as` element
    let creator = epub.metadata().find("creator > file-as").unwrap();
    // Find any first element that has a child `file-as` element
    let creator_alt = epub.metadata().find("* > file-as").unwrap();
    assert_eq!(creator, creator_alt);

    // Find any element with any child element that has a `refines` attribute that equals `#contrib1`
    let creator_alt2 = epub
        .metadata()
        .find_value("* > *[refines=#creator]")
        .unwrap();
    assert_eq!(creator_alt.value(), creator_alt2);
}

#[test]
fn manifest_find_test() {
    let epub = rbook::Epub::new("tests/ebooks/childrens-literature.epub").unwrap();

    let xhtml_toc = epub.manifest().by_property("nav").unwrap();
    let xhtml_toc2 = epub.manifest().find_value("*[properties=nav]").unwrap();
    assert_eq!(xhtml_toc.value(), xhtml_toc2);

    // `find` searches using the element name. For rbook, all manifest
    // element names are its `id` value for convenience.
    let chapter = epub.manifest().by_id("s04").unwrap();
    let chapter_alt1 = epub.manifest().find("s04").unwrap();
    let chapter_alt2 = epub.manifest().find("*[id=s04]").unwrap();
    assert_eq!(chapter, chapter_alt1);
    assert_eq!(chapter_alt1, chapter_alt2);

    let img_files = epub.manifest().all_by_media_type("image/png").unwrap();
    let img_files2 = epub.manifest().find_all("*[media-type=image/png]").unwrap();
    assert_eq!(img_files, img_files2);
}

#[test]
fn spine_find_test() {
    let epub = rbook::Epub::new("tests/ebooks/moby-dick.epub").unwrap();

    let spine_elements = epub.spine().elements();

    // `find` searches using the element name. For rbook, all manifest
    // element names are its `idref` value for convenience.
    let spine_element = epub.spine().find("xchapter_022").unwrap();
    assert!(spine_elements.contains(&spine_element));

    let non_linear_element = epub.spine().find("*[linear=no]").unwrap();
    assert_eq!("cover", non_linear_element.name())
}

#[test]
fn toc_find_test() {
    let epub = rbook::Epub::new("tests/ebooks/childrens-literature.epub").unwrap();

    // `find` searches using the element name. For rbook, all toc
    // element names are its `label` value for convenience.
    let toc_element = epub.toc().find("190 A FOUR-LEAVED CLOVER").unwrap();
    assert_eq!("s04.xhtml#pgepubid00503", toc_element.value());

    let toc_element2 = epub.toc().find("*[id=np-319]").unwrap();
    assert_eq!(toc_element, toc_element2);

    // Retrieving parent element from a nested child element
    let parent = toc_element.parent().unwrap();
    assert_eq!("Abram S. Isaacs", parent.name());

    let toc_element3 = epub
        .toc()
        .find("SECTION IV FAIRY STORIES—MODERN FANTASTIC TALES > Abram S. Isaacs > 190 A FOUR-LEAVED CLOVER")
        .unwrap();
    assert_eq!(toc_element, toc_element3);
}
