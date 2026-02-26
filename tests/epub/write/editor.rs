use crate::epub::util::round_trip_epub;
use rbook::Epub;
use rbook::epub::EpubEditor;
use rbook::epub::metadata::DetachedEpubMetaEntry;
use wasm_bindgen_test::wasm_bindgen_test;

#[rustfmt::skip]
fn epub_author_builder() -> EpubEditor<'static> {
    Epub::builder()
        .author(["John Doe", "Jane Doe"])
        .author([
            DetachedEpubMetaEntry::creator("Hanako Yamada")
                .role("ill")
                .role("trl")
                .role("edt"),
            DetachedEpubMetaEntry::creator("Taro Yamada")
                .role("edt")
                .attribute(("opf:role", "edt")),
        ])
}

#[test]
#[wasm_bindgen_test]
fn test_epub2_editor_author() {
    let built_epub = epub_author_builder().version(2).build();
    let epub = round_trip_epub(&built_epub);
    let metadata = epub.metadata();

    let expected_authors = ["John Doe", "Jane Doe", "Hanako Yamada", "Taro Yamada"];

    for (name, author) in expected_authors.into_iter().zip(metadata.creators()) {
        assert_eq!(name, author.value());
        assert_eq!(1, author.roles().count());
        assert_eq!("aut", author.roles().next().unwrap().code());
        assert_eq!(Some("aut"), author.attributes().get_value("opf:role"));
    }
}

#[test]
#[wasm_bindgen_test]
fn test_epub3_editor_author() {
    let built_epub = epub_author_builder().build();
    let epub = round_trip_epub(&built_epub);
    let metadata = epub.metadata();

    let expected_authors = [
        // (name, roles, opf:role)
        ("John Doe", vec!["aut"], None),
        ("Jane Doe", vec!["aut"], None),
        ("Hanako Yamada", vec!["aut", "ill", "trl", "edt"], None),
        ("Taro Yamada", vec!["aut", "edt"], Some("aut")),
    ];

    assert_eq!(4, metadata.creators().count());

    for ((name, roles, opf_role), author) in expected_authors.into_iter().zip(metadata.creators()) {
        assert_eq!(name, author.value());
        assert_eq!(opf_role, author.attributes().get_value("opf:role"));
        assert_eq!(roles.len(), author.roles().count());

        for (expected_role, role) in author.roles().zip(roles) {
            assert_eq!(expected_role.code(), role);
        }
    }
}
