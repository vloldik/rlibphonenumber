use crate::{
    PHONE_NUMBER_UTIL, Region,
    phonenumber_matcher::{Leniency, PhoneNumberMatch},
    tests::common::get_phone_matcher_factory,
};

#[test]
fn is_valid_number_mismatch() {
    let cases = [("CD(+48X666666644", None::<Region>, true)];

    cases
        .iter()
        .for_each(|(phone, country_code, should_valid)| {
            assert_eq!(
                PHONE_NUMBER_UTIL
                    .parse(phone, *country_code)
                    .unwrap()
                    .is_valid(),
                *should_valid
            )
        });
}

#[test]
fn matcher_number_of_outputs_mismatch() {
    let factory = get_phone_matcher_factory();

    let match_text = |text: &'static str| -> Vec<PhoneNumberMatch<'_>> {
        let matcher = factory.create_matcher(text, Leniency::Valid, u64::MAX, None);
        matcher.collect()
    };

    assert_eq!(match_text(".6+.+492222262+9").len(), 1);

    assert_eq!(
        match_text("0wJ++6262222XxCwJ++62622226666X8888888888888888880").len(),
        1
    );
}
