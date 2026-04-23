use rlibphonenumbers_macro::countries_enum;

countries_enum!(TEST);

// Test result for different countries
#[test]
fn test_module() {
    assert_eq!(TEST::AC.as_ref(), "AC");
    assert_eq!(TEST::US.as_ref(), "US");
    assert_eq!(TEST::NA.as_ref(), "NA");

    assert_eq!(TEST::from_code("AI"), Some(TEST::AI));
    assert_eq!(TEST::from_code("ZW"), Some(TEST::ZW));

    assert_eq!(TEST::from_code("ZZ"), Some(TEST::Custom([b'Z', b'Z'])));
    assert_eq!(TEST::from_code("AA"), Some(TEST::Custom([b'A', b'A'])));

    assert_eq!(TEST::from_code("ZW"), Some(TEST::ZW));

    assert_eq!(TEST::from_code("\u{02A9}"), None);
    assert_eq!(TEST::from_code("\u{0249}"), None);
    assert_eq!(TEST::from_code("//"), None);

    assert_eq!(TEST::from_code("001"), Some(TEST::World));
    assert_eq!(TEST::World.as_ref(), "001");
}
