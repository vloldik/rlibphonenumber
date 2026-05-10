#![no_main]
use libfuzzer_sys::{
    arbitrary::{Arbitrary, Error, Unstructured},
    fuzz_target,
};
use rlibphonenumber::{PHONE_NUMBER_UTIL, PhoneNumberFormat, Region};
use rlibphonenumber_fuzz::ffi;

const ALPHABET: &[u8] = b"+0123456789()-=abcdefghijklmnopqrstuvwxyzABCDEFGHIJKLMNOPQRSTUVWXYZ;";

#[derive(Debug)]
pub struct CustomString(pub String);

impl<'a> Arbitrary<'a> for CustomString {
    fn arbitrary(raw: &mut Unstructured<'a>) -> Result<Self, Error> {
        let bytes = raw.bytes(raw.len())?;
        let mut s = String::with_capacity(bytes.len());

        for &b in bytes {
            let char_idx = (b as usize) % ALPHABET.len();
            s.push(ALPHABET[char_idx] as char);
        }

        Ok(CustomString(s))
    }
}

fuzz_target!(|data: (CustomString, CustomString)| {
    let (number_str, region_str) = (data.0.0, data.1.0);

    let cpp_res = ffi::test_cpp_impl(&number_str, &region_str);
    let Ok(region) = Region::from_code(&region_str)
        .map(|code| Some(code))
        .or_else(|err| {
            // Empty regions as None
            if region_str.is_empty() || region_str == "ZZ" {
                Ok(None::<Region>)
            } else {
                Err(err)
            }
        })
    else {
        assert_eq!(
            cpp_res.is_parsed, false,
            "Mismatch on invalid code parsing! Cpp parsed country code, while rust rejected"
        );
        assert!(
            !cpp_res.error.is_empty(),
            "Mismatch on invalid code parsing! Cpp parsed country code, while rust rejected"
        );
        return;
    };

    let util = &PHONE_NUMBER_UTIL;
    let rust_parsed = util.parse(&number_str, region);

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
        if let Some(region) = region {
            let rust_region = util
                .get_region_for_number(&rust_num)
                .map(|reg| reg.as_region_str());
            let rust_region_str = rust_region.as_deref().unwrap_or("ZZ");
            assert_eq!(
                cpp_res.region_code, rust_region_str,
                "Mismatch on RegionCode"
            );
        }

        let rust_nsn = rust_num.get_national_significant_number();
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

            if let Some(region) = region {
                let rust_mobile = util
                    .format_number_for_mobile_dialing(&rust_num, region, true)
                    .unwrap_or_default();
                assert_eq!(
                    cpp_res.format_mobile,
                    rust_mobile.as_ref(),
                    "Mismatch on Mobile Dialing Format"
                );

                let rust_ouc_keepeng_alpha =
                    util.format_out_of_country_keeping_alpha_chars(&rust_num, region);
                assert_eq!(
                    cpp_res.out_of_country_keeping_alpha, rust_ouc_keepeng_alpha,
                    "Mismatch on Out Of Country Format"
                );
            }
        }
    }
});
