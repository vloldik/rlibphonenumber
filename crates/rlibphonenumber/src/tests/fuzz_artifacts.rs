use std::sync::Arc;

use crate::{
    PHONE_NUMBER_UTIL, PhoneNumberUtil, Region,
    phonenumber_matcher::{Leniency, PhoneNumberMatch, PhoneNumberMatcherFactory},
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
    let factory = PhoneNumberMatcherFactory::new_with_formats(
        Arc::new(PhoneNumberUtil::new().unwrap()),
        None,
    );

    let match_text = |text: &'static str, region| -> Vec<PhoneNumberMatch<'_>> {
        let matcher = factory.create_matcher(text, Leniency::Possible, u64::MAX, region);
        matcher.collect()
    };

    assert_eq!(
        match_text(
            "0wJ++6262222XxCwJ++62622226666X8888888888888888880",
            Some(Region::US)
        )
        .len(),
        1
    );
}
