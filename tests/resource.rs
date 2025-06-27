use rbook::ebook::resource::ResourceKind;
use wasm_bindgen_test::wasm_bindgen_test;

#[test]
#[wasm_bindgen_test]
fn test_resource_key_from() {
    assert_eq!("", ResourceKind::from("").as_str());
    assert_eq!("image/png", ResourceKind::from("image/png").as_str());
    assert_eq!(
        "audio/ogg",
        ResourceKind::from(String::from("audio/ogg")).as_str()
    );
}

#[test]
#[wasm_bindgen_test]
fn test_resource_kind_whitespace() {
    let a = ResourceKind::from("   audio/ogg  ");
    assert_eq!("audio", a.maintype());
    assert_eq!("ogg", a.subtype());

    let b = ResourceKind::from(" application/xhtml+xml  ");
    assert_eq!("application", b.maintype());
    assert_eq!("xhtml", b.subtype());
    assert_eq!(Some("xml"), b.suffix());
}

#[test]
#[wasm_bindgen_test]
fn test_resource_kind_eq() {
    let a = ResourceKind::from("example/test;param=XYZ;param2=ABC");
    let b = ResourceKind::from("  example/TEST; PARAM2 = ABC;param = XYZ;;;   ");
    assert_eq!(a, b);

    let c = ResourceKind::from("  example/test; param3 = ABC;param = XYZ;;;   ");
    assert_ne!(b, c);

    let d = ResourceKind::from("example/test");
    assert_ne!(a, d);

    let e = ResourceKind::from("  example/test    ");
    assert_eq!(d, e);
}
