use crate::PHONE_NUMBER_UTIL;

#[test]
fn is_valid_number_mismatch() {
    let cases = [("CD(+48X666666644", "", true)];

    cases
        .iter()
        .for_each(|(phone, country_code, should_valid)| {
            assert_eq!(
                PHONE_NUMBER_UTIL
                    .parse_with_default_region(phone, country_code)
                    .unwrap()
                    .is_valid(),
                *should_valid
            )
        });
}
