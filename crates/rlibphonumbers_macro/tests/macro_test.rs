#[cfg(test)]
mod test {
    use rlibphonumbers_macro::contries_enum;

    contries_enum!(TEST);

    // Test result for different countries
    #[test]
    fn test_module() {
        assert_eq!(TEST::AC.as_ref(), "AC");
        assert_eq!(TEST::US.as_ref(), "US");
        assert_eq!(TEST::NA.as_ref(), "NA");
    }
}
