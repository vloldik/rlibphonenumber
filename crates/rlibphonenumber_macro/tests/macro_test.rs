use rlibphonenumber_macro::countries_enum;

countries_enum!(TEST);

// Test result for different countries
#[test]
fn test_module() {
    assert_eq!(&*TEST::AC.as_region_str(), "AC");
    assert_eq!(&*TEST::US.as_region_str(), "US");
    assert_eq!(&*TEST::NA.as_region_str(), "NA");

    assert_eq!(TEST::from_code("AI"), Ok(TEST::AI));
    assert_eq!(TEST::from_code("ZW"), Ok(TEST::ZW));

    assert_eq!(TEST::from_code("ZZ"), Ok(TEST::ZZ));
    assert_eq!(TEST::from_code("AA"), Ok(TEST::AA));

    assert_eq!(TEST::from_code("ZW"), Ok(TEST::ZW));

    assert!(matches!(
        TEST::from_code("\u{02A9}"),
        Err(InvalidRegionError::InvalidCharacter(..))
    ));
    assert!(matches!(
        TEST::from_code("\u{0249}"),
        Err(InvalidRegionError::InvalidCharacter(..))
    ));
    assert_eq!(
        TEST::from_code("//"),
        Err(InvalidRegionError::InvalidCharacter([b'/', b'/']))
    );

    assert_eq!(TEST::from_code("001"), Ok(TEST::World));
    assert_eq!(&*TEST::World.as_region_str(), "001");
}
