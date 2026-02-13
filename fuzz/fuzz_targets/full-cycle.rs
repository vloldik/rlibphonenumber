#![no_main]
use libfuzzer_sys::fuzz_target;
use rlibphonenumber::{PHONE_NUMBER_UTIL, PhoneNumberFormat};

fuzz_target!(|data: (String, String)| {
    let (number_str, region_str) = data;
    let util = &PHONE_NUMBER_UTIL;

    if let Ok(phone_number) = util.parse(&number_str, &region_str) {
        let _ = util.is_valid_number(&phone_number);
        let _ = util.is_possible_number(&phone_number);
        let _ = util.is_number_geographical(&phone_number);

        let _ = util.get_region_code_for_number(&phone_number);
        let _ = util.get_number_type(&phone_number);
        let _ = util.get_national_significant_number(&phone_number);

        let _ = util.format(&phone_number, PhoneNumberFormat::E164);
        let _ = util.format(&phone_number, PhoneNumberFormat::International);
        let _ = util.format(&phone_number, PhoneNumberFormat::National);
        let _ = util.format(&phone_number, PhoneNumberFormat::RFC3966);

        let _ = util.format_number_for_mobile_dialing(&phone_number, &region_str, true);
    }
});
