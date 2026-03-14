#![no_main]
use libfuzzer_sys::{
    arbitrary::{Arbitrary, Error, Unstructured},
    fuzz_target,
};
use rlibphonenumber::{PHONE_NUMBER_UTIL, PhoneNumberFormat};
use rlibphonenumber_fuzz::ffi;

const ALPHABET: &[u8] = b"+0123456789()-=abcdefghijklmnopqrstuvwxyzABCDEFGHIJKLMNOPQRSTUVWXYZ";

struct CustomChar(u8);

impl<'a> Arbitrary<'a> for CustomChar {
    fn arbitrary(u: &mut Unstructured<'a>) -> Result<Self, Error> {
        Ok(Self(*u.choose(ALPHABET)?))
    }

    #[inline]
    fn size_hint(_depth: usize) -> (usize, Option<usize>) {
        (1, Some(1))
    }
}

#[derive(Debug)]
pub struct CustomString(pub String);

impl<'a> Arbitrary<'a> for CustomString {
    fn arbitrary(u: &mut Unstructured<'a>) -> Result<Self, Error> {
        let bytes = u
            .arbitrary::<Vec<CustomChar>>()?
            .into_iter()
            .map(|c| c.0)
            .collect::<Vec<u8>>();
        String::from_utf8(bytes)
            .map(Self)
            .map_err(|_| Error::IncorrectFormat)
    }
}

fuzz_target!(|data: (CustomString, CustomString)| {
    let (number_str, region_str) = (data.0.0, data.1.0);

    let cpp_res = ffi::test_cpp_impl(&number_str, &region_str);

    let util = &PHONE_NUMBER_UTIL;
    let rust_parsed = util.parse_with_default_region(&number_str, &region_str);

    assert_eq!(
        cpp_res.is_parsed,
        rust_parsed.is_ok(),
        "Mismatch on parsing! Rust_OK={}, Cpp_OK={} (error = {}). Input: '{}', Region: '{}'",
        rust_parsed.is_ok(),
        cpp_res.is_parsed,
        cpp_res.error,
        number_str,
        region_str
    );

    if let Ok(rust_num) = rust_parsed {
        let rust_is_valid = util.is_valid_number(&rust_num);
        assert_eq!(cpp_res.is_valid, rust_is_valid, "Mismatch on IsValidNumber");

        let rust_is_possible = util.is_possible_number(&rust_num);
        assert_eq!(
            cpp_res.is_possible, rust_is_possible,
            "Mismatch on IsPossibleNumber"
        );

        // ZZ for unknown
        let rust_region = util.get_region_code_for_number(&rust_num).unwrap_or("ZZ");
        assert_eq!(cpp_res.region_code, rust_region, "Mismatch on RegionCode");

        let rust_nsn = PHONE_NUMBER_UTIL.get_national_significant_number(&rust_num);
        assert_eq!(
            cpp_res.nsn,
            rust_nsn.as_ref(),
            "Mismatch on National Significant Number"
        );

        if rust_is_valid {
            let rust_e164 = util.format(&rust_num, PhoneNumberFormat::E164);
            assert_eq!(
                cpp_res.format_e164,
                rust_e164.as_ref(),
                "Mismatch on E164 Format"
            );

            let rust_intl = util.format(&rust_num, PhoneNumberFormat::International);
            assert_eq!(
                cpp_res.format_intl,
                rust_intl.as_ref(),
                "Mismatch on International Format"
            );

            let rust_natl = util.format(&rust_num, PhoneNumberFormat::National);
            assert_eq!(
                cpp_res.format_natl,
                rust_natl.as_ref(),
                "Mismatch on National Format"
            );

            let rust_rfc = util.format(&rust_num, PhoneNumberFormat::RFC3966);
            assert_eq!(
                cpp_res.format_rfc3966,
                rust_rfc.as_ref(),
                "Mismatch on RFC3966 Format"
            );

            let rust_mobile = util
                .format_number_for_mobile_dialing(&rust_num, &region_str, true)
                .unwrap_or_default();
            assert_eq!(
                cpp_res.format_mobile,
                rust_mobile.as_ref(),
                "Mismatch on Mobile Dialing Format"
            );

            let rust_ouc_keepeng_alpha =
                util.format_out_of_country_keeping_alpha_chars(&rust_num, &region_str);
            assert_eq!(
                cpp_res.out_of_country_keeping_alpha, rust_ouc_keepeng_alpha,
                "Mismatch on Out Of Country Format"
            );
        }
    }
});
