use rlibphonenumbers_macro::countries_enum;

countries_enum!(TEST);

// Test result for different countries
#[test]
fn test_module() {
    assert_eq!(TEST::AC.as_ref(), "AC");
    assert_eq!(TEST::US.as_ref(), "US");
    assert_eq!(TEST::NA.as_ref(), "NA");
}
