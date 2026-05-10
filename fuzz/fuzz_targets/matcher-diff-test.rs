#![no_main]
use std::sync::LazyLock;

use libfuzzer_sys::{
    arbitrary::{Arbitrary, Error, Unstructured},
    fuzz_target,
};
use rlibphonenumber::{
    PhoneNumberUtil, Region,
    phonenumber_matcher::{Leniency, PhoneNumberMatcherFactory},
};
use rlibphonenumber_fuzz::ffi;

const ALPHABET: &[u8] = b"+0123456789()-=. abcdefghijklmnopqrstuvwxyzABCDEFGHIJKLMNOPQRSTUVWXYZ;";

#[derive(Debug)]
pub struct FuzzText(pub String);

impl<'a> Arbitrary<'a> for FuzzText {
    fn arbitrary(u: &mut Unstructured<'a>) -> Result<Self, Error> {
        let bytes = u.bytes(u.len())?;
        let mut s = String::with_capacity(bytes.len());
        for &b in bytes {
            s.push(ALPHABET[(b as usize) % ALPHABET.len()] as char);
        }
        Ok(FuzzText(s))
    }
}

static PHONE_NUMBER_MATCHER_FACTORY: LazyLock<
    PhoneNumberMatcherFactory<PhoneNumberUtil, &PhoneNumberUtil>,
> = LazyLock::new(|| PhoneNumberMatcherFactory::new());

fuzz_target!(|data: (FuzzText, FuzzText)| {
    let text = data.0.0;
    let region_str = data.1.0;

    let region: Option<Region> = match Region::from_code(&region_str) {
        Ok(r) => Some(r),
        Err(_) => {
            if region_str.is_empty() || region_str == "ZZ" {
                None
            } else {
                let cpp_matches = ffi::test_cpp_matcher(&text, &region_str);
                assert!(
                    cpp_matches.is_empty(),
                    "C++ found matches for invalid region '{}' in text '{}'",
                    region_str,
                    text,
                );
                return;
            }
        }
    };

    let matcher = PHONE_NUMBER_MATCHER_FACTORY.create_matcher(
        &text,
        Leniency::Valid,
        i32::MAX.try_into().unwrap(),
        region,
    );
    let rust_matches: Vec<_> = matcher.collect();

    let cpp_matches = ffi::test_cpp_matcher(&text, &region_str);

    assert_eq!(
        rust_matches.len(),
        cpp_matches.len(),
        "Match count mismatch: rust={}, cpp={} | text='{}', region='{}', {:?}",
        rust_matches.len(),
        cpp_matches.len(),
        text,
        region_str,
        cpp_matches
    );

    for (i, (rust_m, cpp_m)) in rust_matches.iter().zip(cpp_matches.iter()).enumerate() {
        assert_eq!(
            rust_m.start, cpp_m.start as usize,
            "[match {}] start offset mismatch | text='{}', region='{}'",
            i, text, region_str,
        );

        assert_eq!(
            rust_m.end(),
            cpp_m.end as usize,
            "[match {}] end offset mismatch | text='{}', region='{}'",
            i,
            text,
            region_str,
        );

        assert_eq!(
            rust_m.raw_string,
            cpp_m.raw_string.as_str(),
            "[match {}] raw_string mismatch | text='{}', region='{}'",
            i,
            text,
            region_str,
        );

        let rust_e164 = rust_m
            .number
            .format_as(rlibphonenumber::PhoneNumberFormat::E164);
        assert_eq!(
            rust_e164.as_ref(),
            cpp_m.e164.as_str(),
            "[match {}] E164 mismatch: rust='{}', cpp='{}' | text='{}', region='{}'",
            i,
            rust_e164,
            cpp_m.e164,
            text,
            region_str,
        );
    }
});
