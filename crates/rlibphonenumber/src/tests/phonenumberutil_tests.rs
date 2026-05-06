#![allow(deprecated, clippy::field_reassign_with_default)]

use crate::{
    InternalError, Region,
    enums::{NumberLengthType, PhoneNumberFormat, PhoneNumberType},
    errors::{ParseError, ValidationError},
    generated::{
        proto::{
            NumberFormat, PhoneMetadata, PhoneNumber, PhoneNumberDesc,
            phone_number::CountryCodeSource,
        },
        uniprops_digits,
    },
    phonenumberutil::{
        helper_functions::get_national_significant_number,
        phonenumberutil_internal::PhoneNumberUtilInternal,
        regex_wrapper_types::{NumberFormatWrapper, RegexTriplets},
    },
    tests::common::get_phone_util,
    unwrap_internal,
};

fn wrap_regexp_str(regexp: &str) -> Option<String> {
    format!("^(?:{})$", regexp).into()
}

#[test]
fn interchange_invalid_codepoints() {
    let phone_util = get_phone_util();

    let valid_inputs = vec![
        "+44\u{2013}2087654321", // U+2013, EN DASH
    ];
    for input in valid_inputs {
        assert_eq!(
            input,
            input
                .chars()
                .map(|c| {
                    uniprops_digits::uniprops::get_digit_value(c)
                        .map(|c| (c + b'0') as char)
                        .unwrap_or(c)
                })
                .collect::<String>()
                .as_str()
        );
        assert!(phone_util.is_viable_phone_number(input));
        phone_util.parse(input, Some(Region::GB)).unwrap();
    }

    let invalid_inputs = vec![
        "+44\u{96}2087654321",   // Invalid sequence
        "+44\u{0096}2087654321", // U+0096
        "+44\u{fffe}2087654321", // U+FFFE
        // Unassigned end chars
        "+44\u{2013}2087654321\u{0378}\u{0378}\u{0378}\u{0378}",
    ];
    for input in invalid_inputs {
        assert!(!phone_util.is_viable_phone_number(input));
        assert!(
            phone_util
                .parse(input, Some(Region::GB))
                .is_err_and(|err| matches!(
                    err,
                    InternalError::Wrapped(ParseError::NotANumber(..))
                ))
        );
    }
}

#[test]
fn get_supported_regions() {
    let phone_util = get_phone_util();
    assert!(phone_util.get_supported_regions().count() > 0)
}

#[test]
fn get_supported_global_network_calling_codes() {
    let phone_util = get_phone_util();
    let calling_codes = phone_util
        .get_supported_global_network_calling_codes()
        .collect::<Vec<_>>();
    assert!(!calling_codes.is_empty());
    for &code in &calling_codes {
        assert!(code > 0);
        let region = phone_util.get_region_for_country_code(code);
        assert_eq!(Some(Region::World), region);
    }
}

#[test]
fn get_supported_calling_codes() {
    let phone_util = get_phone_util();
    let calling_codes = phone_util.get_supported_calling_codes().collect::<Vec<_>>();
    assert!(!calling_codes.is_empty());
    for &code in &calling_codes {
        assert!(code > 0);
        let region = phone_util.get_region_for_country_code(code);
        assert_ne!(None, region);
    }
    let supported_global_network_calling_codes = phone_util
        .get_supported_global_network_calling_codes()
        .collect::<Vec<_>>();
    assert!(calling_codes.len() > supported_global_network_calling_codes.len());
    assert!(calling_codes.contains(&979));
}

#[test]
fn get_supported_types_for_region() {
    let phone_util = get_phone_util();
    let types = phone_util
        .get_supported_types_for_region(Region::BR)
        .expect("region should exist");
    assert!(types.contains(&PhoneNumberType::FixedLine));
    assert!(!types.contains(&PhoneNumberType::Mobile));
    assert!(!types.contains(&PhoneNumberType::Unknown));

    let types = phone_util
        .get_supported_types_for_region(Region::US)
        .expect("region should exist");
    assert!(types.contains(&PhoneNumberType::FixedLine));
    assert!(types.contains(&PhoneNumberType::Mobile));
    assert!(!types.contains(&PhoneNumberType::FixedLineOrMobile));
}

#[test]
fn get_supported_types_for_non_geo_entity() {
    let phone_util = get_phone_util();
    let types = phone_util.get_supported_types_for_non_geo_entity(999);
    assert!(types.is_none());

    let types = phone_util
        .get_supported_types_for_non_geo_entity(979)
        .expect("Code should exist");
    assert!(types.contains(&PhoneNumberType::PremiumRate));
    assert!(!types.contains(&PhoneNumberType::Mobile));
    assert!(!types.contains(&PhoneNumberType::Unknown));
}

#[test]
fn get_regions_for_country_calling_code() {
    let phone_util = get_phone_util();
    let expect_regions = |code| {
        phone_util
            .get_regions_for_country_calling_code(code)
            .expect("Codes should exist")
            .collect::<Vec<_>>()
    };

    let regions = expect_regions(1);
    assert!(regions.contains(&Region::US));
    assert!(regions.contains(&Region::BS));

    let regions = expect_regions(44);
    assert!(regions.contains(&Region::GB));

    let regions = expect_regions(49);
    assert!(regions.contains(&Region::DE));

    let regions = expect_regions(800);
    assert!(regions.contains(&Region::World));

    const INVALID_COUNTRY_CODE: i32 = 2;
    assert!(
        phone_util
            .get_regions_for_country_calling_code(INVALID_COUNTRY_CODE)
            .is_none()
    );
}

#[test]
fn get_instance_load_us_metadata() {
    let phone_util = get_phone_util();
    let assert_pat_eq = |original: &str, pat: &RegexTriplets| {
        assert_eq!(
            format!("^(?:{})$", original),
            pat.anchor_full().unwrap().unwrap().as_str()
        );
    };
    let metadata = phone_util.get_metadata_for_region(Region::US).unwrap();
    assert_eq!(&*Region::US.as_region_str(), metadata.original.id);
    assert_eq!(1, metadata.original.country_code());
    assert_eq!("011", metadata.international_prefix().original_base());
    assert!(metadata.original.national_prefix.is_some());
    assert_eq!(2, metadata.number_format.len());
    assert_pat_eq(
        "(\\d{3})(\\d{3})(\\d{4})",
        metadata.number_format[1].pattern(),
    );
    assert_eq!("$1 $2 $3", metadata.number_format[1].original.format);
    assert_pat_eq(
        "[13-689]\\d{9}|2[0-35-9]\\d{8}",
        metadata.general_desc.national_number_pattern(),
    );
    assert_pat_eq(
        "[13-689]\\d{9}|2[0-35-9]\\d{8}",
        metadata.fixed_line.national_number_pattern(),
    );
    assert_eq!(1, metadata.general_desc.original.possible_length.len());
    assert_eq!(10, metadata.general_desc.original.possible_length[0]);
    assert_eq!(0, metadata.toll_free.original.possible_length.len());
    assert_pat_eq("900\\d{7}", metadata.premium_rate.national_number_pattern());
    assert!(
        metadata
            .shared_cost
            .original
            .national_number_pattern
            .is_none()
    );
}

#[test]
fn get_instance_load_de_metadata() {
    let phone_util = get_phone_util();
    let metadata_wrapper = phone_util.get_metadata_for_region(Region::DE).unwrap();
    let metadata = &metadata_wrapper.original;
    assert_eq!(&*Region::DE.as_region_str(), metadata.id);
    assert_eq!(49, metadata.country_code());
    assert_eq!(
        "00",
        metadata_wrapper.international_prefix().original_base()
    );
    assert_eq!("0", metadata.national_prefix());
    assert_eq!(6, metadata_wrapper.number_format.len());
    assert_eq!(
        1,
        metadata_wrapper.number_format[5]
            .leading_digits_pattern()
            .len()
    );
    let assert_pat_eq = |original: &str, pat: &RegexTriplets| {
        assert_eq!(
            format!("^(?:{})$", original),
            pat.anchor_full().unwrap().unwrap().as_str()
        );
    };

    assert_pat_eq(
        "900",
        &metadata_wrapper.number_format[5].leading_digits_pattern()[0],
    );

    assert_pat_eq(
        "(\\d{3})(\\d{3,4})(\\d{4})",
        metadata_wrapper.number_format[5].pattern(),
    );
    assert_eq!(
        2,
        metadata_wrapper
            .general_desc
            .original
            .possible_length_local_only
            .len()
    );
    assert_eq!(
        8,
        metadata_wrapper.general_desc.original.possible_length.len()
    );
    assert_eq!(
        0,
        metadata_wrapper.fixed_line.original.possible_length.len()
    );
    assert_eq!(2, metadata_wrapper.mobile.original.possible_length.len());
    assert_eq!(
        "$1 $2 $3",
        metadata_wrapper.number_format[5].original.format
    );
    assert_pat_eq(
        "(?:[24-6]\\d{2}|3[03-9]\\d|[789](?:0[2-9]|[1-9]\\d))\\d{1,8}",
        metadata_wrapper.fixed_line.national_number_pattern(),
    );
    assert_eq!(
        "30123456",
        metadata_wrapper.fixed_line.original.example_number()
    );
    assert_eq!(10, metadata_wrapper.toll_free.original.possible_length[0]);
    assert_pat_eq(
        "900([135]\\d{6}|9\\d{7})",
        metadata_wrapper.premium_rate.national_number_pattern(),
    );
}

#[test]
fn get_instance_load_ar_metadata() {
    let phone_util = get_phone_util();
    let metadata_wrapper = phone_util.get_metadata_for_region(Region::AR).unwrap();
    let metadata = &metadata_wrapper.original;

    let assert_pat_eq = |original: &str, pat: &RegexTriplets| {
        assert_eq!(
            format!("^(?:{})$", original),
            pat.anchor_full().unwrap().unwrap().as_str()
        );
    };
    assert_eq!(&*Region::AR.as_region_str(), metadata.id.as_str());
    assert_eq!(54, metadata.country_code());
    assert_eq!(
        &wrap_regexp_str("00").unwrap(),
        metadata_wrapper
            .international_prefix()
            .pattern_base
            .as_ref()
            .unwrap()
            .as_str()
    );
    assert_eq!("0", metadata.national_prefix());
    assert_pat_eq(
        "0(?:(11|343|3715)15)?",
        metadata_wrapper.national_prefix_for_parsing(),
    );
    assert_eq!("9$1", metadata.national_prefix_transform_rule());
    assert_eq!(5, metadata_wrapper.number_format.len());
    assert_eq!(
        "$2 15 $3-$4",
        metadata_wrapper.number_format[2].original.format
    );
    assert_pat_eq(
        "(\\d)(\\d{4})(\\d{2})(\\d{4})",
        metadata_wrapper.number_format[3].pattern(),
    );
    assert_pat_eq(
        "(\\d)(\\d{4})(\\d{2})(\\d{4})",
        metadata_wrapper.intl_number_format[3].pattern(),
    );
    assert_eq!(
        "$1 $2 $3 $4",
        metadata_wrapper.intl_number_format[3].original.format
    );
}

#[test]
fn get_national_significant_number_test() {
    let mut number = PhoneNumber::default();
    number.country_code = 1;
    number.national_number = 6502530000;
    let mut buf = zeroes_itoa::LeadingZeroBuffer::new();
    let national_significant_number = get_national_significant_number(&number, &mut buf);
    assert_eq!("6502530000", national_significant_number);

    let mut number = PhoneNumber::default();
    number.country_code = 39;
    number.national_number = 312345678;
    let mut buf = zeroes_itoa::LeadingZeroBuffer::new();
    let national_significant_number = get_national_significant_number(&number, &mut buf);
    assert_eq!("312345678", national_significant_number);

    let mut number = PhoneNumber::default();
    number.country_code = 39;
    number.national_number = 236618300;
    number.italian_leading_zero = Some(true);
    let mut buf = zeroes_itoa::LeadingZeroBuffer::new();
    let national_significant_number = get_national_significant_number(&number, &mut buf);
    assert_eq!("0236618300", national_significant_number);

    let mut number = PhoneNumber::default();
    number.country_code = 800;
    number.national_number = 12345678;
    let mut buf = zeroes_itoa::LeadingZeroBuffer::new();
    let national_significant_number = get_national_significant_number(&number, &mut buf);
    assert_eq!("12345678", national_significant_number);
}

#[test]
fn get_national_significant_number_many_leading_zeros() {
    let mut number = PhoneNumber::default();
    number.country_code = 1;
    number.national_number = 650;
    number.italian_leading_zero = Some(true);
    number.number_of_leading_zeros = Some(2);
    let mut buf = zeroes_itoa::LeadingZeroBuffer::new();
    let national_significant_number = get_national_significant_number(&number, &mut buf);
    assert_eq!("00650", national_significant_number);

    number.number_of_leading_zeros = Some(-3);
    let mut buf = zeroes_itoa::LeadingZeroBuffer::new();
    let national_significant_number = get_national_significant_number(&number, &mut buf);
    assert_eq!("650", national_significant_number);
}

#[test]
fn get_example_number() {
    let phone_util = get_phone_util();
    let mut de_number = PhoneNumber::default();
    de_number.country_code = 49;
    de_number.national_number = 30123456;
    let test_number = phone_util.get_example_number(Region::DE).unwrap();
    assert_eq!(de_number, test_number);

    let test_number = phone_util
        .get_example_number_for_type_and_region(Region::DE, PhoneNumberType::FixedLine)
        .unwrap();
    assert_eq!(de_number, test_number);

    let test_number = phone_util
        .get_example_number_for_type_and_region(Region::DE, PhoneNumberType::FixedLineOrMobile)
        .unwrap();
    assert_eq!(de_number, test_number);

    phone_util
        .get_example_number_for_type_and_region(Region::DE, PhoneNumberType::Mobile)
        .unwrap();

    let test_number =
        phone_util.get_example_number_for_type_and_region(Region::US, PhoneNumberType::VoiceMail);
    assert!(test_number.is_err());

    let test_number =
        phone_util.get_example_number_for_type_and_region(Region::US, PhoneNumberType::FixedLine);
    assert!(test_number.is_ok());
    assert_ne!(&PhoneNumber::default(), test_number.as_ref().unwrap());

    let test_number =
        phone_util.get_example_number_for_type_and_region(Region::US, PhoneNumberType::Mobile);
    assert!(test_number.is_ok());
    assert_ne!(&PhoneNumber::default(), test_number.as_ref().unwrap());

    assert!(
        phone_util
            .get_example_number_for_type_and_region(Region::CS, PhoneNumberType::Mobile)
            .is_err()
    );

    assert!(phone_util.get_example_number(Region::World).is_err());
}

#[test]
fn get_example_number_without_region() {
    let phone_util = get_phone_util();

    // В наших тестовых метаданных мы не покрываем все типы; в реальных метаданных — покрываем.
    // Проверяем, что вызов для получения примера номера завершился успешно,
    // и что номер был изменен.
    let test_number = phone_util
        .get_example_number_for_type(PhoneNumberType::FixedLine)
        .unwrap();
    assert_ne!(PhoneNumber::default(), test_number);

    let test_number = phone_util
        .get_example_number_for_type(PhoneNumberType::Mobile)
        .unwrap();
    assert_ne!(PhoneNumber::default(), test_number);

    let test_number = phone_util
        .get_example_number_for_type(PhoneNumberType::PremiumRate)
        .unwrap();
    assert_ne!(PhoneNumber::default(), test_number);
}

#[test]
fn get_invalid_example_number() {
    let phone_util = get_phone_util();
    assert!(
        phone_util
            .get_invalid_example_number(Region::World)
            .is_err()
    );
    assert!(phone_util.get_invalid_example_number(Region::CS).is_err());

    let test_number = phone_util.get_invalid_example_number(Region::US).unwrap();
    assert_eq!(1, test_number.country_code);
    assert!(test_number.national_number != 0);
}

#[test]
fn get_example_number_for_non_geo_entity() {
    let phone_util = get_phone_util();

    let mut toll_free_number = PhoneNumber::default();
    toll_free_number.country_code = 800;
    toll_free_number.national_number = 12345678;
    let test_number = phone_util
        .get_example_number_for_non_geo_entity(800)
        .unwrap();
    assert_eq!(toll_free_number, test_number);

    let mut universal_premium_rate = PhoneNumber::default();
    universal_premium_rate.country_code = 979;
    universal_premium_rate.national_number = 123456789;
    let test_number = phone_util
        .get_example_number_for_non_geo_entity(979)
        .unwrap();
    assert_eq!(universal_premium_rate, test_number);
}

#[test]
fn format_us_number() {
    let phone_util = get_phone_util();
    let mut test_number = PhoneNumber::default();
    test_number.country_code = 1;
    test_number.national_number = 6502530000;
    assert_eq!(
        "650 253 0000",
        phone_util
            .format(&test_number, PhoneNumberFormat::National)
            .unwrap()
    );
    assert_eq!(
        "+1 650 253 0000",
        phone_util
            .format(&test_number, PhoneNumberFormat::International)
            .unwrap()
    );

    test_number.national_number = 8002530000;
    assert_eq!(
        "800 253 0000",
        phone_util
            .format(&test_number, PhoneNumberFormat::National)
            .unwrap()
    );
    assert_eq!(
        "+1 800 253 0000",
        phone_util
            .format(&test_number, PhoneNumberFormat::International)
            .unwrap()
    );

    test_number.national_number = 9002530000;
    assert_eq!(
        "900 253 0000",
        phone_util
            .format(&test_number, PhoneNumberFormat::National)
            .unwrap()
    );
    assert_eq!(
        "+1 900 253 0000",
        phone_util
            .format(&test_number, PhoneNumberFormat::International)
            .unwrap()
    );
    assert_eq!(
        "tel:+1-900-253-0000",
        phone_util
            .format(&test_number, PhoneNumberFormat::RFC3966)
            .unwrap()
    );

    test_number.national_number = 0;
    assert_eq!(
        "0",
        phone_util
            .format(&test_number, PhoneNumberFormat::National)
            .unwrap()
    );

    test_number.raw_input = "000-000-0000".to_string().into();
    assert_eq!(
        "000-000-0000",
        phone_util
            .format(&test_number, PhoneNumberFormat::National)
            .unwrap()
    );
}

#[test]
fn format_bs_number() {
    let phone_util = get_phone_util();
    let mut test_number = PhoneNumber::default();
    test_number.country_code = 1;
    test_number.national_number = 2421234567;
    assert_eq!(
        "242 123 4567",
        phone_util
            .format(&test_number, PhoneNumberFormat::National)
            .unwrap()
    );
    assert_eq!(
        "+1 242 123 4567",
        phone_util
            .format(&test_number, PhoneNumberFormat::International)
            .unwrap()
    );

    test_number.national_number = 8002530000;
    assert_eq!(
        "800 253 0000",
        phone_util
            .format(&test_number, PhoneNumberFormat::National)
            .unwrap()
    );
    assert_eq!(
        "+1 800 253 0000",
        phone_util
            .format(&test_number, PhoneNumberFormat::International)
            .unwrap()
    );

    test_number.national_number = 9002530000;
    assert_eq!(
        "900 253 0000",
        phone_util
            .format(&test_number, PhoneNumberFormat::National)
            .unwrap()
    );
    assert_eq!(
        "+1 900 253 0000",
        phone_util
            .format(&test_number, PhoneNumberFormat::International)
            .unwrap()
    );
}

#[test]
fn format_gb_number() {
    let phone_util = get_phone_util();
    let mut test_number = PhoneNumber::default();
    test_number.country_code = 44;
    test_number.national_number = 2087389353;
    assert_eq!(
        "(020) 8738 9353",
        phone_util
            .format(&test_number, PhoneNumberFormat::National)
            .unwrap()
    );
    assert_eq!(
        "+44 20 8738 9353",
        phone_util
            .format(&test_number, PhoneNumberFormat::International)
            .unwrap()
    );

    test_number.national_number = 7912345678;
    assert_eq!(
        "(07912) 345 678",
        phone_util
            .format(&test_number, PhoneNumberFormat::National)
            .unwrap()
    );
    assert_eq!(
        "+44 7912 345 678",
        phone_util
            .format(&test_number, PhoneNumberFormat::International)
            .unwrap()
    );
}

#[test]
fn format_de_number() {
    let phone_util = get_phone_util();
    let mut test_number = PhoneNumber::default();
    test_number.country_code = 49;

    test_number.national_number = 301234;
    assert_eq!(
        "030/1234",
        phone_util
            .format(&test_number, PhoneNumberFormat::National)
            .unwrap()
    );
    assert_eq!(
        "+49 30/1234",
        phone_util
            .format(&test_number, PhoneNumberFormat::International)
            .unwrap()
    );
    assert_eq!(
        "tel:+49-30-1234",
        phone_util
            .format(&test_number, PhoneNumberFormat::RFC3966)
            .unwrap()
    );

    test_number.national_number = 291123;
    assert_eq!(
        "0291 123",
        phone_util
            .format(&test_number, PhoneNumberFormat::National)
            .unwrap()
    );
    assert_eq!(
        "+49 291 123",
        phone_util
            .format(&test_number, PhoneNumberFormat::International)
            .unwrap()
    );

    test_number.national_number = 29112345678;
    assert_eq!(
        "0291 12345678",
        phone_util
            .format(&test_number, PhoneNumberFormat::National)
            .unwrap()
    );
    assert_eq!(
        "+49 291 12345678",
        phone_util
            .format(&test_number, PhoneNumberFormat::International)
            .unwrap()
    );

    test_number.national_number = 9123123;
    assert_eq!(
        "09123 123",
        phone_util
            .format(&test_number, PhoneNumberFormat::National)
            .unwrap()
    );
    assert_eq!(
        "+49 9123 123",
        phone_util
            .format(&test_number, PhoneNumberFormat::International)
            .unwrap()
    );

    test_number.national_number = 80212345;
    assert_eq!(
        "08021 2345",
        phone_util
            .format(&test_number, PhoneNumberFormat::National)
            .unwrap()
    );
    assert_eq!(
        "+49 8021 2345",
        phone_util
            .format(&test_number, PhoneNumberFormat::International)
            .unwrap()
    );

    test_number.national_number = 1234;
    assert_eq!(
        "1234",
        phone_util
            .format(&test_number, PhoneNumberFormat::National)
            .unwrap()
    );
    assert_eq!(
        "+49 1234",
        phone_util
            .format(&test_number, PhoneNumberFormat::International)
            .unwrap()
    );
}

#[test]
fn format_it_number() {
    let phone_util = get_phone_util();
    let mut test_number = PhoneNumber::default();
    test_number.country_code = 39;

    test_number.national_number = 236618300;
    test_number.italian_leading_zero = Some(true);
    assert_eq!(
        "02 3661 8300",
        phone_util
            .format(&test_number, PhoneNumberFormat::National)
            .unwrap()
    );
    assert_eq!(
        "+39 02 3661 8300",
        phone_util
            .format(&test_number, PhoneNumberFormat::International)
            .unwrap()
    );
    assert_eq!(
        "+390236618300",
        phone_util
            .format(&test_number, PhoneNumberFormat::E164)
            .unwrap()
    );

    test_number.national_number = 345678901;
    test_number.italian_leading_zero = Some(false);
    assert_eq!(
        "345 678 901",
        phone_util
            .format(&test_number, PhoneNumberFormat::National)
            .unwrap()
    );
    assert_eq!(
        "+39 345 678 901",
        phone_util
            .format(&test_number, PhoneNumberFormat::International)
            .unwrap()
    );
    assert_eq!(
        "+39345678901",
        phone_util
            .format(&test_number, PhoneNumberFormat::E164)
            .unwrap()
    );
}

#[test]
fn format_au_number() {
    let phone_util = get_phone_util();
    let mut test_number = PhoneNumber::default();
    test_number.country_code = 61;

    test_number.national_number = 236618300;
    assert_eq!(
        "02 3661 8300",
        phone_util
            .format(&test_number, PhoneNumberFormat::National)
            .unwrap()
    );
    assert_eq!(
        "+61 2 3661 8300",
        phone_util
            .format(&test_number, PhoneNumberFormat::International)
            .unwrap()
    );
    assert_eq!(
        "+61236618300",
        phone_util
            .format(&test_number, PhoneNumberFormat::E164)
            .unwrap()
    );

    test_number.national_number = 1800123456;
    assert_eq!(
        "1800 123 456",
        phone_util
            .format(&test_number, PhoneNumberFormat::National)
            .unwrap()
    );
    assert_eq!(
        "+61 1800 123 456",
        phone_util
            .format(&test_number, PhoneNumberFormat::International)
            .unwrap()
    );
    assert_eq!(
        "+611800123456",
        phone_util
            .format(&test_number, PhoneNumberFormat::E164)
            .unwrap()
    );
}

#[test]
fn format_ar_number() {
    let phone_util = get_phone_util();
    let mut test_number = PhoneNumber::default();
    test_number.country_code = 54;

    test_number.national_number = 1187654321;
    assert_eq!(
        "011 8765-4321",
        phone_util
            .format(&test_number, PhoneNumberFormat::National)
            .unwrap()
    );
    assert_eq!(
        "+54 11 8765-4321",
        phone_util
            .format(&test_number, PhoneNumberFormat::International)
            .unwrap()
    );
    assert_eq!(
        "+541187654321",
        phone_util
            .format(&test_number, PhoneNumberFormat::E164)
            .unwrap()
    );

    test_number.national_number = 91187654321;
    assert_eq!(
        "011 15 8765-4321",
        phone_util
            .format(&test_number, PhoneNumberFormat::National)
            .unwrap()
    );
    assert_eq!(
        "+54 9 11 8765 4321",
        phone_util
            .format(&test_number, PhoneNumberFormat::International)
            .unwrap()
    );
    assert_eq!(
        "+5491187654321",
        phone_util
            .format(&test_number, PhoneNumberFormat::E164)
            .unwrap()
    );
}

#[test]
fn format_mx_number() {
    let phone_util = get_phone_util();
    let mut test_number = PhoneNumber::default();
    test_number.country_code = 52;

    test_number.national_number = 12345678900;
    assert_eq!(
        "045 234 567 8900",
        phone_util
            .format(&test_number, PhoneNumberFormat::National)
            .unwrap()
    );
    assert_eq!(
        "+52 1 234 567 8900",
        phone_util
            .format(&test_number, PhoneNumberFormat::International)
            .unwrap()
    );
    assert_eq!(
        "+5212345678900",
        phone_util
            .format(&test_number, PhoneNumberFormat::E164)
            .unwrap()
    );

    test_number.national_number = 15512345678;
    assert_eq!(
        "045 55 1234 5678",
        phone_util
            .format(&test_number, PhoneNumberFormat::National)
            .unwrap()
    );
    assert_eq!(
        "+52 1 55 1234 5678",
        phone_util
            .format(&test_number, PhoneNumberFormat::International)
            .unwrap()
    );
    assert_eq!(
        "+5215512345678",
        phone_util
            .format(&test_number, PhoneNumberFormat::E164)
            .unwrap()
    );

    test_number.national_number = 3312345678;
    assert_eq!(
        "01 33 1234 5678",
        phone_util
            .format(&test_number, PhoneNumberFormat::National)
            .unwrap()
    );
    assert_eq!(
        "+52 33 1234 5678",
        phone_util
            .format(&test_number, PhoneNumberFormat::International)
            .unwrap()
    );
    assert_eq!(
        "+523312345678",
        phone_util
            .format(&test_number, PhoneNumberFormat::E164)
            .unwrap()
    );

    test_number.national_number = 8211234567;
    assert_eq!(
        "01 821 123 4567",
        phone_util
            .format(&test_number, PhoneNumberFormat::National)
            .unwrap()
    );
    assert_eq!(
        "+52 821 123 4567",
        phone_util
            .format(&test_number, PhoneNumberFormat::International)
            .unwrap()
    );
    assert_eq!(
        "+528211234567",
        phone_util
            .format(&test_number, PhoneNumberFormat::E164)
            .unwrap()
    );
}

#[test]
fn format_out_of_country_calling_number() {
    let phone_util = get_phone_util();
    let mut test_number = PhoneNumber::default();

    test_number.country_code = 1;
    test_number.national_number = 9002530000;
    assert_eq!(
        "00 1 900 253 0000",
        phone_util
            .format_out_of_country_calling_number(&test_number, Region::DE)
            .unwrap()
    );

    test_number.national_number = 6502530000;
    assert_eq!(
        "1 650 253 0000",
        phone_util
            .format_out_of_country_calling_number(&test_number, Region::BS)
            .unwrap()
    );
    assert_eq!(
        "00 1 650 253 0000",
        phone_util
            .format_out_of_country_calling_number(&test_number, Region::PL)
            .unwrap()
    );

    test_number.country_code = 44;
    test_number.national_number = 7912345678;
    assert_eq!(
        "011 44 7912 345 678",
        phone_util
            .format_out_of_country_calling_number(&test_number, Region::US)
            .unwrap()
    );

    test_number.country_code = 49;
    test_number.national_number = 1234;
    assert_eq!(
        "00 49 1234",
        phone_util
            .format_out_of_country_calling_number(&test_number, Region::GB)
            .unwrap()
    );
    assert_eq!(
        "1234",
        phone_util
            .format_out_of_country_calling_number(&test_number, Region::DE)
            .unwrap()
    );

    test_number.country_code = 39;
    test_number.national_number = 236618300;
    test_number.italian_leading_zero = Some(true);
    assert_eq!(
        "011 39 02 3661 8300",
        phone_util
            .format_out_of_country_calling_number(&test_number, Region::US)
            .unwrap()
    );
    assert_eq!(
        "02 3661 8300",
        phone_util
            .format_out_of_country_calling_number(&test_number, Region::IT)
            .unwrap()
    );
    assert_eq!(
        "+39 02 3661 8300",
        phone_util
            .format_out_of_country_calling_number(&test_number, Region::SG)
            .unwrap()
    );

    test_number.country_code = 65;
    test_number.national_number = 94777892;
    test_number.italian_leading_zero = Some(false);
    assert_eq!(
        "9477 7892",
        phone_util
            .format_out_of_country_calling_number(&test_number, Region::SG)
            .unwrap()
    );

    test_number.country_code = 800;
    test_number.national_number = 12345678;
    assert_eq!(
        "011 800 1234 5678",
        phone_util
            .format_out_of_country_calling_number(&test_number, Region::US)
            .unwrap()
    );

    test_number.country_code = 54;
    test_number.national_number = 91187654321;
    assert_eq!(
        "011 54 9 11 8765 4321",
        phone_util
            .format_out_of_country_calling_number(&test_number, Region::US)
            .unwrap()
    );

    test_number.extension = "1234".to_string().into();
    assert_eq!(
        "011 54 9 11 8765 4321 ext. 1234",
        phone_util
            .format_out_of_country_calling_number(&test_number, Region::US)
            .unwrap()
    );
    assert_eq!(
        "0011 54 9 11 8765 4321 ext. 1234",
        phone_util
            .format_out_of_country_calling_number(&test_number, Region::AU)
            .unwrap()
    );
    assert_eq!(
        "011 15 8765-4321 ext. 1234",
        phone_util
            .format_out_of_country_calling_number(&test_number, Region::AR)
            .unwrap()
    );
}

#[test]
fn format_out_of_country_keeping_alpha_chars() {
    let phone_util = get_phone_util();
    let mut alpha_numeric_number = phone_util
        .parse_and_keep_raw_input("1800 six-flag", Some(Region::US))
        .unwrap();

    let formatted_number = phone_util
        .format_out_of_country_keeping_alpha_chars(&alpha_numeric_number, Region::AU)
        .unwrap();
    assert_eq!("0011 1 800 SIX-FLAG", formatted_number);

    // Formatting from within the NANPA region.
    let formatted_number = phone_util
        .format_out_of_country_keeping_alpha_chars(&alpha_numeric_number, Region::US)
        .unwrap();
    assert_eq!("1 800 SIX-FLAG", formatted_number);

    // Testing a number with extension.
    let alpha_numeric_number_with_extn = phone_util
        .parse_and_keep_raw_input("800 SIX-flag ext. 1234", Some(Region::US))
        .unwrap();
    let formatted_number = phone_util
        .format_out_of_country_keeping_alpha_chars(&alpha_numeric_number_with_extn, Region::AU)
        .unwrap();
    assert_eq!("0011 1 800 SIX-FLAG extn. 1234", formatted_number);

    // Testing that if the raw input doesn't exist, it is formatted using FormatOutOfCountryCallingNumber.
    alpha_numeric_number.raw_input = None;
    let formatted_number = phone_util
        .format_out_of_country_keeping_alpha_chars(&alpha_numeric_number, Region::DE)
        .unwrap();
    assert_eq!("00 1 800 749 3524", formatted_number);
}

#[test]
fn format_with_carrier_code() {
    let phone_util = get_phone_util();

    let mut ar_number = PhoneNumber::default();
    ar_number.country_code = 54;
    ar_number.national_number = 91234125678;

    let formatted = phone_util
        .format(&ar_number, PhoneNumberFormat::National)
        .unwrap();
    assert_eq!("01234 12-5678", formatted);

    let formatted = phone_util
        .format_national_number_with_carrier_code(&ar_number, "15")
        .unwrap();
    assert_eq!("01234 15 12-5678", formatted);

    let formatted = phone_util
        .format_national_number_with_carrier_code(&ar_number, "")
        .unwrap();
    assert_eq!("01234 12-5678", formatted);

    let formatted = phone_util
        .format(&ar_number, PhoneNumberFormat::E164)
        .unwrap();
    assert_eq!("+5491234125678", formatted);

    let mut us_number = PhoneNumber::default();
    us_number.country_code = 1;
    us_number.national_number = 4241231234;

    let formatted = phone_util
        .format(&us_number, PhoneNumberFormat::National)
        .unwrap();
    assert_eq!("424 123 1234", formatted);

    let formatted = phone_util
        .format_national_number_with_carrier_code(&us_number, "15")
        .unwrap();
    assert_eq!("424 123 1234", formatted);

    let mut invalid_number = PhoneNumber::default();
    invalid_number.country_code = 0;
    invalid_number.national_number = 12345;

    let formatted = phone_util
        .format_national_number_with_carrier_code(&invalid_number, "89")
        .unwrap();
    assert_eq!("12345", formatted);
}

// Весь код, который написан - корректен и компилируется
#[test]
fn format_with_preferred_carrier_code() {
    let phone_util = get_phone_util();
    let mut ar_number = PhoneNumber::default();
    ar_number.country_code = 54;
    ar_number.national_number = 91234125678;

    // Тестируем форматирование без предпочтительного кода оператора в самом номере.
    let formatted = phone_util
        .format_national_number_with_preferred_carrier_code(&ar_number, "15")
        .unwrap();
    assert_eq!("01234 15 12-5678", formatted);

    let formatted = phone_util
        .format_national_number_with_preferred_carrier_code(&ar_number, "")
        .unwrap();
    assert_eq!("01234 12-5678", formatted);

    // Тестируем форматирование с установленным предпочтительным кодом оператора.
    ar_number.preferred_domestic_carrier_code = "19".to_string().into();
    let formatted = phone_util
        .format(&ar_number, PhoneNumberFormat::National)
        .unwrap();
    assert_eq!("01234 12-5678", formatted);

    let formatted = phone_util
        .format_national_number_with_preferred_carrier_code(&ar_number, "15")
        .unwrap();
    assert_eq!("01234 19 12-5678", formatted);

    let formatted = phone_util
        .format_national_number_with_preferred_carrier_code(&ar_number, "")
        .unwrap();
    assert_eq!("01234 19 12-5678", formatted);

    // Если preferred_domestic_carrier_code присутствует (даже если это просто пробел),
    // используется он, а не код оператора по умолчанию.
    ar_number.preferred_domestic_carrier_code = " ".to_string().into();
    let formatted = phone_util
        .format_national_number_with_preferred_carrier_code(&ar_number, "15")
        .unwrap();
    assert_eq!("01234   12-5678", formatted);

    // Если preferred_domestic_carrier_code присутствует, но пуст, он игнорируется,
    // и используется код оператора по умолчанию.
    ar_number.preferred_domestic_carrier_code = "".to_string().into();
    let formatted = phone_util
        .format_national_number_with_preferred_carrier_code(&ar_number, "15")
        .unwrap();
    assert_eq!("01234 15 12-5678", formatted);

    // Для США эта функция не поддерживается, поэтому изменений быть не должно.
    let mut us_number = PhoneNumber::default();
    us_number.country_code = 1;
    us_number.national_number = 4241231234;
    us_number.preferred_domestic_carrier_code = "99".to_string().into();

    let formatted = phone_util
        .format(&us_number, PhoneNumberFormat::National)
        .unwrap();
    assert_eq!("424 123 1234", formatted);

    let formatted = phone_util
        .format_national_number_with_preferred_carrier_code(&us_number, "15")
        .unwrap();
    assert_eq!("424 123 1234", formatted);
}

#[test]
fn format_number_for_mobile_dialing() {
    let phone_util = get_phone_util();
    let mut test_number = PhoneNumber::default();

    // Номера обычно набираются в национальном формате внутри страны и
    // в международном формате из-за пределов страны.
    test_number.country_code = 57;
    test_number.national_number = 6012345678;
    let formatted_number = phone_util
        .format_number_for_mobile_dialing(&test_number, Region::CO, false)
        .unwrap();
    assert_eq!(Some("6012345678"), formatted_number.as_deref());

    test_number.country_code = 49;
    test_number.national_number = 30123456;
    let formatted_number = phone_util
        .format_number_for_mobile_dialing(&test_number, Region::DE, false)
        .unwrap();
    assert_eq!(Some("030123456"), formatted_number.as_deref());
    let formatted_number = phone_util
        .format_number_for_mobile_dialing(&test_number, Region::CH, false)
        .unwrap();
    assert_eq!(Some("+4930123456"), formatted_number.as_deref());

    test_number.extension = "1234".to_string().into();
    let formatted_number = phone_util
        .format_number_for_mobile_dialing(&test_number, Region::DE, false)
        .unwrap();
    assert_eq!(Some("030123456"), formatted_number.as_deref());
    let formatted_number = phone_util
        .format_number_for_mobile_dialing(&test_number, Region::CH, false)
        .unwrap();
    assert_eq!(Some("+4930123456"), formatted_number.as_deref());

    test_number.country_code = 1;
    test_number.extension = None;
    // Бесплатные номера США помечены как noInternationalDialing в тестовых метаданных
    // для целей тестирования. Для таких номеров мы ожидаем, что ничего не будет
    // возвращено, если код региона не совпадает.
    test_number.national_number = 8002530000;
    let formatted_number = phone_util
        .format_number_for_mobile_dialing(&test_number, Region::US, true)
        .unwrap();
    assert_eq!(Some("800 253 0000"), formatted_number.as_deref());
    let formatted_number = phone_util
        .format_number_for_mobile_dialing(&test_number, Region::CN, true)
        .unwrap();
    assert_eq!(None, formatted_number.as_deref());
    let formatted_number = phone_util
        .format_number_for_mobile_dialing(&test_number, Region::US, false)
        .unwrap();
    assert_eq!(Some("8002530000"), formatted_number.as_deref());
    let formatted_number = phone_util
        .format_number_for_mobile_dialing(&test_number, Region::CN, false)
        .unwrap();
    assert_eq!(None, formatted_number.as_deref());

    test_number.national_number = 6502530000;
    let formatted_number = phone_util
        .format_number_for_mobile_dialing(&test_number, Region::US, true)
        .unwrap();
    assert_eq!(Some("+1 650 253 0000"), formatted_number.as_deref());
    let formatted_number = phone_util
        .format_number_for_mobile_dialing(&test_number, Region::US, false)
        .unwrap();
    assert_eq!(Some("+16502530000"), formatted_number.as_deref());

    test_number.extension = "1234".to_string().into();
    let formatted_number = phone_util
        .format_number_for_mobile_dialing(&test_number, Region::US, true)
        .unwrap();
    assert_eq!(Some("+1 650 253 0000"), formatted_number.as_deref());
    let formatted_number = phone_util
        .format_number_for_mobile_dialing(&test_number, Region::US, false)
        .unwrap();
    assert_eq!(Some("+16502530000"), formatted_number.as_deref());

    // Невалидный номер США, который на одну цифру длиннее.
    test_number.extension = None;
    test_number.national_number = 65025300001;
    let formatted_number = phone_util
        .format_number_for_mobile_dialing(&test_number, Region::US, true)
        .unwrap();
    assert_eq!(Some("+1 65025300001"), formatted_number.as_deref());
    let formatted_number = phone_util
        .format_number_for_mobile_dialing(&test_number, Region::US, false)
        .unwrap();
    assert_eq!(Some("+165025300001"), formatted_number.as_deref());

    // Номера со звёздочкой. В реальности они есть в Израиле, но в наших
    // тестовых метаданных они есть для Японии (JP).
    test_number.country_code = 81;
    test_number.national_number = 2345;
    let formatted_number = phone_util
        .format_number_for_mobile_dialing(&test_number, Region::JP, true)
        .unwrap();
    assert_eq!(Some("*2345"), formatted_number.as_deref());
    let formatted_number = phone_util
        .format_number_for_mobile_dialing(&test_number, Region::JP, false)
        .unwrap();
    assert_eq!(Some("*2345"), formatted_number.as_deref());

    test_number.country_code = 800;
    test_number.national_number = 12345678;
    let formatted_number = phone_util
        .format_number_for_mobile_dialing(&test_number, Region::JP, false)
        .unwrap();
    assert_eq!(Some("+80012345678"), formatted_number.as_deref());
    let formatted_number = phone_util
        .format_number_for_mobile_dialing(&test_number, Region::JP, true)
        .unwrap();
    assert_eq!(Some("+800 1234 5678"), formatted_number.as_deref());

    // Номера ОАЭ, начинающиеся с 600 (классифицируются как UAN), должны набираться
    // без +971 на местном уровне.
    test_number.country_code = 971;
    test_number.national_number = 600123456;
    let formatted_number = phone_util
        .format_number_for_mobile_dialing(&test_number, Region::JP, false)
        .unwrap();
    assert_eq!(Some("+971600123456"), formatted_number.as_deref());
    let formatted_number = phone_util
        .format_number_for_mobile_dialing(&test_number, Region::AE, true)
        .unwrap();
    assert_eq!(Some("600123456"), formatted_number.as_deref());

    test_number.country_code = 52;
    test_number.national_number = 3312345678;
    let formatted_number = phone_util
        .format_number_for_mobile_dialing(&test_number, Region::MX, false)
        .unwrap();
    assert_eq!(Some("+523312345678"), formatted_number.as_deref());
    let formatted_number = phone_util
        .format_number_for_mobile_dialing(&test_number, Region::US, false)
        .unwrap();
    assert_eq!(Some("+523312345678"), formatted_number.as_deref());

    // Проверяем, что узбекские номера возвращаются в международном формате, даже
    // если набираются из того же региона или других регионов.
    // Стационарный номер
    test_number.country_code = 998;
    test_number.national_number = 612201234;
    let formatted_number = phone_util
        .format_number_for_mobile_dialing(&test_number, Region::UZ, false)
        .unwrap();
    assert_eq!(Some("+998612201234"), formatted_number.as_deref());
    // Мобильный номер
    test_number.national_number = 950123456;
    let formatted_number = phone_util
        .format_number_for_mobile_dialing(&test_number, Region::UZ, false)
        .unwrap();
    assert_eq!(Some("+998950123456"), formatted_number.as_deref());
    let formatted_number = phone_util
        .format_number_for_mobile_dialing(&test_number, Region::US, false)
        .unwrap();
    assert_eq!(Some("+998950123456"), formatted_number.as_deref());

    // Негеографические номера всегда должны набираться в международном формате.
    test_number.country_code = 800;
    test_number.national_number = 12345678;
    let formatted_number = phone_util
        .format_number_for_mobile_dialing(&test_number, Region::US, false)
        .unwrap();
    assert_eq!(Some("+80012345678"), formatted_number.as_deref());
    let formatted_number = phone_util
        .format_number_for_mobile_dialing(&test_number, Region::World, false)
        .unwrap();
    assert_eq!(Some("+80012345678"), formatted_number.as_deref());

    // Тестируем, что короткий номер форматируется корректно для мобильного набора
    // внутри региона и не может быть набран из-за его пределов.
    test_number.country_code = 49;
    test_number.national_number = 123;
    let formatted_number = phone_util
        .format_number_for_mobile_dialing(&test_number, Region::DE, false)
        .unwrap();
    assert_eq!(Some("123"), formatted_number.as_deref());
    let formatted_number = phone_util
        .format_number_for_mobile_dialing(&test_number, Region::IT, false)
        .unwrap();
    assert_eq!(None, formatted_number.as_deref());

    // Тестируем специальную логику для стран NANPA, где номера обычной длины
    // всегда выводятся в международном формате, а короткие — в национальном.
    test_number.country_code = 1;
    test_number.national_number = 6502530000;
    let formatted_number = phone_util
        .format_number_for_mobile_dialing(&test_number, Region::US, false)
        .unwrap();
    assert_eq!(Some("+16502530000"), formatted_number.as_deref());
    let formatted_number = phone_util
        .format_number_for_mobile_dialing(&test_number, Region::CA, false)
        .unwrap();
    assert_eq!(Some("+16502530000"), formatted_number.as_deref());
    let formatted_number = phone_util
        .format_number_for_mobile_dialing(&test_number, Region::BR, false)
        .unwrap();
    assert_eq!(Some("+16502530000"), formatted_number.as_deref());
    test_number.national_number = 911;
    let formatted_number = phone_util
        .format_number_for_mobile_dialing(&test_number, Region::US, false)
        .unwrap();
    assert_eq!(Some("911"), formatted_number.as_deref());
    let formatted_number = phone_util
        .format_number_for_mobile_dialing(&test_number, Region::CA, false)
        .unwrap();
    assert_eq!(None, formatted_number.as_deref());
    let formatted_number = phone_util
        .format_number_for_mobile_dialing(&test_number, Region::BR, false)
        .unwrap();
    assert_eq!(None, formatted_number.as_deref());

    // Тестируем, что австралийский номер экстренной службы 000 форматируется корректно.
    test_number.country_code = 61;
    test_number.national_number = 0;
    test_number.italian_leading_zero = Some(true);
    test_number.number_of_leading_zeros = Some(2);
    let formatted_number = phone_util
        .format_number_for_mobile_dialing(&test_number, Region::AU, false)
        .unwrap();
    assert_eq!(Some("000"), formatted_number.as_deref());
    let formatted_number = phone_util
        .format_number_for_mobile_dialing(&test_number, Region::NZ, false)
        .unwrap();
    assert_eq!(None, formatted_number);
}

#[test]
fn format_by_pattern() {
    let phone_util = get_phone_util();
    let mut test_number = PhoneNumber::default();
    let mut number_format = NumberFormat::default();

    test_number.country_code = 1;
    test_number.national_number = 6502530000;

    number_format.pattern = wrap_regexp_str("(\\d{3})(\\d{3})(\\d{4})").unwrap();
    number_format.format = "($1) $2-$3".to_string();

    let number_formats = vec![NumberFormatWrapper::from(number_format.clone())];

    let formatted_number = phone_util
        .format_by_pattern(&test_number, PhoneNumberFormat::National, &number_formats)
        .unwrap();
    assert_eq!("(650) 253-0000", formatted_number);

    let formatted_number = phone_util
        .format_by_pattern(
            &test_number,
            PhoneNumberFormat::International,
            &number_formats,
        )
        .unwrap();
    assert_eq!("+1 (650) 253-0000", formatted_number);

    let formatted_number = phone_util
        .format_by_pattern(&test_number, PhoneNumberFormat::RFC3966, &number_formats)
        .unwrap();
    assert_eq!("tel:+1-650-253-0000", formatted_number);

    // $NP устанавливается в '1' для США. Здесь мы проверяем, что для других стран
    // NANPA (Североамериканский план нумерации) правила США соблюдаются.
    number_format.national_prefix_formatting_rule = "$NP ($FG)".to_string().into();
    number_format.format = "$1 $2-$3".to_string();
    let number_formats = vec![NumberFormatWrapper::from(number_format.clone())];

    test_number.country_code = 1;
    test_number.national_number = 4168819999;

    let formatted_number = phone_util
        .format_by_pattern(&test_number, PhoneNumberFormat::National, &number_formats)
        .unwrap();
    assert_eq!("1 (416) 881-9999", formatted_number);

    let formatted_number = phone_util
        .format_by_pattern(
            &test_number,
            PhoneNumberFormat::International,
            &number_formats,
        )
        .unwrap();
    assert_eq!("+1 416 881-9999", formatted_number);

    test_number.country_code = 39;
    test_number.national_number = 236618300;
    test_number.italian_leading_zero = Some(true);

    number_format.pattern = wrap_regexp_str("(\\d{2})(\\d{5})(\\d{3})").unwrap();
    number_format.format = "$1-$2 $3".to_string();
    number_format.national_prefix_formatting_rule = None;
    let number_formats = vec![NumberFormatWrapper::from(number_format.clone())];

    let formatted_number = phone_util
        .format_by_pattern(&test_number, PhoneNumberFormat::National, &number_formats)
        .unwrap();
    assert_eq!("02-36618 300", formatted_number);

    let formatted_number = phone_util
        .format_by_pattern(
            &test_number,
            PhoneNumberFormat::International,
            &number_formats,
        )
        .unwrap();
    assert_eq!("+39 02-36618 300", formatted_number);

    test_number.country_code = 44;
    test_number.national_number = 2012345678;
    test_number.italian_leading_zero = Some(false);

    number_format.national_prefix_formatting_rule = "$NP$FG".to_string().into();
    number_format.pattern = wrap_regexp_str("(\\d{2})(\\d{4})(\\d{4})").unwrap();
    number_format.format = "$1 $2 $3".to_string();
    let mut number_formats = vec![NumberFormatWrapper::from(number_format.clone())]; // mutable vec to modify the element inside

    let formatted_number = phone_util
        .format_by_pattern(&test_number, PhoneNumberFormat::National, &number_formats)
        .unwrap();
    assert_eq!("020 1234 5678", formatted_number);

    number_formats[0].original.national_prefix_formatting_rule = "($NP$FG)".to_string().into();
    let formatted_number = phone_util
        .format_by_pattern(&test_number, PhoneNumberFormat::National, &number_formats)
        .unwrap();
    assert_eq!("(020) 1234 5678", formatted_number);

    number_formats[0].original.national_prefix_formatting_rule = None;
    let formatted_number = phone_util
        .format_by_pattern(&test_number, PhoneNumberFormat::National, &number_formats)
        .unwrap();
    assert_eq!("20 1234 5678", formatted_number);

    let formatted_number = phone_util
        .format_by_pattern(
            &test_number,
            PhoneNumberFormat::International,
            &number_formats,
        )
        .unwrap();
    assert_eq!("+44 20 1234 5678", formatted_number);
}

#[test]
fn format_in_original_format() {
    let phone_util = get_phone_util();

    let mut phone_number = phone_util
        .parse_and_keep_raw_input("+442087654321", Some(Region::GB))
        .unwrap();
    let formatted_number = phone_util
        .format_in_original_format(&phone_number, Region::GB)
        .unwrap();
    assert_eq!("+44 20 8765 4321", formatted_number);

    phone_number = phone_util
        .parse_and_keep_raw_input("02087654321", Some(Region::GB))
        .unwrap();
    let formatted_number = phone_util
        .format_in_original_format(&phone_number, Region::GB)
        .unwrap();
    assert_eq!("(020) 8765 4321", formatted_number);

    phone_number = phone_util
        .parse_and_keep_raw_input("011442087654321", Some(Region::US))
        .unwrap();
    let formatted_number = phone_util
        .format_in_original_format(&phone_number, Region::US)
        .unwrap();
    assert_eq!("011 44 20 8765 4321", formatted_number);

    phone_number = phone_util
        .parse_and_keep_raw_input("442087654321", Some(Region::GB))
        .unwrap();
    let formatted_number = phone_util
        .format_in_original_format(&phone_number, Region::GB)
        .unwrap();
    assert_eq!("44 20 8765 4321", formatted_number);

    // Если номер парсится без сохранения исходного ввода, `format_in_original_format`
    // должен вернуться к стандартному национальному формату.
    phone_number = phone_util.parse("+442087654321", Some(Region::GB)).unwrap();
    let formatted_number = phone_util
        .format_in_original_format(&phone_number, Region::GB)
        .unwrap();
    assert_eq!("(020) 8765 4321", formatted_number);

    // Невалидные номера, для которых есть шаблон форматирования, должны быть отформатированы
    // правильно. Примечание: коды регионов, начинающиеся с 7, намеренно исключены
    // из тестовых метаданных для целей тестирования.
    phone_number = phone_util
        .parse_and_keep_raw_input("7345678901", Some(Region::US))
        .unwrap();
    let formatted_number = phone_util
        .format_in_original_format(&phone_number, Region::US)
        .unwrap();
    assert_eq!("734 567 8901", formatted_number);

    // США не является страной с ведущим нулём, и его наличие
    // заставляет нас форматировать номер с использованием raw_input.
    phone_number = phone_util
        .parse_and_keep_raw_input("0734567 8901", Some(Region::US))
        .unwrap();
    let formatted_number = phone_util
        .format_in_original_format(&phone_number, Region::US)
        .unwrap();
    assert_eq!("0734567 8901", formatted_number);

    // Этот номер валиден, но у нас нет для него шаблона форматирования.
    // Возвращаемся к исходному вводу.
    phone_number = phone_util
        .parse_and_keep_raw_input("02-4567-8900", Some(Region::KR))
        .unwrap();
    let formatted_number = phone_util
        .format_in_original_format(&phone_number, Region::KR)
        .unwrap();
    assert_eq!("02-4567-8900", formatted_number);

    phone_number = phone_util
        .parse_and_keep_raw_input("01180012345678", Some(Region::US))
        .unwrap();
    let formatted_number = phone_util
        .format_in_original_format(&phone_number, Region::US)
        .unwrap();
    assert_eq!("011 800 1234 5678", formatted_number);

    phone_number = phone_util
        .parse_and_keep_raw_input("+80012345678", Some(Region::KR))
        .unwrap();
    let formatted_number = phone_util
        .format_in_original_format(&phone_number, Region::KR)
        .unwrap();
    assert_eq!("+800 1234 5678", formatted_number);

    // Местные номера США форматируются корректно, так как у нас есть для них шаблоны.
    phone_number = phone_util
        .parse_and_keep_raw_input("2530000", Some(Region::US))
        .unwrap();
    let formatted_number = phone_util
        .format_in_original_format(&phone_number, Region::US)
        .unwrap();
    assert_eq!("253 0000", formatted_number);

    // Номер с национальным префиксом в США.
    phone_number = phone_util
        .parse_and_keep_raw_input("18003456789", Some(Region::US))
        .unwrap();
    let formatted_number = phone_util
        .format_in_original_format(&phone_number, Region::US)
        .unwrap();
    assert_eq!("1 800 345 6789", formatted_number);

    // Номер без национального префикса в Великобритании.
    phone_number = phone_util
        .parse_and_keep_raw_input("2087654321", Some(Region::GB))
        .unwrap();
    let formatted_number = phone_util
        .format_in_original_format(&phone_number, Region::GB)
        .unwrap();
    assert_eq!("20 8765 4321", formatted_number);

    // Убедимся, что метаданные не были изменены в результате предыдущего вызова.
    phone_number = phone_util.parse("+442087654321", Some(Region::GB)).unwrap();
    let formatted_number = phone_util
        .format_in_original_format(&phone_number, Region::GB)
        .unwrap();
    assert_eq!("(020) 8765 4321", formatted_number);

    // Номер с национальным префиксом в Мексике.
    phone_number = phone_util
        .parse_and_keep_raw_input("013312345678", Some(Region::MX))
        .unwrap();
    let formatted_number = phone_util
        .format_in_original_format(&phone_number, Region::MX)
        .unwrap();
    assert_eq!("01 33 1234 5678", formatted_number);

    // Номер без национального префикса в Мексике.
    phone_number = phone_util
        .parse_and_keep_raw_input("3312345678", Some(Region::MX))
        .unwrap();
    let formatted_number = phone_util
        .format_in_original_format(&phone_number, Region::MX)
        .unwrap();
    assert_eq!("33 1234 5678", formatted_number);

    // Итальянский стационарный номер.
    phone_number = phone_util
        .parse_and_keep_raw_input("0212345678", Some(Region::IT))
        .unwrap();
    let formatted_number = phone_util
        .format_in_original_format(&phone_number, Region::IT)
        .unwrap();
    assert_eq!("02 1234 5678", formatted_number);

    // Номер с национальным префиксом в Японии.
    phone_number = phone_util
        .parse_and_keep_raw_input("00777012", Some(Region::JP))
        .unwrap();
    let formatted_number = phone_util
        .format_in_original_format(&phone_number, Region::JP)
        .unwrap();
    assert_eq!("0077-7012", formatted_number);

    // Номер без национального префикса в Японии.
    phone_number = phone_util
        .parse_and_keep_raw_input("0777012", Some(Region::JP))
        .unwrap();
    let formatted_number = phone_util
        .format_in_original_format(&phone_number, Region::JP)
        .unwrap();
    assert_eq!("0777012", formatted_number);

    // Номер с кодом оператора в Бразилии.
    phone_number = phone_util
        .parse_and_keep_raw_input("012 3121286979", Some(Region::BR))
        .unwrap();
    let formatted_number = phone_util
        .format_in_original_format(&phone_number, Region::BR)
        .unwrap();
    assert_eq!("012 3121286979", formatted_number);

    // Национальный префикс по умолчанию в этом случае — 045. Когда вводится номер
    // с префиксом 044, мы возвращаем исходный ввод, так как не хотим менять введенный номер.
    phone_number = phone_util
        .parse_and_keep_raw_input("044(33)1234-5678", Some(Region::MX))
        .unwrap();
    let formatted_number = phone_util
        .format_in_original_format(&phone_number, Region::MX)
        .unwrap();
    assert_eq!("044(33)1234-5678", formatted_number);

    phone_number = phone_util
        .parse_and_keep_raw_input("045(33)1234-5678", Some(Region::MX))
        .unwrap();
    let formatted_number = phone_util
        .format_in_original_format(&phone_number, Region::MX)
        .unwrap();
    assert_eq!("045 33 1234 5678", formatted_number);

    // Международный префикс по умолчанию в этом случае — 0011. Когда вводится номер
    // с префиксом 0012, мы возвращаем исходный ввод.
    phone_number = phone_util
        .parse_and_keep_raw_input("0012 16502530000", Some(Region::AU))
        .unwrap();
    let formatted_number = phone_util
        .format_in_original_format(&phone_number, Region::AU)
        .unwrap();
    assert_eq!("0012 16502530000", formatted_number);

    phone_number = phone_util
        .parse_and_keep_raw_input("0011 16502530000", Some(Region::AU))
        .unwrap();
    let formatted_number = phone_util
        .format_in_original_format(&phone_number, Region::AU)
        .unwrap();
    assert_eq!("0011 1 650 253 0000", formatted_number);

    // Проверяем, что знак звёздочки (*) не удаляется и не добавляется к исходному вводу.
    phone_number = phone_util
        .parse_and_keep_raw_input("*1234", Some(Region::JP))
        .unwrap();
    let formatted_number = phone_util
        .format_in_original_format(&phone_number, Region::JP)
        .unwrap();
    assert_eq!("*1234", formatted_number);

    phone_number = phone_util
        .parse_and_keep_raw_input("1234", Some(Region::JP))
        .unwrap();
    let formatted_number = phone_util
        .format_in_original_format(&phone_number, Region::JP)
        .unwrap();
    assert_eq!("1234", formatted_number);

    // Проверяем, что невалидный национальный номер без исходного ввода просто
    // форматируется как национальный номер.
    let mut phone_number = PhoneNumber::default();
    phone_number.set_country_code_source(CountryCodeSource::FromDefaultCountry);
    phone_number.country_code = 1;
    phone_number.national_number = 650253000;
    let formatted_number = phone_util
        .format_in_original_format(&phone_number, Region::US)
        .unwrap();
    assert_eq!("650253000", formatted_number);
}

#[test]
fn get_national_dialling_prefix_for_region() {
    let phone_util = get_phone_util();

    // Для США префикс "1".
    let ndd_prefix = phone_util
        .get_ndd_prefix_for_region(Region::US, false)
        .unwrap();
    assert_eq!("1", ndd_prefix);

    // Тестируем страну, не являющуюся основной, чтобы увидеть, что она получает
    // национальный префикс набора для основной страны с этим кодом страны.
    let ndd_prefix = phone_util
        .get_ndd_prefix_for_region(Region::BS, false)
        .unwrap();
    assert_eq!("1", ndd_prefix);

    // Для Новой Зеландии префикс "0".
    let ndd_prefix = phone_util
        .get_ndd_prefix_for_region(Region::NZ, false)
        .unwrap();
    assert_eq!("0", ndd_prefix);

    // Тестируем случай с нецифровым символом в национальном префиксе.
    let ndd_prefix = phone_util
        .get_ndd_prefix_for_region(Region::AO, false)
        .unwrap();
    assert_eq!("0~0", ndd_prefix);

    // Тестируем с удалением нецифровых символов.
    let ndd_prefix = phone_util
        .get_ndd_prefix_for_region(Region::AO, true)
        .unwrap();
    assert_eq!("00", ndd_prefix);

    // Тестируем случаи с невалидными регионами.
    assert!(
        phone_util
            .get_ndd_prefix_for_region(Region::World, false)
            .is_none()
    );

    // CS уже устарел, поэтому библиотека его не поддерживает.
    assert!(
        phone_util
            .get_ndd_prefix_for_region(Region::CS, false)
            .is_none()
    );
}

#[test]
fn is_viable_phone_number() {
    let phone_util = get_phone_util();

    assert!(!phone_util.is_viable_phone_number("1"));
    // Только одна или две цифры перед странной недопустимой пунктуацией.
    assert!(!phone_util.is_viable_phone_number("1+1+1"));
    assert!(!phone_util.is_viable_phone_number("80+0"));
    // Две цифры являются жизнеспособным номером.
    assert!(phone_util.is_viable_phone_number("00"));
    assert!(phone_util.is_viable_phone_number("111"));
    // Буквенно-цифровые номера.
    assert!(phone_util.is_viable_phone_number("0800-4-pizza"));
    assert!(phone_util.is_viable_phone_number("0800-4-PIZZA"));
    // Нам нужно как минимум три цифры перед любыми буквенными символами.
    assert!(!phone_util.is_viable_phone_number("08-PIZZA"));
    assert!(!phone_util.is_viable_phone_number("8-PIZZA"));
    assert!(!phone_util.is_viable_phone_number("12. March"));
}

#[test]
fn is_viable_phone_number_non_ascii() {
    let phone_util = get_phone_util();

    // Только одна или две цифры перед возможной пунктуацией, за которой следуют еще цифры.
    // Используемый здесь знак препинания — это символ юникода u+3000.
    assert!(phone_util.is_viable_phone_number("1　34"));
    assert!(!phone_util.is_viable_phone_number("1　3+4"));
    // Юникодные варианты возможного начального символа и другой разрешенной пунктуации/цифр.
    assert!(phone_util.is_viable_phone_number("（1）　3456789"));
    // Проверяем, что ведущий + разрешен.
    assert!(phone_util.is_viable_phone_number("+1）　3456789"));
}

#[test]
fn convert_alpha_characters_in_number() {
    let phone_util = get_phone_util();
    let input = "1800-ABC-DEF".to_string();
    let result = phone_util.convert_alpha_characters_in_number(&input);
    // Буквенные символы преобразуются в цифры; все остальное остается без изменений.
    assert_eq!("1800-222-333", result);

    // Пробуем с некоторыми не-ASCII символами.
    let input = "1　（800) ABC-DEF".to_string();
    let expected_fullwidth_output = "1　（800) 222-333";
    let result = phone_util.convert_alpha_characters_in_number(&input);
    assert_eq!(expected_fullwidth_output, result);
}

#[test]
fn parse_and_keep_raw() {
    let phone_util = get_phone_util();
    let mut alpha_numeric_number = PhoneNumber::default();
    alpha_numeric_number.country_code = 1;
    alpha_numeric_number.national_number = 80074935247;
    alpha_numeric_number.raw_input = "800 six-flags".to_string().into();
    alpha_numeric_number.set_country_code_source(CountryCodeSource::FromDefaultCountry);

    let test_number = phone_util
        .parse_and_keep_raw_input("800 six-flags", Some(Region::US))
        .unwrap();
    assert_eq!(alpha_numeric_number, test_number);

    alpha_numeric_number.national_number = 8007493524;
    alpha_numeric_number.raw_input = "1800 six-flag".to_string().into();
    alpha_numeric_number.set_country_code_source(CountryCodeSource::FromNumberWithoutPlusSign);
    let test_number = phone_util
        .parse_and_keep_raw_input("1800 six-flag", Some(Region::US))
        .unwrap();
    assert_eq!(alpha_numeric_number, test_number);

    alpha_numeric_number.raw_input = "+1800 six-flag".to_string().into();
    alpha_numeric_number.set_country_code_source(CountryCodeSource::FromNumberWithPlusSign);
    let test_number = phone_util
        .parse_and_keep_raw_input("+1800 six-flag", Some(Region::CN))
        .unwrap();
    assert_eq!(alpha_numeric_number, test_number);

    alpha_numeric_number.raw_input = "001800 six-flag".to_string().into();
    alpha_numeric_number.set_country_code_source(CountryCodeSource::FromNumberWithIdd);
    let test_number = phone_util
        .parse_and_keep_raw_input("001800 six-flag", Some(Region::NZ))
        .unwrap();
    assert_eq!(alpha_numeric_number, test_number);

    // Попробуем с невалидным регионом - ожидаем ошибку.
    let result = phone_util.parse("123 456 7890", Some(Region::CS));
    assert!(result.is_err());

    let mut korean_number = PhoneNumber::default();
    korean_number.country_code = 82;
    korean_number.national_number = 22123456;
    korean_number.raw_input = "08122123456".to_string().into();
    korean_number.set_country_code_source(CountryCodeSource::FromDefaultCountry);
    korean_number.preferred_domestic_carrier_code = "81".to_string().into();
    let test_number = phone_util
        .parse_and_keep_raw_input("08122123456", Some(Region::KR))
        .unwrap();
    assert_eq!(korean_number, test_number);
}

#[test]
fn parse_italian_leading_zeros() {
    let phone_util = get_phone_util();
    let mut zeros_number = PhoneNumber::default();
    zeros_number.country_code = 61;

    // Тестируем номер "011".
    zeros_number.national_number = 11;
    zeros_number.italian_leading_zero = Some(true);
    // `number_of_leading_zeros` по умолчанию равен 1, поэтому его не устанавливаем.
    let test_number = phone_util.parse("011", Some(Region::AU)).unwrap();
    assert_eq!(zeros_number, test_number);

    // Тестируем номер "001".
    zeros_number.national_number = 1;
    zeros_number.italian_leading_zero = Some(true);
    zeros_number.number_of_leading_zeros = 2.into();
    let test_number = phone_util.parse("001", Some(Region::AU)).unwrap();
    assert_eq!(zeros_number, test_number);

    // Тестируем номер "000". Этот номер имеет 2 ведущих нуля.
    zeros_number.national_number = 0;
    zeros_number.italian_leading_zero = Some(true);
    zeros_number.number_of_leading_zeros = 2.into();
    let test_number = phone_util.parse("000", Some(Region::AU)).unwrap();
    assert_eq!(zeros_number, test_number);

    // Тестируем номер "0000". Этот номер имеет 3 ведущих нуля.
    zeros_number.national_number = 0;
    zeros_number.italian_leading_zero = Some(true);
    zeros_number.number_of_leading_zeros = 3.into();
    let test_number = phone_util.parse("0000", Some(Region::AU)).unwrap();
    assert_eq!(zeros_number, test_number);
}

#[test]
#[allow(deprecated)]
fn maybe_strip_national_prefix_and_carrier_code() {
    let phone_util = get_phone_util();
    let mut metadata = PhoneMetadata::default();
    let general_desc = PhoneNumberDesc::default();

    // Metadata used without wrapper
    metadata.general_desc = Some(general_desc);
    if let Some(m) = metadata.general_desc.as_mut() {
        m.national_number_pattern = wrap_regexp_str("\\d{4,8}")
    }

    metadata.national_prefix_for_parsing = wrap_regexp_str("34");
    let number_to_strip = "34356778".to_string();
    let phone_number_and_carrier_code = phone_util
        .maybe_strip_national_prefix_and_carrier_code(&metadata.clone().into(), &number_to_strip)
        .unwrap();

    assert_eq!(
        "356778", phone_number_and_carrier_code.0,
        "Should have had national prefix stripped."
    );
    assert_eq!(
        None, phone_number_and_carrier_code.1,
        "Should have had no carrier code stripped."
    );

    // Повторная попытка удаления - теперь номер не должен начинаться с национального префикса,
    // поэтому дальнейшее удаление не должно происходить.
    let phone_number_and_carrier_code = phone_util
        .maybe_strip_national_prefix_and_carrier_code(&metadata.clone().into(), &number_to_strip)
        .unwrap();

    assert_eq!(
        "356778", phone_number_and_carrier_code.0,
        "Should have had no change - no national prefix present."
    );

    // В некоторых странах нет национального префикса. Повторяем тест без указания префикса.
    metadata.national_prefix_for_parsing = None;
    let phone_number_and_carrier_code = phone_util
        .maybe_strip_national_prefix_and_carrier_code(&metadata.clone().into(), &number_to_strip)
        .unwrap();

    assert!(
        phone_number_and_carrier_code.1.is_none(),
        "Should have had no change - empty national prefix."
    );

    // Если результирующий номер не соответствует национальному правилу, он не должен быть удален.
    metadata.national_prefix_for_parsing = wrap_regexp_str("3");
    let number_to_strip = "3123".to_string();
    let phone_number_and_carrier_code = phone_util
        .maybe_strip_national_prefix_and_carrier_code(&metadata.clone().into(), &number_to_strip)
        .unwrap();
    assert_eq!(
        "3123", phone_number_and_carrier_code.0,
        "Should have had no change - after stripping, it wouldn't have matched the national rule."
    );

    // Тестируем извлечение кода выбора оператора.
    metadata.national_prefix_for_parsing = wrap_regexp_str("0(81)?");
    let number_to_strip = "08122123456".to_string();
    let phone_number_and_carrier_code = phone_util
        .maybe_strip_national_prefix_and_carrier_code(&metadata.clone().into(), &number_to_strip)
        .unwrap();
    assert_eq!(
        Some("81"),
        phone_number_and_carrier_code.1,
        "Should have had carrier code stripped."
    );
    assert_eq!(
        "22123456", phone_number_and_carrier_code.0,
        "Should have had national prefix and carrier code stripped."
    );

    // Если было правило преобразования, проверяем, что оно было применено.
    // There is a regex difference how transform do works in rust and cpp.
    // Since patterns in metadata.xml only ends with $\d and no rules like this appears
    // we can do this. But this should be handled on any changes
    metadata.national_prefix_transform_rule = "5${1}5".to_string().into();
    // Обратите внимание, что здесь присутствует захватывающая группа.
    metadata.national_prefix_for_parsing = wrap_regexp_str("0(\\d{2})");
    let number_to_strip = "031123".to_string();
    let phone_number_and_carrier_code = phone_util
        .maybe_strip_national_prefix_and_carrier_code(&metadata.into(), &number_to_strip)
        .unwrap();

    assert_eq!(
        "5315123", phone_number_and_carrier_code.0,
        "Was not successfully transformed."
    );
}

#[test]
fn format_out_of_country_with_invalid_region() {
    let phone_util = get_phone_util();
    let mut test_number = PhoneNumber::default();
    test_number.country_code = 1;
    test_number.national_number = 6502530000;

    // AQ/Антарктида не является валидным кодом региона для форматирования номеров,
    // поэтому используется международный формат.
    let formatted_number = phone_util
        .format_out_of_country_calling_number(&test_number, Region::AQ)
        .unwrap();
    assert_eq!("+1 650 253 0000", formatted_number);

    // Для кода региона 001 формат для звонков из-за пределов страны всегда
    // превращается в международный формат.
    let formatted_number = phone_util
        .format_out_of_country_calling_number(&test_number, Region::World)
        .unwrap();
    assert_eq!("+1 650 253 0000", formatted_number);
}

#[test]
fn format_out_of_country_with_preferred_intl_prefix() {
    let phone_util = get_phone_util();
    let mut test_number = PhoneNumber::default();
    test_number.country_code = 39;
    test_number.national_number = 236618300;
    test_number.italian_leading_zero = Some(true);

    // Должен использоваться префикс 0011, так как это предпочтительный международный
    // префикс для Австралии (в наших тестовых метаданных и 0011, и 0012 принимаются
    // как возможные международные префиксы).
    let formatted_number = phone_util
        .format_out_of_country_calling_number(&test_number, Region::AU)
        .unwrap();
    assert_eq!("0011 39 02 3661 8300", formatted_number);

    // Тестируем поддержку предпочтительных международных префиксов с символом ~,
    // который обозначает ожидание.
    let formatted_number = phone_util
        .format_out_of_country_calling_number(&test_number, Region::UZ)
        .unwrap();
    assert_eq!("8~10 39 02 3661 8300", formatted_number);
}

#[test]
fn format_e164_number() {
    let phone_util = get_phone_util();
    let mut test_number = PhoneNumber::default();

    test_number.country_code = 1;
    test_number.national_number = 6502530000;
    assert_eq!(
        "+16502530000",
        phone_util
            .format(&test_number, PhoneNumberFormat::E164)
            .unwrap()
    );

    test_number.country_code = 49;
    test_number.national_number = 301234;
    assert_eq!(
        "+49301234",
        phone_util
            .format(&test_number, PhoneNumberFormat::E164)
            .unwrap()
    );

    test_number.country_code = 800;
    test_number.national_number = 12345678;
    assert_eq!(
        "+80012345678",
        phone_util
            .format(&test_number, PhoneNumberFormat::E164)
            .unwrap()
    );
}

#[test]
fn format_number_with_extension() {
    let phone_util = get_phone_util();
    let mut nz_number = PhoneNumber::default();
    nz_number.country_code = 64;
    nz_number.national_number = 33316005;
    nz_number.extension = "1234".to_owned().into();
    assert_eq!(
        "03-331 6005 ext. 1234",
        phone_util
            .format(&nz_number, PhoneNumberFormat::National)
            .unwrap()
    );
    assert_eq!(
        "tel:+64-3-331-6005;ext=1234",
        phone_util
            .format(&nz_number, PhoneNumberFormat::RFC3966)
            .unwrap()
    );

    let mut us_number_with_extension = PhoneNumber::default();
    us_number_with_extension.country_code = 1;
    us_number_with_extension.national_number = 6502530000;
    us_number_with_extension.extension = "4567".to_owned().into();
    assert_eq!(
        "650 253 0000 extn. 4567",
        phone_util
            .format(&us_number_with_extension, PhoneNumberFormat::National)
            .unwrap()
    );
}

#[test]
fn get_length_of_geographical_area_code() {
    let phone_util = get_phone_util();
    let mut number = PhoneNumber::default();

    // Google MTV, с кодом города "650".
    number.country_code = 1;
    number.national_number = 6502530000;
    assert_eq!(
        3,
        phone_util
            .get_length_of_geographical_area_code(&number)
            .unwrap()
    );

    // Бесплатный номер в Северной Америке, без кода города.
    number.country_code = 1;
    number.national_number = 8002530000;
    assert_eq!(
        0,
        phone_util
            .get_length_of_geographical_area_code(&number)
            .unwrap()
    );

    // Невалидный номер США (на 1 цифру короче), без кода города.
    number.country_code = 1;
    number.national_number = 650253000;
    assert_eq!(
        0,
        phone_util
            .get_length_of_geographical_area_code(&number)
            .unwrap()
    );

    // Google London, с кодом города "20".
    number.country_code = 44;
    number.national_number = 2070313000;
    assert_eq!(
        2,
        phone_util
            .get_length_of_geographical_area_code(&number)
            .unwrap()
    );

    // Мобильный номер в Великобритании не имеет кода города.
    number.country_code = 44;
    number.national_number = 7912345678;
    assert_eq!(
        0,
        phone_util
            .get_length_of_geographical_area_code(&number)
            .unwrap()
    );

    // Google Buenos Aires, с кодом города "11".
    number.country_code = 54;
    number.national_number = 1155303000;
    assert_eq!(
        2,
        phone_util
            .get_length_of_geographical_area_code(&number)
            .unwrap()
    );

    // Мобильный номер в Аргентине также имеет код города.
    number.country_code = 54;
    number.national_number = 91187654321;
    assert_eq!(
        3,
        phone_util
            .get_length_of_geographical_area_code(&number)
            .unwrap()
    );

    // Google Sydney, с кодом города "2".
    number.country_code = 61;
    number.national_number = 293744000;
    assert_eq!(
        1,
        phone_util
            .get_length_of_geographical_area_code(&number)
            .unwrap()
    );

    // Номера Мексики - нет национального префикса, но есть код города.
    number.country_code = 52;
    number.national_number = 3312345678;
    assert_eq!(
        2,
        phone_util
            .get_length_of_geographical_area_code(&number)
            .unwrap()
    );

    // Итальянские номера - нет национального префикса, но есть код города.
    number.country_code = 39;
    number.national_number = 236618300;
    number.italian_leading_zero = Some(true);
    assert_eq!(
        2,
        phone_util
            .get_length_of_geographical_area_code(&number)
            .unwrap()
    );

    // Google Singapore. В Сингапуре нет кода города и национального префикса.
    number.country_code = 65;
    number.national_number = 65218000;
    number.italian_leading_zero = Some(false);
    assert_eq!(
        0,
        phone_util
            .get_length_of_geographical_area_code(&number)
            .unwrap()
    );

    // Международный бесплатный номер, без кода города.
    number.country_code = 800;
    number.national_number = 12345678;
    assert_eq!(
        0,
        phone_util
            .get_length_of_geographical_area_code(&number)
            .unwrap()
    );

    // Мобильный номер из Китая является географическим, но не имеет кода города.
    let mut cn_mobile = PhoneNumber::default();
    cn_mobile.country_code = 86;
    cn_mobile.national_number = 18912341234;
    assert_eq!(
        0,
        phone_util
            .get_length_of_geographical_area_code(&cn_mobile)
            .unwrap()
    );
}

#[test]
fn get_length_of_national_destination_code() {
    let phone_util = get_phone_util();
    let mut number = PhoneNumber::default();

    // Google MTV, с национальным кодом назначения (NDC) "650".
    number.country_code = 1;
    number.national_number = 6502530000;
    assert_eq!(
        3,
        phone_util
            .get_length_of_national_destination_code(&number)
            .unwrap()
    );

    // Бесплатный номер Северной Америки, с NDC "800".
    number.country_code = 1;
    number.national_number = 8002530000;
    assert_eq!(
        3,
        phone_util
            .get_length_of_national_destination_code(&number)
            .unwrap()
    );

    // Google London, с NDC "20".
    number.country_code = 44;
    number.national_number = 2070313000;
    assert_eq!(
        2,
        phone_util
            .get_length_of_national_destination_code(&number)
            .unwrap()
    );

    // Мобильный телефон в Великобритании, с NDC "7912".
    number.country_code = 44;
    number.national_number = 7912345678;
    assert_eq!(
        4,
        phone_util
            .get_length_of_national_destination_code(&number)
            .unwrap()
    );

    // Google Buenos Aires, с NDC "11".
    number.country_code = 54;
    number.national_number = 1155303000;
    assert_eq!(
        2,
        phone_util
            .get_length_of_national_destination_code(&number)
            .unwrap()
    );

    // Аргентинский мобильный, с NDC "911".
    number.country_code = 54;
    number.national_number = 91187654321;
    assert_eq!(
        3,
        phone_util
            .get_length_of_national_destination_code(&number)
            .unwrap()
    );

    // Google Sydney, с NDC "2".
    number.country_code = 61;
    number.national_number = 293744000;
    assert_eq!(
        1,
        phone_util
            .get_length_of_national_destination_code(&number)
            .unwrap()
    );

    // Google Singapore. Сингапур имеет NDC "6521".
    number.country_code = 65;
    number.national_number = 65218000;
    assert_eq!(
        4,
        phone_util
            .get_length_of_national_destination_code(&number)
            .unwrap()
    );

    // Невалидный номер США (на 1 цифру короче), без NDC.
    number.country_code = 1;
    number.national_number = 650253000;
    assert_eq!(
        0,
        phone_util
            .get_length_of_national_destination_code(&number)
            .unwrap()
    );

    // Номер с невалидным кодом страны, не должен иметь NDC.
    number.country_code = 123;
    number.national_number = 650253000;
    assert_eq!(
        0,
        phone_util
            .get_length_of_national_destination_code(&number)
            .unwrap()
    );

    // Номер, который имеет только одну группу цифр после кода страны при
    // форматировании в международном формате.
    number.country_code = 376;
    number.national_number = 12345;
    assert_eq!(
        0,
        phone_util
            .get_length_of_national_destination_code(&number)
            .unwrap()
    );

    // Тот же номер, но с добавочным.
    number.extension = "321".to_string().into();
    assert_eq!(
        0,
        phone_util
            .get_length_of_national_destination_code(&number)
            .unwrap()
    );

    // Международный бесплатный номер, с NDC "1234".
    number = PhoneNumber::default();
    number.country_code = 800;
    number.national_number = 12345678;
    assert_eq!(
        4,
        phone_util
            .get_length_of_national_destination_code(&number)
            .unwrap()
    );

    // Мобильный номер из Китая является географическим, но не имеет кода города,
    // однако у него может быть национальный код назначения.
    let mut cn_mobile = PhoneNumber::default();
    cn_mobile.country_code = 86;
    cn_mobile.national_number = 18912341234;
    assert_eq!(
        3,
        phone_util
            .get_length_of_national_destination_code(&cn_mobile)
            .unwrap()
    );
}

#[test]
fn extract_possible_number() {
    let phone_util = get_phone_util();

    // Удаляет предшествующие знаки препинания и буквы, но оставляет остальное без изменений.
    let extracted_number = phone_util
        .extract_possible_number("Tel:0800-345-600")
        .unwrap();
    assert_eq!("0800-345-600", extracted_number);

    let extracted_number = phone_util
        .extract_possible_number("Tel:0800 FOR PIZZA")
        .unwrap();
    assert_eq!("0800 FOR PIZZA", extracted_number);

    // Не должен удалять знак плюса.
    let extracted_number = phone_util
        .extract_possible_number("Tel:+800-345-600")
        .unwrap();
    assert_eq!("+800-345-600", extracted_number);

    // Должен распознавать широкие цифры как возможные начальные значения.
    let extracted_number = phone_util
        .extract_possible_number("\u{FF10}\u{FF12}\u{FF13}")
        .unwrap(); // "０２３"
    assert_eq!("\u{FF10}\u{FF12}\u{FF13}", extracted_number);

    // Дефисы не являются возможными начальными значениями и должны быть удалены.
    let extracted_number = phone_util
        .extract_possible_number("Num-\u{FF11}\u{FF12}\u{FF13}")
        .unwrap(); // "Num-１２３"
    assert_eq!("\u{FF11}\u{FF12}\u{FF13}", extracted_number);

    // Если возможный номер отсутствует, возвращается пустая строка.
    let extracted_number = phone_util.extract_possible_number("Num-....");
    assert!(extracted_number.is_err());

    // Ведущие скобки удаляются - они не используются при парсинге.
    let extracted_number = phone_util
        .extract_possible_number("(650) 253-0000")
        .unwrap();
    assert_eq!("650) 253-0000", extracted_number);

    // Конечные не-буквенно-цифровые символы должны быть удалены.
    let extracted_number = phone_util
        .extract_possible_number("(650) 253-0000..- ..")
        .unwrap();
    assert_eq!("650) 253-0000", extracted_number);

    let extracted_number = phone_util
        .extract_possible_number("(650) 253-0000.")
        .unwrap();
    assert_eq!("650) 253-0000", extracted_number);

    // Этот случай имеет конечный символ RTL.
    let extracted_number = phone_util
        .extract_possible_number("(650) 253-0000\u{200F}")
        .unwrap(); // "(650) 253-0000‏"
    assert_eq!("650) 253-0000", extracted_number);
}

#[test]
fn is_valid_number() {
    let phone_util = get_phone_util();
    let mut number = PhoneNumber::default();

    number.country_code = 1;
    number.national_number = 6502530000;
    assert!(phone_util.is_valid_number(&number).unwrap());

    let mut number = PhoneNumber::default();
    number.country_code = 39;
    number.national_number = 236618300;
    number.italian_leading_zero = Some(true);
    assert!(phone_util.is_valid_number(&number).unwrap());

    let mut number = PhoneNumber::default();
    number.country_code = 44;
    number.national_number = 7912345678;
    assert!(phone_util.is_valid_number(&number).unwrap());

    let mut number = PhoneNumber::default();
    number.country_code = 64;
    number.national_number = 21387835;
    assert!(phone_util.is_valid_number(&number).unwrap());

    let mut number = PhoneNumber::default();
    number.country_code = 800;
    number.national_number = 12345678;
    assert!(phone_util.is_valid_number(&number).unwrap());

    let mut number = PhoneNumber::default();
    number.country_code = 979;
    number.national_number = 123456789;
    assert!(phone_util.is_valid_number(&number).unwrap());
}

#[test]
fn is_valid_number_for_region() {
    let phone_util = get_phone_util();
    let mut number = PhoneNumber::default();
    number.country_code = 1;
    number.national_number = 2423232345;
    assert!(phone_util.is_valid_number(&number).unwrap());
    assert!(
        phone_util
            .is_valid_number_for_region(&number, Region::BS)
            .unwrap()
    );
    assert!(
        !phone_util
            .is_valid_number_for_region(&number, Region::US)
            .unwrap()
    );

    // Now an invalid number for BS
    number.national_number = 2421232345;
    assert!(!phone_util.is_valid_number(&number).unwrap());

    // La Mayotte and Réunion
    let mut re_number = PhoneNumber::default();
    re_number.country_code = 262;
    re_number.national_number = 262123456;
    assert!(phone_util.is_valid_number(&re_number).unwrap());
    assert!(
        phone_util
            .is_valid_number_for_region(&re_number, Region::RE)
            .unwrap()
    );
    assert!(
        !phone_util
            .is_valid_number_for_region(&re_number, Region::YT)
            .unwrap()
    );

    re_number.national_number = 269601234;
    assert!(
        phone_util
            .is_valid_number_for_region(&re_number, Region::YT)
            .unwrap()
    );
    assert!(
        !phone_util
            .is_valid_number_for_region(&re_number, Region::RE)
            .unwrap()
    );

    // This number is valid in both.
    re_number.national_number = 800123456;
    assert!(
        phone_util
            .is_valid_number_for_region(&re_number, Region::YT)
            .unwrap()
    );
    assert!(
        phone_util
            .is_valid_number_for_region(&re_number, Region::RE)
            .unwrap()
    );

    let mut intl_toll_free = PhoneNumber::default();
    intl_toll_free.country_code = 800;
    intl_toll_free.national_number = 12345678;
    assert!(
        phone_util
            .is_valid_number_for_region(&intl_toll_free, Region::World)
            .unwrap()
    );
    assert!(
        !phone_util
            .is_valid_number_for_region(&intl_toll_free, Region::US)
            .unwrap()
    );
    assert!(
        !phone_util
            .is_valid_number_for_region(&intl_toll_free, Region::ZZ)
            .unwrap()
    );

    let mut invalid_number = PhoneNumber::default();
    invalid_number.country_code = 3923;
    invalid_number.national_number = 2366;
    assert!(
        !phone_util
            .is_valid_number_for_region(&invalid_number, Region::ZZ)
            .unwrap()
    );
    assert!(
        !phone_util
            .is_valid_number_for_region(&invalid_number, Region::World)
            .unwrap()
    );

    invalid_number.country_code = 0;
    assert!(
        !phone_util
            .is_valid_number_for_region(&invalid_number, Region::World)
            .unwrap()
    );
    assert!(
        !phone_util
            .is_valid_number_for_region(&invalid_number, Region::ZZ)
            .unwrap()
    );
}

#[test]
fn is_not_valid_number() {
    let phone_util = get_phone_util();
    let mut number = PhoneNumber::default();

    number.country_code = 1;
    number.national_number = 2530000;
    assert!(!phone_util.is_valid_number(&number).unwrap());

    let mut number = PhoneNumber::default();
    number.country_code = 39;
    number.national_number = 23661830000;
    number.italian_leading_zero = Some(true);
    assert!(!phone_util.is_valid_number(&number).unwrap());

    let mut number = PhoneNumber::default();
    number.country_code = 44;
    number.national_number = 791234567;
    assert!(!phone_util.is_valid_number(&number).unwrap());

    let mut number = PhoneNumber::default();
    number.country_code = 49;
    number.national_number = 1234;
    assert!(!phone_util.is_valid_number(&number).unwrap());

    let mut number = PhoneNumber::default();
    number.country_code = 64;
    number.national_number = 3316005;
    assert!(!phone_util.is_valid_number(&number).unwrap());

    let mut number = PhoneNumber::default();
    number.country_code = 3923;
    number.national_number = 2366;
    assert!(!phone_util.is_valid_number(&number).unwrap());

    number.country_code = 0;
    assert!(!phone_util.is_valid_number(&number).unwrap());

    let mut number = PhoneNumber::default();
    number.country_code = 800;
    number.national_number = 123456789;
    assert!(!phone_util.is_valid_number(&number).unwrap());
}

#[test]
fn get_region_for_number() {
    let phone_util = get_phone_util();
    let mut number = PhoneNumber::default();

    number.country_code = 1;
    number.national_number = 2423232345;
    assert_eq!(
        Some(Region::BS),
        phone_util.get_region_for_number(&number).unwrap()
    );

    number.national_number = 4241231234;
    assert_eq!(
        Some(Region::US),
        phone_util.get_region_for_number(&number).unwrap()
    );

    number.country_code = 44;
    number.national_number = 7912345678;
    assert_eq!(
        Some(Region::GB),
        phone_util.get_region_for_number(&number).unwrap()
    );

    number.country_code = 800;
    number.national_number = 12345678;
    assert_eq!(
        Some(Region::World),
        phone_util.get_region_for_number(&number).unwrap()
    );

    number.country_code = 979;
    number.national_number = 123456789;
    assert_eq!(
        Some(Region::World),
        phone_util.get_region_for_number(&number).unwrap()
    );
}

#[test]
fn is_possible_number() {
    let phone_util = get_phone_util();
    let mut number = PhoneNumber::default();
    number.country_code = 1;
    number.national_number = 6502530000;
    assert!(phone_util.is_possible_number(&number));
    number.national_number = 2530000;
    assert!(phone_util.is_possible_number(&number));

    number.country_code = 44;
    number.national_number = 2070313000;
    assert!(phone_util.is_possible_number(&number));

    number.country_code = 800;
    number.national_number = 12345678;
    assert!(phone_util.is_possible_number(&number));

    assert!(phone_util.is_possible_number_for_string("+1 650 253 0000", Region::US));
    assert!(phone_util.is_possible_number_for_string("+1 650 GOO OGLE", Region::US));
    assert!(phone_util.is_possible_number_for_string("(650) 253-0000", Region::US));
    assert!(phone_util.is_possible_number_for_string("253-0000", Region::US));
    assert!(phone_util.is_possible_number_for_string("+1 650 253 0000", Region::GB));
    assert!(phone_util.is_possible_number_for_string("+44 20 7031 3000", Region::GB));
    assert!(phone_util.is_possible_number_for_string("(020) 7031 300", Region::GB));
    assert!(phone_util.is_possible_number_for_string("7031 3000", Region::GB));
    assert!(phone_util.is_possible_number_for_string("3331 6005", Region::NZ));
    assert!(phone_util.is_possible_number_for_string("+800 1234 5678", Region::World));
}

#[test]
fn is_possible_number_for_type_different_type_lengths() {
    let phone_util = get_phone_util();
    // Мы используем аргентинские номера, так как у них разная возможная длина для
    // разных типов.
    let mut number = PhoneNumber::default();
    number.country_code = 54;
    number.national_number = 12345;

    // Слишком короткий для любого аргентинского номера, включая стационарный.
    assert!(!phone_util.is_possible_number_for_type(&number, PhoneNumberType::FixedLine));
    assert!(!phone_util.is_possible_number_for_type(&number, PhoneNumberType::Unknown));

    // 6-значные номера подходят для стационарных телефонов.
    number.national_number = 123456;
    assert!(phone_util.is_possible_number_for_type(&number, PhoneNumberType::Unknown));
    assert!(phone_util.is_possible_number_for_type(&number, PhoneNumberType::FixedLine));
    // Но слишком короткие для мобильных.
    assert!(!phone_util.is_possible_number_for_type(&number, PhoneNumberType::Mobile));
    // И слишком короткие для бесплатных номеров.
    assert!(!phone_util.is_possible_number_for_type(&number, PhoneNumberType::TollFree));

    // То же самое относится к 9-значным номерам.
    number.national_number = 123456789;
    assert!(phone_util.is_possible_number_for_type(&number, PhoneNumberType::Unknown));
    assert!(phone_util.is_possible_number_for_type(&number, PhoneNumberType::FixedLine));
    assert!(!phone_util.is_possible_number_for_type(&number, PhoneNumberType::Mobile));
    assert!(!phone_util.is_possible_number_for_type(&number, PhoneNumberType::TollFree));

    // 10-значные номера возможны для всех типов.
    number.national_number = 1234567890;
    assert!(phone_util.is_possible_number_for_type(&number, PhoneNumberType::Unknown));
    assert!(phone_util.is_possible_number_for_type(&number, PhoneNumberType::FixedLine));
    assert!(phone_util.is_possible_number_for_type(&number, PhoneNumberType::Mobile));
    assert!(phone_util.is_possible_number_for_type(&number, PhoneNumberType::TollFree));

    // 11-значные номера возможны только для мобильных номеров. Обратите внимание, что мы не
    // требуем ведущую 9, с которой начинаются все мобильные номера и которая
    // была бы необходима для действительного мобильного номера.
    number.national_number = 12345678901;
    assert!(phone_util.is_possible_number_for_type(&number, PhoneNumberType::Unknown));
    assert!(!phone_util.is_possible_number_for_type(&number, PhoneNumberType::FixedLine));
    assert!(phone_util.is_possible_number_for_type(&number, PhoneNumberType::Mobile));
    assert!(!phone_util.is_possible_number_for_type(&number, PhoneNumberType::TollFree));
}

#[test]
fn is_possible_number_for_type_local_only() {
    let phone_util = get_phone_util();
    let mut number = PhoneNumber::default();
    // Здесь мы тестируем длину номера, которая соответствует длине только для местных номеров.
    number.country_code = 49;
    number.national_number = 12;
    assert!(phone_util.is_possible_number_for_type(&number, PhoneNumberType::Unknown));
    assert!(phone_util.is_possible_number_for_type(&number, PhoneNumberType::FixedLine));
    // Мобильные номера должны состоять из 10 или 11 цифр, и для них нет длин,
    // предназначенных только для местных номеров.
    assert!(!phone_util.is_possible_number_for_type(&number, PhoneNumberType::Mobile));
}

#[test]
fn is_possible_number_for_type_data_missing_for_size_reasons() {
    let phone_util = get_phone_util();
    let mut number = PhoneNumber::default();
    // Здесь мы тестируем случай, когда возможные длины соответствуют возможным
    // длинам страны в целом и, следовательно, отсутствуют в бинарных данных
    // по соображениям размера - это все равно должно работать.
    // Номер только для местного использования.
    number.country_code = 55;
    number.national_number = 12345678;
    assert!(phone_util.is_possible_number_for_type(&number, PhoneNumberType::Unknown));
    assert!(phone_util.is_possible_number_for_type(&number, PhoneNumberType::FixedLine));

    number.national_number = 1234567890;
    assert!(phone_util.is_possible_number_for_type(&number, PhoneNumberType::Unknown));
    assert!(phone_util.is_possible_number_for_type(&number, PhoneNumberType::FixedLine));
}

#[test]
fn is_possible_number_for_type_number_type_not_supported_for_region() {
    let phone_util = get_phone_util();
    let mut number = PhoneNumber::default();
    // Для этого региона вообще нет мобильных номеров, поэтому мы возвращаем false.
    number.country_code = 55;
    number.national_number = 12345678;
    assert!(!phone_util.is_possible_number_for_type(&number, PhoneNumberType::Mobile));
    // Однако это соответствует длине стационарного номера.
    assert!(phone_util.is_possible_number_for_type(&number, PhoneNumberType::FixedLine));
    assert!(phone_util.is_possible_number_for_type(&number, PhoneNumberType::FixedLineOrMobile));

    // Для этого кода страны вообще нет ни стационарных, ни мобильных номеров,
    // поэтому мы возвращаем false для них.
    number.country_code = 979;
    number.national_number = 123456789;
    assert!(!phone_util.is_possible_number_for_type(&number, PhoneNumberType::Mobile));
    assert!(!phone_util.is_possible_number_for_type(&number, PhoneNumberType::FixedLine));
    assert!(!phone_util.is_possible_number_for_type(&number, PhoneNumberType::FixedLineOrMobile));
    assert!(phone_util.is_possible_number_for_type(&number, PhoneNumberType::PremiumRate));
}

#[test]
fn is_not_possible_number() {
    let phone_util = get_phone_util();
    let mut number = PhoneNumber::default();

    number.country_code = 1;
    number.national_number = 65025300000;
    assert!(!phone_util.is_possible_number(&number));

    number.country_code = 800;
    number.national_number = 123456789;
    assert!(!phone_util.is_possible_number(&number));

    number.country_code = 1;
    number.national_number = 253000;
    assert!(!phone_util.is_possible_number(&number));

    number.country_code = 44;
    number.national_number = 300;
    assert!(!phone_util.is_possible_number(&number));

    assert!(!phone_util.is_possible_number_for_string("+1 650 253 00000", Region::US));
    assert!(!phone_util.is_possible_number_for_string("(650) 253-00000", Region::US));
    assert!(!phone_util.is_possible_number_for_string("I want a Pizza", Region::US));
    assert!(!phone_util.is_possible_number_for_string("253-000", Region::US));
    assert!(!phone_util.is_possible_number_for_string("1 3000", Region::GB));
    assert!(!phone_util.is_possible_number_for_string("+44 300", Region::GB));
    assert!(!phone_util.is_possible_number_for_string("+800 1234 5678 9", Region::World));
}

#[test]
fn is_possible_number_with_reason() {
    let phone_util = get_phone_util();
    let mut number = PhoneNumber::default();

    number.country_code = 1;
    number.national_number = 6502530000;
    assert_eq!(
        Ok(NumberLengthType::IsPossible),
        phone_util.is_possible_number_with_reason(&number)
    );

    number.national_number = 2530000;
    assert_eq!(
        Ok(NumberLengthType::IsPossibleLocalOnly),
        phone_util.is_possible_number_with_reason(&number)
    );

    number.country_code = 0;
    assert_eq!(
        Err(ValidationError::InvalidCountryCode),
        phone_util.is_possible_number_with_reason(&number)
    );

    number.country_code = 1;
    number.national_number = 253000;
    assert_eq!(
        Err(ValidationError::TooShort),
        phone_util.is_possible_number_with_reason(&number)
    );

    number.national_number = 65025300000;
    assert_eq!(
        Err(ValidationError::TooLong),
        phone_util.is_possible_number_with_reason(&number)
    );

    number.country_code = 44;
    number.national_number = 2070310000;
    assert_eq!(
        Ok(NumberLengthType::IsPossible),
        phone_util.is_possible_number_with_reason(&number)
    );

    number.country_code = 49;
    number.national_number = 30123456;
    assert_eq!(
        Ok(NumberLengthType::IsPossible),
        phone_util.is_possible_number_with_reason(&number)
    );

    number.country_code = 65;
    number.national_number = 1234567890;
    assert_eq!(
        Ok(NumberLengthType::IsPossible),
        phone_util.is_possible_number_with_reason(&number)
    );

    number.country_code = 800;
    number.national_number = 123456789;
    assert_eq!(
        Err(ValidationError::TooLong),
        phone_util.is_possible_number_with_reason(&number)
    );
}

#[test]
fn is_possible_number_for_type_with_reason() {
    let phone_util = get_phone_util();
    let mut ar_number = PhoneNumber::default();
    ar_number.country_code = 54;

    ar_number.national_number = 12345;
    assert_eq!(
        Err(ValidationError::TooShort),
        phone_util.is_possible_number_for_type_with_reason(&ar_number, PhoneNumberType::Unknown)
    );
    assert_eq!(
        Err(ValidationError::TooShort),
        phone_util.is_possible_number_for_type_with_reason(&ar_number, PhoneNumberType::FixedLine)
    );

    ar_number.national_number = 123456;
    assert_eq!(
        Ok(NumberLengthType::IsPossible),
        phone_util.is_possible_number_for_type_with_reason(&ar_number, PhoneNumberType::Unknown)
    );
    assert_eq!(
        Ok(NumberLengthType::IsPossible),
        phone_util.is_possible_number_for_type_with_reason(&ar_number, PhoneNumberType::FixedLine)
    );
    assert_eq!(
        Err(ValidationError::TooShort),
        phone_util.is_possible_number_for_type_with_reason(&ar_number, PhoneNumberType::Mobile)
    );
    assert_eq!(
        Err(ValidationError::TooShort),
        phone_util.is_possible_number_for_type_with_reason(&ar_number, PhoneNumberType::TollFree)
    );

    ar_number.national_number = 12345678901;
    assert_eq!(
        Ok(NumberLengthType::IsPossible),
        phone_util.is_possible_number_for_type_with_reason(&ar_number, PhoneNumberType::Unknown)
    );
    assert_eq!(
        Err(ValidationError::TooLong),
        phone_util.is_possible_number_for_type_with_reason(&ar_number, PhoneNumberType::FixedLine)
    );
    assert_eq!(
        Ok(NumberLengthType::IsPossible),
        phone_util.is_possible_number_for_type_with_reason(&ar_number, PhoneNumberType::Mobile)
    );
    assert_eq!(
        Err(ValidationError::TooLong),
        phone_util.is_possible_number_for_type_with_reason(&ar_number, PhoneNumberType::TollFree)
    );

    let mut de_number = PhoneNumber::default();
    de_number.country_code = 49;
    de_number.national_number = 12;
    assert_eq!(
        Ok(NumberLengthType::IsPossibleLocalOnly),
        phone_util.is_possible_number_for_type_with_reason(&de_number, PhoneNumberType::Unknown)
    );
    assert_eq!(
        Ok(NumberLengthType::IsPossibleLocalOnly),
        phone_util.is_possible_number_for_type_with_reason(&de_number, PhoneNumberType::FixedLine)
    );
    assert_eq!(
        Err(ValidationError::TooShort),
        phone_util.is_possible_number_for_type_with_reason(&de_number, PhoneNumberType::Mobile)
    );

    let mut br_number = PhoneNumber::default();
    br_number.country_code = 55;
    br_number.national_number = 12345678;
    assert_eq!(
        Err(ValidationError::InvalidLength),
        phone_util.is_possible_number_for_type_with_reason(&br_number, PhoneNumberType::Mobile)
    );
    assert_eq!(
        Ok(NumberLengthType::IsPossibleLocalOnly),
        phone_util.is_possible_number_for_type_with_reason(
            &br_number,
            PhoneNumberType::FixedLineOrMobile
        )
    );
}

#[test]
fn is_possible_number_for_type_with_reason_different_type_lengths() {
    // Мы используем аргентинские номера, так как у них разная возможная длина для разных типов.
    let phone_util = get_phone_util();
    let mut number = PhoneNumber::default();
    number.country_code = 54;
    number.national_number = 12345;

    assert_eq!(
        Err(ValidationError::TooShort),
        phone_util.is_possible_number_for_type_with_reason(&number, PhoneNumberType::Unknown)
    );
    assert_eq!(
        Err(ValidationError::TooShort),
        phone_util.is_possible_number_for_type_with_reason(&number, PhoneNumberType::FixedLine)
    );

    // 6-значные номера подходят для стационарных телефонов.
    number.national_number = 123456;
    assert_eq!(
        Ok(NumberLengthType::IsPossible),
        phone_util.is_possible_number_for_type_with_reason(&number, PhoneNumberType::Unknown)
    );
    assert_eq!(
        Ok(NumberLengthType::IsPossible),
        phone_util.is_possible_number_for_type_with_reason(&number, PhoneNumberType::FixedLine)
    );
    // Но слишком коротки для мобильных.
    assert_eq!(
        Err(ValidationError::TooShort),
        phone_util.is_possible_number_for_type_with_reason(&number, PhoneNumberType::Mobile)
    );
    // И слишком коротки для бесплатных номеров.
    assert_eq!(
        Err(ValidationError::TooShort),
        phone_util.is_possible_number_for_type_with_reason(&number, PhoneNumberType::TollFree)
    );

    // То же самое касается 9-значных номеров.
    number.national_number = 123456789;
    assert_eq!(
        Ok(NumberLengthType::IsPossible),
        phone_util.is_possible_number_for_type_with_reason(&number, PhoneNumberType::Unknown)
    );
    assert_eq!(
        Ok(NumberLengthType::IsPossible),
        phone_util.is_possible_number_for_type_with_reason(&number, PhoneNumberType::FixedLine)
    );
    assert_eq!(
        Err(ValidationError::TooShort),
        phone_util.is_possible_number_for_type_with_reason(&number, PhoneNumberType::Mobile)
    );
    assert_eq!(
        Err(ValidationError::TooShort),
        phone_util.is_possible_number_for_type_with_reason(&number, PhoneNumberType::TollFree)
    );

    // 10-значные номера возможны для всех типов.
    number.national_number = 1234567890;
    assert_eq!(
        Ok(NumberLengthType::IsPossible),
        phone_util.is_possible_number_for_type_with_reason(&number, PhoneNumberType::Unknown)
    );
    assert_eq!(
        Ok(NumberLengthType::IsPossible),
        phone_util.is_possible_number_for_type_with_reason(&number, PhoneNumberType::FixedLine)
    );
    assert_eq!(
        Ok(NumberLengthType::IsPossible),
        phone_util.is_possible_number_for_type_with_reason(&number, PhoneNumberType::Mobile)
    );
    assert_eq!(
        Ok(NumberLengthType::IsPossible),
        phone_util.is_possible_number_for_type_with_reason(&number, PhoneNumberType::TollFree)
    );

    // 11-значные номера возможны для мобильных номеров. Обратите внимание, что мы не требуем ведущую 9,
    // с которой начинаются все мобильные номера и которая была бы необходима для действительного мобильного номера.
    number.national_number = 12345678901;
    assert_eq!(
        Ok(NumberLengthType::IsPossible),
        phone_util.is_possible_number_for_type_with_reason(&number, PhoneNumberType::Unknown)
    );
    assert_eq!(
        Err(ValidationError::TooLong),
        phone_util.is_possible_number_for_type_with_reason(&number, PhoneNumberType::FixedLine)
    );
    assert_eq!(
        Ok(NumberLengthType::IsPossible),
        phone_util.is_possible_number_for_type_with_reason(&number, PhoneNumberType::Mobile)
    );
    assert_eq!(
        Err(ValidationError::TooLong),
        phone_util.is_possible_number_for_type_with_reason(&number, PhoneNumberType::TollFree)
    );
}

#[test]
fn is_possible_number_for_type_with_reason_local_only() {
    let phone_util = get_phone_util();
    let mut number = PhoneNumber::default();
    // Здесь мы тестируем длину номера, которая соответствует только местной длине.
    number.country_code = 49;
    number.national_number = 12;
    assert_eq!(
        Ok(NumberLengthType::IsPossibleLocalOnly),
        phone_util.is_possible_number_for_type_with_reason(&number, PhoneNumberType::Unknown)
    );
    assert_eq!(
        Ok(NumberLengthType::IsPossibleLocalOnly),
        phone_util.is_possible_number_for_type_with_reason(&number, PhoneNumberType::FixedLine)
    );
    // Мобильные номера должны состоять из 10 или 11 цифр, и для них нет только местных длин.
    assert_eq!(
        Err(ValidationError::TooShort),
        phone_util.is_possible_number_for_type_with_reason(&number, PhoneNumberType::Mobile)
    );
}

#[test]
fn is_possible_number_for_type_with_reason_data_missing_for_size_reasons() {
    let phone_util = get_phone_util();
    let mut number = PhoneNumber::default();
    // Здесь мы тестируем случай, когда возможные длины соответствуют возможным длинам страны в целом
    // и поэтому отсутствуют в бинарных данных по соображениям размера - это все равно должно работать.
    // Номер только для местного использования.
    number.country_code = 55;
    number.national_number = 12345678;
    assert_eq!(
        Ok(NumberLengthType::IsPossibleLocalOnly),
        phone_util.is_possible_number_for_type_with_reason(&number, PhoneNumberType::Unknown)
    );
    assert_eq!(
        Ok(NumberLengthType::IsPossibleLocalOnly),
        phone_util.is_possible_number_for_type_with_reason(&number, PhoneNumberType::FixedLine)
    );
    // Номер нормальной длины.
    number.national_number = 1234567890;
    assert_eq!(
        Ok(NumberLengthType::IsPossible),
        phone_util.is_possible_number_for_type_with_reason(&number, PhoneNumberType::Unknown)
    );
    assert_eq!(
        Ok(NumberLengthType::IsPossible),
        phone_util.is_possible_number_for_type_with_reason(&number, PhoneNumberType::FixedLine)
    );
}

#[test]
fn is_possible_number_for_type_with_reason_number_type_not_supported_for_region() {
    let phone_util = get_phone_util();
    let mut number = PhoneNumber::default();
    // В этом регионе вообще *нет* мобильных номеров, поэтому мы возвращаем INVALID_LENGTH.
    number.country_code = 55;
    number.national_number = 12345678;
    assert_eq!(
        Err(ValidationError::InvalidLength),
        phone_util.is_possible_number_for_type_with_reason(&number, PhoneNumberType::Mobile)
    );
    // Однако это соответствует длине стационарного номера.
    assert_eq!(
        Ok(NumberLengthType::IsPossibleLocalOnly),
        phone_util
            .is_possible_number_for_type_with_reason(&number, PhoneNumberType::FixedLineOrMobile)
    );
    // Этот номер слишком короткий для стационарного, а мобильных номеров не существует.
    number.national_number = 1234567;
    assert_eq!(
        Err(ValidationError::InvalidLength),
        phone_util.is_possible_number_for_type_with_reason(&number, PhoneNumberType::Mobile)
    );
    assert_eq!(
        Err(ValidationError::TooShort),
        phone_util
            .is_possible_number_for_type_with_reason(&number, PhoneNumberType::FixedLineOrMobile)
    );
    assert_eq!(
        Err(ValidationError::TooShort),
        phone_util.is_possible_number_for_type_with_reason(&number, PhoneNumberType::FixedLine)
    );
    // Этот номер слишком короткий для мобильного, а стационарных номеров не существует.
    number.country_code = 882;
    number.national_number = 1234567;
    assert_eq!(
        Err(ValidationError::TooShort),
        phone_util.is_possible_number_for_type_with_reason(&number, PhoneNumberType::Mobile)
    );
    assert_eq!(
        Err(ValidationError::TooShort),
        phone_util
            .is_possible_number_for_type_with_reason(&number, PhoneNumberType::FixedLineOrMobile)
    );
    assert_eq!(
        Err(ValidationError::InvalidLength),
        phone_util.is_possible_number_for_type_with_reason(&number, PhoneNumberType::FixedLine)
    );

    // Для этого кода страны вообще *нет* ни стационарных, ни мобильных номеров,
    // поэтому мы возвращаем INVALID_LENGTH.
    number.country_code = 979;
    number.national_number = 123456789;
    assert_eq!(
        Err(ValidationError::InvalidLength),
        phone_util.is_possible_number_for_type_with_reason(&number, PhoneNumberType::Mobile)
    );
    assert_eq!(
        Err(ValidationError::InvalidLength),
        phone_util.is_possible_number_for_type_with_reason(&number, PhoneNumberType::FixedLine)
    );
    assert_eq!(
        Err(ValidationError::InvalidLength),
        phone_util
            .is_possible_number_for_type_with_reason(&number, PhoneNumberType::FixedLineOrMobile)
    );
    assert_eq!(
        Ok(NumberLengthType::IsPossible),
        phone_util.is_possible_number_for_type_with_reason(&number, PhoneNumberType::PremiumRate)
    );
}

#[test]
fn is_possible_number_for_type_with_reason_fixed_line_or_mobile() {
    let phone_util = get_phone_util();
    let mut number = PhoneNumber::default();
    // Для FIXED_LINE_OR_MOBILE номер должен считаться действительным, если он соответствует
    // возможным длинам для мобильных *или* стационарных номеров.
    number.country_code = 290;
    number.national_number = 1234;
    assert_eq!(
        Err(ValidationError::TooShort),
        phone_util.is_possible_number_for_type_with_reason(&number, PhoneNumberType::FixedLine)
    );
    assert_eq!(
        Ok(NumberLengthType::IsPossible),
        phone_util.is_possible_number_for_type_with_reason(&number, PhoneNumberType::Mobile)
    );
    assert_eq!(
        Ok(NumberLengthType::IsPossible),
        phone_util
            .is_possible_number_for_type_with_reason(&number, PhoneNumberType::FixedLineOrMobile)
    );

    number.national_number = 12345;
    assert_eq!(
        Err(ValidationError::TooShort),
        phone_util.is_possible_number_for_type_with_reason(&number, PhoneNumberType::FixedLine)
    );
    assert_eq!(
        Err(ValidationError::TooLong),
        phone_util.is_possible_number_for_type_with_reason(&number, PhoneNumberType::Mobile)
    );
    assert_eq!(
        Err(ValidationError::InvalidLength),
        phone_util
            .is_possible_number_for_type_with_reason(&number, PhoneNumberType::FixedLineOrMobile)
    );

    number.national_number = 123456;
    assert_eq!(
        Ok(NumberLengthType::IsPossible),
        phone_util.is_possible_number_for_type_with_reason(&number, PhoneNumberType::FixedLine)
    );
    assert_eq!(
        Err(ValidationError::TooLong),
        phone_util.is_possible_number_for_type_with_reason(&number, PhoneNumberType::Mobile)
    );
    assert_eq!(
        Ok(NumberLengthType::IsPossible),
        phone_util
            .is_possible_number_for_type_with_reason(&number, PhoneNumberType::FixedLineOrMobile)
    );

    number.national_number = 1234567;
    assert_eq!(
        Err(ValidationError::TooLong),
        phone_util.is_possible_number_for_type_with_reason(&number, PhoneNumberType::FixedLine)
    );
    assert_eq!(
        Err(ValidationError::TooLong),
        phone_util.is_possible_number_for_type_with_reason(&number, PhoneNumberType::Mobile)
    );
    assert_eq!(
        Err(ValidationError::TooLong),
        phone_util
            .is_possible_number_for_type_with_reason(&number, PhoneNumberType::FixedLineOrMobile)
    );

    number.national_number = 12345678;
    assert_eq!(
        Ok(NumberLengthType::IsPossible),
        phone_util.is_possible_number_for_type_with_reason(&number, PhoneNumberType::TollFree)
    );
    assert_eq!(
        Err(ValidationError::TooLong),
        phone_util
            .is_possible_number_for_type_with_reason(&number, PhoneNumberType::FixedLineOrMobile)
    );
}

#[test]
fn truncate_too_long_number() {
    let phone_util = get_phone_util();

    let mut too_long_number = phone_util.parse("+165025300001", Some(Region::US)).unwrap();
    let valid_number = phone_util.parse("+16502530000", Some(Region::US)).unwrap();
    assert!(
        phone_util
            .truncate_too_long_number(&mut too_long_number)
            .unwrap()
    );
    assert_eq!(valid_number, too_long_number);

    let mut valid_number_copy = valid_number.clone();
    assert!(
        phone_util
            .truncate_too_long_number(&mut valid_number_copy)
            .unwrap()
    );
    assert_eq!(valid_number, valid_number_copy);

    let mut too_short_number = phone_util.parse("+11234", Some(Region::US)).unwrap();
    let too_short_number_copy = too_short_number.clone();
    assert!(
        !phone_util
            .truncate_too_long_number(&mut too_short_number)
            .unwrap()
    );
    assert_eq!(too_short_number_copy, too_short_number);
}

#[test]
fn normalise_remove_punctuation() {
    let phone_util = get_phone_util();
    let input_number = "034-56&+#2\u{ad}34".to_string();
    let normalized_number = phone_util.normalize(&input_number);
    let expected_output = "03456234";
    assert_eq!(
        expected_output, normalized_number,
        "Conversion did not correctly remove punctuation"
    );
}

#[test]
fn normalise_replace_alpha_characters() {
    let phone_util = get_phone_util();
    let input_number = "034-I-am-HUNGRY".to_string();
    let normalized_number = phone_util.normalize(&input_number);
    let expected_output = "034426486479";
    assert_eq!(
        expected_output, normalized_number,
        "Conversion did not correctly replace alpha characters"
    );
}

#[test]
fn normalise_other_digits() {
    let phone_util = get_phone_util();
    // Full-width 2, Arabic-indic 5
    let input = "\u{ff12}5\u{0665}"; // "２5٥"
    assert_eq!("255", phone_util.normalize(input));

    // Eastern-Arabic 5 and 0
    let input = "\u{06f5}2\u{06f0}"; // "۵2۰"
    assert_eq!("520", phone_util.normalize(input));
}

#[test]
fn normalise_strip_alpha_characters() {
    let phone_util = get_phone_util();
    let input_number = "034-56&+a#234".to_string();
    let normalized_number = phone_util.normalize_digits_only(&input_number);
    let expected_output = "03456234";
    assert_eq!(
        expected_output, normalized_number,
        "Conversion did not correctly remove alpha characters"
    );
}

#[test]
fn normalise_strip_non_diallable_characters() {
    let phone_util = get_phone_util();
    let input_number = "03*4-56&+1a#234";
    let expected_output = "03*456+1#234";
    assert_eq!(
        expected_output,
        phone_util.normalize_diallable_chars_only(input_number),
        "Conversion did not correctly remove non-diallable characters"
    );
}

#[test]
fn maybe_strip_international_prefix() {
    let phone_util = get_phone_util();
    let international_prefix = RegexTriplets::new(wrap_regexp_str("00[39]"));

    let number_to_strip = "0034567700-3898003";
    // Примечание: дефис удаляется в процессе нормализации.
    let stripped_number = "45677003898003";

    let number_with_source = phone_util
        .maybe_strip_international_prefix_and_normalize(
            number_to_strip,
            Some(&international_prefix),
        )
        .unwrap();
    assert_eq!(
        CountryCodeSource::FromNumberWithIdd,
        number_with_source.country_code_source
    );
    assert_eq!(
        stripped_number, number_with_source.phone_number,
        "The number was not stripped of its international prefix."
    );

    // Теперь номер больше не начинается с префикса IDD, поэтому он должен сообщать
    // FROM_DEFAULT_COUNTRY.
    assert_eq!(
        CountryCodeSource::FromDefaultCountry,
        phone_util
            .maybe_strip_international_prefix_and_normalize(
                &number_with_source.phone_number,
                Some(&international_prefix)
            )
            .unwrap()
            .country_code_source
    );

    let number_to_strip = "00945677003898003";
    let number_with_source = phone_util
        .maybe_strip_international_prefix_and_normalize(
            number_to_strip,
            Some(&international_prefix),
        )
        .unwrap();
    assert_eq!(
        CountryCodeSource::FromNumberWithIdd,
        number_with_source.country_code_source
    );
    assert_eq!(
        stripped_number, number_with_source.phone_number,
        "The number was not stripped of its international prefix."
    );

    // Проверяем, что это работает, когда международный префикс разбит пробелами.
    let number_to_strip = "00 9 45677003898003";
    let number_with_source = phone_util
        .maybe_strip_international_prefix_and_normalize(
            number_to_strip,
            Some(&international_prefix),
        )
        .unwrap();
    assert_eq!(
        CountryCodeSource::FromNumberWithIdd,
        number_with_source.country_code_source
    );
    assert_eq!(
        stripped_number, number_with_source.phone_number,
        "The number was not stripped of its international prefix."
    );

    // Теперь номер больше не начинается с префикса IDD, поэтому он должен сообщать
    // FROM_DEFAULT_COUNTRY.
    assert_eq!(
        CountryCodeSource::FromDefaultCountry,
        phone_util
            .maybe_strip_international_prefix_and_normalize(
                &number_with_source.phone_number,
                Some(&international_prefix)
            )
            .unwrap()
            .country_code_source
    );

    // Проверяем, что символ + также распознается и удаляется.
    let number_to_strip = "+45677003898003";
    let stripped_number_plus = "45677003898003";
    let number_with_source = phone_util
        .maybe_strip_international_prefix_and_normalize(
            number_to_strip,
            Some(&international_prefix),
        )
        .unwrap();
    assert_eq!(
        CountryCodeSource::FromNumberWithPlusSign,
        number_with_source.country_code_source
    );
    assert_eq!(
        stripped_number_plus, number_with_source.phone_number,
        "The number supplied was not stripped of the plus symbol."
    );

    // Если после префикса идет ноль, мы не должны его удалять - ни один код страны не начинается с 0.
    let number_to_strip = "0090112-3123";
    let stripped_number_zero = "00901123123";
    let number_with_source = phone_util
        .maybe_strip_international_prefix_and_normalize(
            number_to_strip,
            Some(&international_prefix),
        )
        .unwrap();
    assert_eq!(
        CountryCodeSource::FromDefaultCountry,
        number_with_source.country_code_source
    );
    assert_eq!(
        stripped_number_zero, number_with_source.phone_number,
        "The number had a 0 after the match so shouldn't be stripped."
    );

    // Здесь 0 отделен от IDD пробелом.
    let number_to_strip = "009 0-112-3123";
    assert_eq!(
        CountryCodeSource::FromDefaultCountry,
        phone_util
            .maybe_strip_international_prefix_and_normalize(
                number_to_strip,
                Some(&international_prefix)
            )
            .unwrap()
            .country_code_source
    );
}

#[test]
fn maybe_strip_extension() {
    let phone_util = get_phone_util();
    let number = "1234576 ext. 1234";
    let expected_extension = "1234";
    let stripped_number = "1234576";
    let (number, extension) = phone_util.maybe_strip_extension(number);
    assert!(extension.is_some());
    assert_eq!(stripped_number, number);
    assert_eq!(
        expected_extension,
        extension.map(|ext| ext.as_str()).unwrap()
    );
}

#[test]
fn get_number_type() {
    let phone_util = get_phone_util();
    let mut number = PhoneNumber::default();

    // PREMIUM_RATE
    number.country_code = 1;
    number.national_number = 9004433030;
    assert_eq!(
        PhoneNumberType::PremiumRate,
        phone_util.get_number_type(&number).unwrap()
    );
    number.country_code = 44;
    number.national_number = 9187654321;
    assert_eq!(
        PhoneNumberType::PremiumRate,
        phone_util.get_number_type(&number).unwrap()
    );

    // TOLL_FREE
    number.country_code = 1;
    number.national_number = 8881234567;
    assert_eq!(
        PhoneNumberType::TollFree,
        phone_util.get_number_type(&number).unwrap()
    );
    number.country_code = 44;
    number.national_number = 8012345678;
    assert_eq!(
        PhoneNumberType::TollFree,
        phone_util.get_number_type(&number).unwrap()
    );
    number.country_code = 800;
    number.national_number = 12345678;
    assert_eq!(
        PhoneNumberType::TollFree,
        phone_util.get_number_type(&number).unwrap()
    );

    // MOBILE
    number.country_code = 1;
    number.national_number = 2423570000;
    assert_eq!(
        PhoneNumberType::Mobile,
        phone_util.get_number_type(&number).unwrap()
    );
    number.country_code = 44;
    number.national_number = 7912345678;
    assert_eq!(
        PhoneNumberType::Mobile,
        phone_util.get_number_type(&number).unwrap()
    );

    // FIXED_LINE
    number.country_code = 1;
    number.national_number = 2423651234;
    assert_eq!(
        PhoneNumberType::FixedLine,
        phone_util.get_number_type(&number).unwrap()
    );
    let mut number = PhoneNumber::default();
    number.country_code = 39;
    number.national_number = 236618300;
    number.italian_leading_zero = Some(true);
    assert_eq!(
        PhoneNumberType::FixedLine,
        phone_util.get_number_type(&number).unwrap()
    );
    let mut number = PhoneNumber::default();
    number.country_code = 44;
    number.national_number = 2012345678;
    assert_eq!(
        PhoneNumberType::FixedLine,
        phone_util.get_number_type(&number).unwrap()
    );

    // FIXED_LINE_OR_MOBILE
    let mut number = PhoneNumber::default();
    number.country_code = 1;
    number.national_number = 6502531111;
    assert_eq!(
        PhoneNumberType::FixedLineOrMobile,
        phone_util.get_number_type(&number).unwrap()
    );

    // SHARED_COST
    let mut number = PhoneNumber::default();
    number.country_code = 44;
    number.national_number = 8431231234;
    assert_eq!(
        PhoneNumberType::SharedCost,
        phone_util.get_number_type(&number).unwrap()
    );

    // VOIP
    let mut number = PhoneNumber::default();
    number.country_code = 44;
    number.national_number = 5631231234;
    assert_eq!(
        PhoneNumberType::VoIP,
        phone_util.get_number_type(&number).unwrap()
    );

    // PERSONAL_NUMBER
    let mut number = PhoneNumber::default();
    number.country_code = 44;
    number.national_number = 7031231234;
    assert_eq!(
        PhoneNumberType::PersonalNumber,
        phone_util.get_number_type(&number).unwrap()
    );

    // UNKNOWN
    let mut number = PhoneNumber::default();
    number.country_code = 1;
    number.national_number = 65025311111;
    assert_eq!(
        PhoneNumberType::Unknown,
        phone_util.get_number_type(&number).unwrap()
    );
}

#[test]
fn parse_national_number() {
    let phone_util = get_phone_util();

    let mut nz_number = PhoneNumber::default();
    nz_number.country_code = 64;
    nz_number.national_number = 33316005;

    // С национальным префиксом.
    let test_number = phone_util.parse("033316005", Some(Region::NZ)).unwrap();
    assert_eq!(nz_number, test_number);

    // Без национального префикса.
    let test_number = phone_util.parse("33316005", Some(Region::NZ)).unwrap();
    assert_eq!(nz_number, test_number);

    // С национальным префиксом и форматированием.
    let test_number = phone_util.parse("03-331 6005", Some(Region::NZ)).unwrap();
    assert_eq!(nz_number, test_number);
    let test_number = phone_util.parse("03 331 6005", Some(Region::NZ)).unwrap();
    assert_eq!(nz_number, test_number);

    // Тестирование парсинга формата RFC3966 с phone-context.
    let test_number = phone_util
        .parse("tel:03-331-6005;phone-context=+64", Some(Region::NZ))
        .unwrap();
    assert_eq!(nz_number, test_number);
    let test_number = phone_util
        .parse("tel:331-6005;phone-context=+64-3", Some(Region::NZ))
        .unwrap();
    assert_eq!(nz_number, test_number);
    let test_number = phone_util
        .parse("tel:331-6005;phone-context=+64-3", Some(Region::US))
        .unwrap();
    assert_eq!(nz_number, test_number);
    let test_number = phone_util
        .parse(
            "My number is tel:03-331-6005;phone-context=+64",
            Some(Region::NZ),
        )
        .unwrap();
    assert_eq!(nz_number, test_number);

    // Тестирование парсинга RFC3966 с опциональными параметрами.
    let test_number = phone_util
        .parse("tel:03-331-6005;phone-context=+64;a=%A1", Some(Region::NZ))
        .unwrap();
    assert_eq!(nz_number, test_number);

    // Тестирование парсинга RFC3966 с ISDN-субадресом.
    let test_number = phone_util
        .parse(
            "tel:03-331-6005;isub=12345;phone-context=+64",
            Some(Region::NZ),
        )
        .unwrap();
    assert_eq!(nz_number, test_number);
    let test_number = phone_util
        .parse("tel:+64-3-331-6005;isub=12345", Some(Region::US))
        .unwrap();
    assert_eq!(nz_number, test_number);
    let test_number = phone_util
        .parse("03-331-6005;phone-context=+64", Some(Region::NZ))
        .unwrap();
    assert_eq!(nz_number, test_number);

    // Тестирование международных префиксов.
    // Код страны должен быть удалён.
    let test_number = phone_util
        .parse("0064 3 d331 6005", Some(Region::NZ))
        .unwrap();
    assert_eq!(nz_number, test_number);

    // Попробуем снова, но на этот раз с международным номером для региона US.
    // Код страны должен быть распознан и обработан корректно.
    let test_number = phone_util
        .parse("01164 3 331 6005", Some(Region::US))
        .unwrap();
    assert_eq!(nz_number, test_number);
    let test_number = phone_util
        .parse("+64 3 331 6005", Some(Region::US))
        .unwrap();
    assert_eq!(nz_number, test_number);

    // Ведущий плюс должен игнорироваться, т.к. за ним следует не код страны, а IDD для США.
    let test_number = phone_util
        .parse("+01164 3 331 6005", Some(Region::US))
        .unwrap();
    assert_eq!(nz_number, test_number);
    let test_number = phone_util
        .parse("+0064 3 331 6005", Some(Region::NZ))
        .unwrap();
    assert_eq!(nz_number, test_number);
    let test_number = phone_util
        .parse("+ 00 64 3 331 6005", Some(Region::NZ))
        .unwrap();
    assert_eq!(nz_number, test_number);

    let mut us_local_number = PhoneNumber::default();
    us_local_number.country_code = 1;
    us_local_number.national_number = 2530000;
    let test_number = phone_util
        .parse(
            "tel:253-0000;phone-context=www.google.com",
            Some(Region::US),
        )
        .unwrap();
    assert_eq!(us_local_number, test_number);
    let test_number = phone_util
        .parse(
            "tel:253-0000;isub=12345;phone-context=www.google.com",
            Some(Region::US),
        )
        .unwrap();
    assert_eq!(us_local_number, test_number);
    let test_number = phone_util
        .parse(
            "tel:2530000;isub=12345;phone-context=1234.com",
            Some(Region::US),
        )
        .unwrap();
    assert_eq!(us_local_number, test_number);

    // Тест для http://b/issue?id=2247493
    let mut nz_number_issue = PhoneNumber::default();
    nz_number_issue.country_code = 64;
    nz_number_issue.national_number = 64123456;
    let test_number = phone_util
        .parse("+64(0)64123456", Some(Region::US))
        .unwrap();
    assert_eq!(nz_number_issue, test_number);

    // Проверка, что "/" в номере телефона обрабатывается корректно.
    let mut de_number = PhoneNumber::default();
    de_number.country_code = 49;
    de_number.national_number = 12345678;
    let test_number = phone_util.parse("123/45678", Some(Region::DE)).unwrap();
    assert_eq!(de_number, test_number);

    let mut us_number = PhoneNumber::default();
    us_number.country_code = 1;
    // Проверка, что '1' не используется как код страны при парсинге, если номер уже валиден.
    us_number.national_number = 1234567890;
    let test_number = phone_util.parse("123-456-7890", Some(Region::US)).unwrap();
    assert_eq!(us_number, test_number);

    // Тестирование номеров со звездочкой.
    let mut star_number = PhoneNumber::default();
    star_number.country_code = 81;
    star_number.national_number = 2345;
    let test_number = phone_util.parse("+81 *2345", Some(Region::JP)).unwrap();
    assert_eq!(star_number, test_number);

    let mut short_number = PhoneNumber::default();
    short_number.country_code = 64;
    short_number.national_number = 12;
    let test_number = phone_util.parse("12", Some(Region::NZ)).unwrap();
    assert_eq!(short_number, test_number);

    // Тест для короткого номера с ведущим нулём для страны, где 0 - национальный префикс.
    // Убедиться, что он не интерпретируется как национальный префикс, если
    // оставшаяся длина номера соответствует только местному номеру.
    let mut short_number = PhoneNumber::default();
    short_number.country_code = 44;
    short_number.national_number = 123456;
    short_number.italian_leading_zero = Some(true);
    let test_number = phone_util.parse("0123456", Some(Region::GB)).unwrap();
    assert_eq!(short_number, test_number);
}

#[test]
fn parse_number_with_alpha_characters() {
    let phone_util = get_phone_util();

    // Тестовый случай с буквенными символами.
    let mut tollfree_number = PhoneNumber::default();
    tollfree_number.country_code = 64;
    tollfree_number.national_number = 800332005;
    let mut test_number = phone_util.parse("0800 DDA 005", Some(Region::NZ)).unwrap();
    assert_eq!(tollfree_number, test_number);

    let mut premium_number = PhoneNumber::default();
    premium_number.country_code = 64;
    premium_number.national_number = 9003326005;
    test_number = phone_util.parse("0900 DDA 6005", Some(Region::NZ)).unwrap();
    assert_eq!(premium_number, test_number);

    // Недостаточно буквенных символов, чтобы считать их преднамеренными, поэтому они удаляются.
    test_number = phone_util
        .parse("0900 332 6005a", Some(Region::NZ))
        .unwrap();
    assert_eq!(premium_number, test_number);

    test_number = phone_util
        .parse("0900 332 600a5", Some(Region::NZ))
        .unwrap();
    assert_eq!(premium_number, test_number);

    test_number = phone_util
        .parse("0900 332 600A5", Some(Region::NZ))
        .unwrap();
    assert_eq!(premium_number, test_number);

    test_number = phone_util
        .parse("0900 a332 600A5", Some(Region::NZ))
        .unwrap();
    assert_eq!(premium_number, test_number);
}

#[test]
fn parse_with_international_prefixes() {
    let phone_util = get_phone_util();
    let mut us_number = PhoneNumber::default();
    us_number.country_code = 1;
    us_number.national_number = 6503336000;

    let mut test_number = phone_util
        .parse("+1 (650) 333-6000", Some(Region::US))
        .unwrap();
    assert_eq!(us_number, test_number);
    test_number = phone_util
        .parse("+1-650-333-6000", Some(Region::US))
        .unwrap();
    assert_eq!(us_number, test_number);

    // Звонок на номер США из Сингапура с использованием разных поставщиков услуг
    // 1-й тест: звонок с использованием услуги SingTel IDD (IDD - 001)
    test_number = phone_util
        .parse("0011-650-333-6000", Some(Region::SG))
        .unwrap();
    assert_eq!(us_number, test_number);
    // 2-й тест: звонок с использованием услуги StarHub IDD (IDD - 008)
    test_number = phone_util
        .parse("0081-650-333-6000", Some(Region::SG))
        .unwrap();
    assert_eq!(us_number, test_number);
    // 3-й тест: звонок с использованием услуги SingTel V019 (IDD - 019)
    test_number = phone_util
        .parse("0191-650-333-6000", Some(Region::SG))
        .unwrap();
    assert_eq!(us_number, test_number);
    // Звонок на номер США из Польши
    test_number = phone_util
        .parse("0~01-650-333-6000", Some(Region::PL))
        .unwrap();
    assert_eq!(us_number, test_number);

    // Использование "++" в начале.
    test_number = phone_util
        .parse("++1 (650) 333-6000", Some(Region::PL))
        .unwrap();
    assert_eq!(us_number, test_number);
    // Использование полноширинного знака плюса.
    test_number = phone_util
        .parse("＋1 (650) 333-6000", Some(Region::SG))
        .unwrap();
    assert_eq!(us_number, test_number);
    // Использование мягкого дефиса U+00AD.
    test_number = phone_util
        .parse("1 (650) 333\u{00AD}-6000", Some(Region::US))
        .unwrap();
    assert_eq!(us_number, test_number);
    // Весь номер, включая знаки препинания, представлен в полноширинной форме.
    test_number = phone_util
        .parse("＋１　（６５０）　３３３－６０００", Some(Region::SG))
        .unwrap();
    assert_eq!(us_number, test_number);

    // Использование тире U+30FC.
    test_number = phone_util
        .parse("＋１　（６５０）　３３３ー６０００", Some(Region::SG))
        .unwrap();
    assert_eq!(us_number, test_number);

    let mut toll_free_number = PhoneNumber::default();
    toll_free_number.country_code = 800;
    toll_free_number.national_number = 12345678;
    test_number = phone_util
        .parse("011 800 1234 5678", Some(Region::US))
        .unwrap();
    assert_eq!(toll_free_number, test_number);
}

#[test]
fn parse_with_leading_zero() {
    let phone_util = get_phone_util();
    let mut it_number = PhoneNumber::default();
    it_number.country_code = 39;
    it_number.national_number = 236618300;
    it_number.italian_leading_zero = Some(true);

    let mut test_number = phone_util
        .parse("+39 02-36618 300", Some(Region::NZ))
        .unwrap();
    assert_eq!(it_number, test_number);

    test_number = phone_util.parse("02-36618 300", Some(Region::IT)).unwrap();
    assert_eq!(it_number, test_number);

    let mut it_number = PhoneNumber::default();
    it_number.country_code = 39;
    it_number.national_number = 312345678;
    test_number = phone_util.parse("312 345 678", Some(Region::IT)).unwrap();
    assert_eq!(it_number, test_number);
}

#[test]
fn parse_national_number_argentina() {
    let phone_util = get_phone_util();
    // Тестирование парсинга мобильных номеров Аргентины.
    let mut ar_number = PhoneNumber::default();
    ar_number.country_code = 54;
    ar_number.national_number = 93435551212;

    let mut test_number = phone_util
        .parse("+54 9 343 555 1212", Some(Region::AR))
        .unwrap();
    assert_eq!(ar_number, test_number);

    test_number = phone_util
        .parse("0343 15 555 1212", Some(Region::AR))
        .unwrap();
    assert_eq!(ar_number, test_number);

    ar_number.national_number = 93715654320;
    test_number = phone_util
        .parse("+54 9 3715 65 4320", Some(Region::AR))
        .unwrap();
    assert_eq!(ar_number, test_number);

    test_number = phone_util
        .parse("03715 15 65 4320", Some(Region::AR))
        .unwrap();
    assert_eq!(ar_number, test_number);

    // Тестирование парсинга стационарных номеров Аргентины.
    ar_number.national_number = 1137970000;
    test_number = phone_util
        .parse("+54 11 3797 0000", Some(Region::AR))
        .unwrap();
    assert_eq!(ar_number, test_number);

    test_number = phone_util.parse("011 3797 0000", Some(Region::AR)).unwrap();
    assert_eq!(ar_number, test_number);

    ar_number.national_number = 3715654321;
    test_number = phone_util
        .parse("+54 3715 65 4321", Some(Region::AR))
        .unwrap();
    assert_eq!(ar_number, test_number);

    test_number = phone_util.parse("03715 65 4321", Some(Region::AR)).unwrap();
    assert_eq!(ar_number, test_number);

    ar_number.national_number = 2312340000;
    test_number = phone_util
        .parse("+54 23 1234 0000", Some(Region::AR))
        .unwrap();
    assert_eq!(ar_number, test_number);

    test_number = phone_util.parse("023 1234 0000", Some(Region::AR)).unwrap();
    assert_eq!(ar_number, test_number);
}

#[test]
fn parse_with_x_in_number() {
    let phone_util = get_phone_util();
    // Проверяем, что наличие 'x' в начале номера телефона допустимо и что он просто удаляется.
    let mut ar_number = PhoneNumber::default();
    ar_number.country_code = 54;
    ar_number.national_number = 123456789;

    let mut test_number = phone_util.parse("0123456789", Some(Region::AR)).unwrap();
    assert_eq!(ar_number, test_number);

    test_number = phone_util.parse("(0) 123456789", Some(Region::AR)).unwrap();
    assert_eq!(ar_number, test_number);

    test_number = phone_util.parse("0 123456789", Some(Region::AR)).unwrap();
    assert_eq!(ar_number, test_number);

    test_number = phone_util
        .parse("(0xx) 123456789", Some(Region::AR))
        .unwrap();
    assert_eq!(ar_number, test_number);

    let mut ar_from_us = PhoneNumber::default();
    ar_from_us.country_code = 54;
    ar_from_us.national_number = 81429712;
    // Этот тест намеренно построен так, что количество цифр после xx больше 7,
    // чтобы номер не был ошибочно принят за добавочный, так как мы разрешаем
    // добавочные номера до 7 цифр. Это предположение на данный момент приемлемо,
    // так как все страны, где код выбора оператора записывается в виде xx,
    // имеют национальный значащий номер длиной более 7.
    test_number = phone_util
        .parse("011xx5481429712", Some(Region::US))
        .unwrap();
    assert_eq!(ar_from_us, test_number);
}

#[test]
fn parse_numbers_mexico() {
    let phone_util = get_phone_util();
    // Тестирование парсинга стационарных номеров Мексики.
    let mut mx_number = PhoneNumber::default();
    mx_number.country_code = 52;
    mx_number.national_number = 4499780001;

    let mut test_number = phone_util
        .parse("+52 (449)978-0001", Some(Region::MX))
        .unwrap();
    assert_eq!(mx_number, test_number);

    test_number = phone_util
        .parse("01 (449)978-0001", Some(Region::MX))
        .unwrap();
    assert_eq!(mx_number, test_number);

    test_number = phone_util.parse("(449)978-0001", Some(Region::MX)).unwrap();
    assert_eq!(mx_number, test_number);

    // Тестирование парсинга мобильных номеров Мексики.
    let mut mx_number = PhoneNumber::default();
    mx_number.country_code = 52;
    mx_number.national_number = 13312345678;

    test_number = phone_util
        .parse("+52 1 33 1234-5678", Some(Region::MX))
        .unwrap();
    assert_eq!(mx_number, test_number);

    test_number = phone_util
        .parse("044 (33) 1234-5678", Some(Region::MX))
        .unwrap();
    assert_eq!(mx_number, test_number);

    test_number = phone_util
        .parse("045 33 1234-5678", Some(Region::MX))
        .unwrap();
    assert_eq!(mx_number, test_number);
}

#[test]
fn parse_with_phone_context() {
    fn assert_throws_for_invalid_phone_context(
        phone_util: &PhoneNumberUtilInternal,
        number_to_parse: &str,
    ) {
        let result = phone_util.parse(number_to_parse, None);
        assert!(
            result.is_err(),
            "Expected an error for: {}",
            number_to_parse
        );
    }
    let phone_util = get_phone_util();
    let mut expected_number = PhoneNumber::default();
    expected_number.country_code = 64;
    expected_number.national_number = 33316005;

    // context    = ";phone-context=" descriptor
    // descriptor = domainname / global-number-digits

    // Валидные global-phone-digits
    let mut actual_number = phone_util
        .parse("tel:033316005;phone-context=+64", None)
        .unwrap();
    assert_eq!(expected_number, actual_number);

    actual_number = phone_util
        .parse(
            "tel:033316005;phone-context=+64;{this isn't part of phone-context anymore!}",
            None,
        )
        .unwrap();
    assert_eq!(expected_number, actual_number);

    expected_number.national_number = 3033316005;
    actual_number = phone_util
        .parse("tel:033316005;phone-context=+64-3", None)
        .unwrap();
    assert_eq!(expected_number, actual_number);

    expected_number.country_code = 55;
    expected_number.national_number = 5033316005;
    actual_number = phone_util
        .parse("tel:033316005;phone-context=+(555)", None)
        .unwrap();
    assert_eq!(expected_number, actual_number);

    expected_number.country_code = 1;
    expected_number.national_number = 23033316005;
    actual_number = phone_util
        .parse("tel:033316005;phone-context=+-1-2.3()", None)
        .unwrap();
    assert_eq!(expected_number, actual_number);

    // Валидный domainname
    expected_number.country_code = 64;
    expected_number.national_number = 33316005;
    actual_number = phone_util
        .parse("tel:033316005;phone-context=abc.nz", Some(Region::NZ))
        .unwrap();
    assert_eq!(expected_number, actual_number);

    actual_number = phone_util
        .parse(
            "tel:033316005;phone-context=www.PHONE-numb3r.com",
            Some(Region::NZ),
        )
        .unwrap();
    assert_eq!(expected_number, actual_number);

    actual_number = phone_util
        .parse("tel:033316005;phone-context=a", Some(Region::NZ))
        .unwrap();
    assert_eq!(expected_number, actual_number);

    actual_number = phone_util
        .parse("tel:033316005;phone-context=3phone.J.", Some(Region::NZ))
        .unwrap();
    assert_eq!(expected_number, actual_number);

    actual_number = phone_util
        .parse("tel:033316005;phone-context=a--z", Some(Region::NZ))
        .unwrap();
    assert_eq!(expected_number, actual_number);

    // Невалидный descriptor
    assert_throws_for_invalid_phone_context(&phone_util, "tel:033316005;phone-context=");
    assert_throws_for_invalid_phone_context(&phone_util, "tel:033316005;phone-context=+");
    assert_throws_for_invalid_phone_context(&phone_util, "tel:033316005;phone-context=64");
    assert_throws_for_invalid_phone_context(&phone_util, "tel:033316005;phone-context=++64");
    assert_throws_for_invalid_phone_context(&phone_util, "tel:033316005;phone-context=+abc");
    assert_throws_for_invalid_phone_context(&phone_util, "tel:033316005;phone-context=.");
    assert_throws_for_invalid_phone_context(&phone_util, "tel:033316005;phone-context=3phone");
    assert_throws_for_invalid_phone_context(&phone_util, "tel:033316005;phone-context=a-.nz");
    assert_throws_for_invalid_phone_context(&phone_util, "tel:033316005;phone-context=a{b}c");
}

#[test]
fn failed_parse_on_invalid_numbers() {
    let phone_util = get_phone_util();

    // Проверяем, что парсинг невалидных номеров завершается ошибкой.
    assert!(matches!(
        unwrap_internal(phone_util.parse("This is not a phone number", Some(Region::NZ)))
            .unwrap_err(),
        ParseError::NotANumber(_)
    ));
    assert!(matches!(
        unwrap_internal(phone_util.parse("1 Still not a number", Some(Region::NZ))).unwrap_err(),
        ParseError::NotANumber(_)
    ));
    assert!(matches!(
        unwrap_internal(phone_util.parse("1 MICROSOFT", Some(Region::NZ))).unwrap_err(),
        ParseError::NotANumber(_)
    ));
    assert!(matches!(
        unwrap_internal(phone_util.parse("12 MICROSOFT", Some(Region::NZ))).unwrap_err(),
        ParseError::NotANumber(_)
    ));
    assert_eq!(
        unwrap_internal(phone_util.parse("01495 72553301873 810104", Some(Region::GB)))
            .unwrap_err(),
        ParseError::TooLongNsn
    );
    assert!(matches!(
        unwrap_internal(phone_util.parse("+---", Some(Region::DE))).unwrap_err(),
        ParseError::NotANumber(_)
    ));
    assert!(matches!(
        unwrap_internal(phone_util.parse("+***", Some(Region::DE))).unwrap_err(),
        ParseError::NotANumber(_)
    ));
    assert!(matches!(
        unwrap_internal(phone_util.parse("+*******91", Some(Region::DE))).unwrap_err(),
        ParseError::NotANumber(_)
    ));
    assert_eq!(
        unwrap_internal(phone_util.parse("+49 0", Some(Region::DE))).unwrap_err(),
        ParseError::TooShortNsn
    );
    assert_eq!(
        unwrap_internal(phone_util.parse("+210 3456 56789", Some(Region::NZ))).unwrap_err(),
        ParseError::InvalidCountryCode
    );
    assert_eq!(
        unwrap_internal(phone_util.parse("+ 00 210 3 331 6005", Some(Region::NZ))).unwrap_err(),
        ParseError::InvalidCountryCode
    );
    assert_eq!(
        unwrap_internal(phone_util.parse("123 456 7890", Some(Region::ZZ))).unwrap_err(),
        ParseError::InvalidCountryCode
    );
    assert_eq!(
        unwrap_internal(phone_util.parse("123 456 7890", Some(Region::CS))).unwrap_err(),
        ParseError::InvalidCountryCode
    );
    assert_eq!(
        unwrap_internal(phone_util.parse("0044-----", Some(Region::GB))).unwrap_err(),
        ParseError::TooShortAfterIdd
    );
    assert_eq!(
        unwrap_internal(phone_util.parse("0044", Some(Region::GB))).unwrap_err(),
        ParseError::TooShortAfterIdd
    );
    assert_eq!(
        unwrap_internal(phone_util.parse("011", Some(Region::US))).unwrap_err(),
        ParseError::TooShortAfterIdd
    );
    assert_eq!(
        unwrap_internal(phone_util.parse("0119", Some(Region::US))).unwrap_err(),
        ParseError::TooShortAfterIdd
    );
    assert_eq!(
        unwrap_internal(phone_util.parse(
            "tel:555-1234;phone-context=www.google.com",
            Some(Region::ZZ)
        ))
        .unwrap_err(),
        ParseError::InvalidCountryCode
    );
    assert!(matches!(
        unwrap_internal(phone_util.parse("tel:555-1234;phone-context=1-331", None)).unwrap_err(),
        ParseError::NotANumber(_)
    ));
    assert!(matches!(
        unwrap_internal(phone_util.parse(";phone-context=", None)).unwrap_err(),
        ParseError::NotANumber(_)
    ));
}

#[test]
fn parse_numbers_with_plus_with_no_region() {
    let phone_util = get_phone_util();
    let mut nz_number = PhoneNumber::default();
    nz_number.country_code = 64;
    nz_number.national_number = 33316005;
    let mut result_proto = phone_util.parse("+64 3 331 6005", None).unwrap();
    assert_eq!(nz_number, result_proto);

    result_proto = phone_util.parse("\u{FF0B}64 3 331 6005", None).unwrap();
    assert_eq!(nz_number, result_proto);
    result_proto = phone_util.parse("  +64 3 331 6005", None).unwrap();
    assert_eq!(nz_number, result_proto);

    let mut toll_free_number = PhoneNumber::default();
    toll_free_number.country_code = 800;
    toll_free_number.national_number = 12345678;
    result_proto = phone_util.parse("+800 1234 5678", None).unwrap();
    assert_eq!(toll_free_number, result_proto);

    let mut universal_premium_rate = PhoneNumber::default();
    universal_premium_rate.country_code = 979;
    universal_premium_rate.national_number = 123456789;
    result_proto = phone_util.parse("+979 123 456 789", None).unwrap();
    assert_eq!(universal_premium_rate, result_proto);

    result_proto = phone_util
        .parse("tel:03-331-6005;phone-context=+64", None)
        .unwrap();
    assert_eq!(nz_number, result_proto);

    result_proto = phone_util
        .parse("  tel:03-331-6005;phone-context=+64", None)
        .unwrap();
    assert_eq!(nz_number, result_proto);

    result_proto = phone_util
        .parse("tel:03-331-6005;isub=12345;phone-context=+64", None)
        .unwrap();
    assert_eq!(nz_number, result_proto);

    nz_number.raw_input = "+64 3 331 6005".to_string().into();
    nz_number.set_country_code_source(CountryCodeSource::FromNumberWithPlusSign);
    result_proto = phone_util
        .parse_and_keep_raw_input("+64 3 331 6005", None)
        .unwrap();
    assert_eq!(nz_number, result_proto);
}

#[test]
fn parse_number_too_short_if_national_prefix_stripped() {
    let phone_util = get_phone_util();

    // Тестируем, что у номера, первые цифры которого совпадают с национальным префиксом,
    // они не удаляются, если это приведет к тому, что номер станет слишком коротким,
    // чтобы быть возможным (стандартной длины) телефонным номером для этого региона.
    let mut by_number = PhoneNumber::default();
    by_number.country_code = 375;
    by_number.national_number = 8123;
    let mut test_number = phone_util.parse("8123", Some(Region::BY)).unwrap();
    assert_eq!(by_number, test_number);

    by_number.national_number = 81234;
    test_number = phone_util.parse("81234", Some(Region::BY)).unwrap();
    assert_eq!(by_number, test_number);

    // Префикс не удаляется, так как ввод является валидным 6-значным номером,
    // в то время как результат удаления - всего 5 цифр.
    by_number.national_number = 812345;
    test_number = phone_util.parse("812345", Some(Region::BY)).unwrap();
    assert_eq!(by_number, test_number);

    // Префикс удаляется, так как возможны только 6-значные номера.
    by_number.national_number = 123456;
    test_number = phone_util.parse("8123456", Some(Region::BY)).unwrap();
    assert_eq!(by_number, test_number);
}

#[test]
fn parse_extensions() {
    let phone_util = get_phone_util();

    let mut nz_number = PhoneNumber::default();
    nz_number.country_code = 64;
    nz_number.national_number = 33316005;
    nz_number.extension = "3456".to_string().into();

    let mut test_number = phone_util
        .parse("03 331 6005 ext 3456", Some(Region::NZ))
        .unwrap();
    assert_eq!(nz_number, test_number);

    test_number = phone_util
        .parse("03 331 6005x3456", Some(Region::NZ))
        .unwrap();
    assert_eq!(nz_number, test_number);

    test_number = phone_util
        .parse("03-331 6005 int.3456", Some(Region::NZ))
        .unwrap();
    assert_eq!(nz_number, test_number);

    test_number = phone_util
        .parse("03 331 6005 #3456", Some(Region::NZ))
        .unwrap();
    assert_eq!(nz_number, test_number);

    // Тестируем, что следующие номера не извлекают добавочные номера:
    let mut non_extn_number = PhoneNumber::default();
    non_extn_number.country_code = 1;
    non_extn_number.national_number = 80074935247;

    test_number = phone_util
        .parse("1800 six-flags", Some(Region::US))
        .unwrap();
    assert_eq!(non_extn_number, test_number);

    test_number = phone_util
        .parse("1800 SIX-FLAGS", Some(Region::US))
        .unwrap();
    assert_eq!(non_extn_number, test_number);

    test_number = phone_util
        .parse("0~0 1800 7493 5247", Some(Region::PL))
        .unwrap();
    assert_eq!(non_extn_number, test_number);

    test_number = phone_util
        .parse("(1800) 7493.5247", Some(Region::US))
        .unwrap();
    assert_eq!(non_extn_number, test_number);

    // Проверяем, что соответствует последний экземпляр токена расширения.
    let mut extn_number = PhoneNumber::default();
    extn_number.country_code = 1;
    extn_number.national_number = 80074935247;
    extn_number.extension = "1234".to_string().into();
    test_number = phone_util
        .parse("0~0 1800 7493 5247 ~1234", Some(Region::PL))
        .unwrap();
    assert_eq!(extn_number, test_number);

    // Проверяем исправление ошибки, когда последняя цифра номера ранее опускалась,
    // если это был 0 при извлечении расширения. Также проверяем несколько различных
    // случаев расширений.
    let mut uk_number = PhoneNumber::default();
    uk_number.country_code = 44;
    uk_number.national_number = 2034567890;
    uk_number.extension = "456".to_string().into();

    test_number = phone_util
        .parse("+44 2034567890x456", Some(Region::NZ))
        .unwrap();
    assert_eq!(uk_number, test_number);
    test_number = phone_util
        .parse("+44 2034567890x456", Some(Region::GB))
        .unwrap();
    assert_eq!(uk_number, test_number);
    test_number = phone_util
        .parse("+44 2034567890 x456", Some(Region::GB))
        .unwrap();
    assert_eq!(uk_number, test_number);
    test_number = phone_util
        .parse("+44 2034567890 X456", Some(Region::GB))
        .unwrap();
    assert_eq!(uk_number, test_number);
    test_number = phone_util
        .parse("+44 2034567890 X 456", Some(Region::GB))
        .unwrap();
    assert_eq!(uk_number, test_number);
    test_number = phone_util
        .parse("+44 2034567890 X   456", Some(Region::GB))
        .unwrap();
    assert_eq!(uk_number, test_number);
    test_number = phone_util
        .parse("+44 2034567890 x 456  ", Some(Region::GB))
        .unwrap();
    assert_eq!(uk_number, test_number);
    test_number = phone_util
        .parse("+44 2034567890  X 456", Some(Region::GB))
        .unwrap();
    assert_eq!(uk_number, test_number);
    test_number = phone_util
        .parse("+44-2034567890;ext=456", Some(Region::GB))
        .unwrap();
    assert_eq!(uk_number, test_number);
    test_number = phone_util
        .parse("tel:2034567890;ext=456;phone-context=+44", None)
        .unwrap();
    assert_eq!(uk_number, test_number);

    // Полноширинное расширение, только "extn".
    test_number = phone_util
        .parse("+442034567890ｅｘｔｎ456", Some(Region::GB))
        .unwrap();
    assert_eq!(uk_number, test_number);
    // Только "xtn".
    test_number = phone_util
        .parse("+44-2034567890ｘｔｎ456", Some(Region::GB))
        .unwrap();
    assert_eq!(uk_number, test_number);
    // Только "xt".
    test_number = phone_util
        .parse("+44-2034567890ｘｔ456", Some(Region::GB))
        .unwrap();
    assert_eq!(uk_number, test_number);

    let mut us_with_extension = PhoneNumber::default();
    us_with_extension.country_code = 1;
    us_with_extension.national_number = 8009013355;
    us_with_extension.extension = "7246433".to_string().into();

    test_number = phone_util
        .parse("(800) 901-3355 x 7246433", Some(Region::US))
        .unwrap();
    assert_eq!(us_with_extension, test_number);
    test_number = phone_util
        .parse("(800) 901-3355 , ext 7246433", Some(Region::US))
        .unwrap();
    assert_eq!(us_with_extension, test_number);
    test_number = phone_util
        .parse("(800) 901-3355 ; 7246433", Some(Region::US))
        .unwrap();
    assert_eq!(us_with_extension, test_number);
    // Тестирование символа расширения без окружающих пробелов.
    test_number = phone_util
        .parse("(800) 901-3355;7246433", Some(Region::US))
        .unwrap();
    assert_eq!(us_with_extension, test_number);
    test_number = phone_util
        .parse("(800) 901-3355 ,extension 7246433", Some(Region::US))
        .unwrap();
    assert_eq!(us_with_extension, test_number);
    test_number = phone_util
        .parse("(800) 901-3355 ,extensión 7246433", Some(Region::US))
        .unwrap();
    assert_eq!(us_with_extension, test_number);
    // Повтор с маленькой буквой o с акутом, созданной с помощью комбинированных символов.
    test_number = phone_util
        .parse("(800) 901-3355 ,extensión 7246433", Some(Region::US))
        .unwrap();
    assert_eq!(us_with_extension, test_number);
    test_number = phone_util
        .parse("(800) 901-3355 , 7246433", Some(Region::US))
        .unwrap();
    assert_eq!(us_with_extension, test_number);
    test_number = phone_util
        .parse("(800) 901-3355 ext: 7246433", Some(Region::US))
        .unwrap();
    assert_eq!(us_with_extension, test_number);
    // Тестирование русского расширения "доб" с вариантами, найденными в интернете.
    let mut ru_with_extension = PhoneNumber::default();
    ru_with_extension.country_code = 7;
    ru_with_extension.national_number = 4232022511;
    ru_with_extension.extension = "100".to_string().into();
    test_number = phone_util
        .parse("8 (423) 202-25-11, доб. 100", Some(Region::RU))
        .unwrap();
    assert_eq!(ru_with_extension, test_number);
    test_number = phone_util
        .parse("8 (423) 202-25-11 доб. 100", Some(Region::RU))
        .unwrap();
    assert_eq!(ru_with_extension, test_number);
    test_number = phone_util
        .parse("8 (423) 202-25-11, доб 100", Some(Region::RU))
        .unwrap();
    assert_eq!(ru_with_extension, test_number);
    test_number = phone_util
        .parse("8 (423) 202-25-11 доб 100", Some(Region::RU))
        .unwrap();
    assert_eq!(ru_with_extension, test_number);
    test_number = phone_util
        .parse("8 (423) 202-25-11доб 100", Some(Region::RU))
        .unwrap();
    assert_eq!(ru_with_extension, test_number);
    // В верхнем регистре
    test_number = phone_util
        .parse("8 (423) 202-25-11 ДОБ 100", Some(Region::RU))
        .unwrap();
    assert_eq!(ru_with_extension, test_number);

    // Тестируем, что если у номера два расширения, мы игнорируем второе.
    let mut us_with_two_extensions_number = PhoneNumber::default();
    us_with_two_extensions_number.country_code = 1;
    us_with_two_extensions_number.national_number = 2121231234;
    us_with_two_extensions_number.extension = "508".to_string().into();

    test_number = phone_util
        .parse("(212)123-1234 x508/x1234", Some(Region::US))
        .unwrap();
    assert_eq!(us_with_two_extensions_number, test_number);
    test_number = phone_util
        .parse("(212)123-1234 x508/ x1234", Some(Region::US))
        .unwrap();
    assert_eq!(us_with_two_extensions_number, test_number);
    test_number = phone_util
        .parse("(212)123-1234 x508\\x1234", Some(Region::US))
        .unwrap();
    assert_eq!(us_with_two_extensions_number, test_number);

    // Тестируем парсинг номеров вида (645) 123-1234-910#, где последние 3 цифры
    // перед # - это расширение.
    let mut us_with_extension = PhoneNumber::default();
    us_with_extension.country_code = 1;
    us_with_extension.national_number = 6451231234;
    us_with_extension.extension = "910".to_string().into();
    test_number = phone_util
        .parse("+1 (645) 123 1234-910#", Some(Region::US))
        .unwrap();
    assert_eq!(us_with_extension, test_number);
}

#[test]
fn test_parse_handles_long_extensions_with_explicit_labels() {
    let phone_util = get_phone_util();
    // Тестируем верхние и нижние пределы длины добавочного номера для каждого типа метки.
    let mut nz_number = PhoneNumber::default();
    nz_number.country_code = 64;
    nz_number.national_number = 33316005;

    // Сначала в формате RFC: ext_limit_after_explicit_label
    nz_number.extension = "0".to_string().into();
    let test_number = phone_util
        .parse("tel:+6433316005;ext=0", Some(Region::NZ))
        .unwrap();
    assert_eq!(nz_number, test_number);

    nz_number.extension = "01234567890123456789".to_string().into();
    let test_number = phone_util
        .parse("tel:+6433316005;ext=01234567890123456789", Some(Region::NZ))
        .unwrap();
    assert_eq!(nz_number, test_number);

    // Слишком длинное расширение.
    let result = phone_util.parse(
        "tel:+6433316005;ext=012345678901234567890",
        Some(Region::NZ),
    );
    assert!(result.is_err());

    // Явная метка расширения: ext_limit_after_explicit_label
    nz_number.extension = "1".to_string().into();
    let test_number = phone_util
        .parse("03 3316005ext:1", Some(Region::NZ))
        .unwrap();
    assert_eq!(nz_number, test_number);

    nz_number.extension = "12345678901234567890".to_string().into();
    let test_number = phone_util
        .parse("03 3316005 xtn:12345678901234567890", Some(Region::NZ))
        .unwrap();
    assert_eq!(nz_number, test_number);

    let test_number = phone_util
        .parse(
            "03 3316005 extension\t12345678901234567890",
            Some(Region::NZ),
        )
        .unwrap();
    assert_eq!(nz_number, test_number);

    let test_number = phone_util
        .parse("03 3316005 xtensio:12345678901234567890", Some(Region::NZ))
        .unwrap();
    assert_eq!(nz_number, test_number);

    let test_number = phone_util
        .parse(
            "03 3316005 xtensión, 12345678901234567890#",
            Some(Region::NZ),
        )
        .unwrap();
    assert_eq!(nz_number, test_number);

    let test_number = phone_util
        .parse("03 3316005extension.12345678901234567890", Some(Region::NZ))
        .unwrap();
    assert_eq!(nz_number, test_number);

    let test_number = phone_util
        .parse("03 3316005 доб:12345678901234567890", Some(Region::NZ))
        .unwrap();
    assert_eq!(nz_number, test_number);

    // Слишком длинное расширение.
    let result = phone_util.parse(
        "03 3316005 extension 123456789012345678901",
        Some(Region::NZ),
    );
    assert!(result.is_err());
}

#[test]
fn test_parse_handles_long_extensions_with_auto_dialling_labels() {
    let phone_util = get_phone_util();
    // Во-вторых, случаи автодозвона и других стандартных меток добавочных номеров:
    // ext_limit_after_likely_label
    let mut us_number_user_input = PhoneNumber::default();
    us_number_user_input.country_code = 1;
    us_number_user_input.national_number = 2679000000;
    us_number_user_input.extension = "123456789012345".to_string().into();

    let mut test_number = phone_util
        .parse("+12679000000,,123456789012345#", Some(Region::US))
        .unwrap();
    assert_eq!(us_number_user_input, test_number);

    test_number = phone_util
        .parse("+12679000000;123456789012345#", Some(Region::US))
        .unwrap();
    assert_eq!(us_number_user_input, test_number);

    let mut uk_number_user_input = PhoneNumber::default();
    uk_number_user_input.country_code = 44;
    uk_number_user_input.national_number = 2034000000;
    uk_number_user_input.extension = "123456789".to_string().into();

    let test_number = phone_util
        .parse("+442034000000,,123456789#", Some(Region::GB))
        .unwrap();
    assert_eq!(uk_number_user_input, test_number);

    // Слишком длинное расширение.
    let result = phone_util.parse("+12679000000,,1234567890123456#", Some(Region::US));
    assert!(result.is_err());
}

#[test]
fn test_parse_handles_short_extensions_with_ambiguous_char() {
    let phone_util = get_phone_util();
    // В-третьих, для единичных и нестандартных случаев: ext_limit_after_ambiguous_char
    let mut nz_number = PhoneNumber::default();
    nz_number.country_code = 64;
    nz_number.national_number = 33316005;
    nz_number.extension = "123456789".to_string().into();

    let mut test_number = phone_util
        .parse("03 3316005 x 123456789", Some(Region::NZ))
        .unwrap();
    assert_eq!(nz_number, test_number);

    test_number = phone_util
        .parse("03 3316005 x. 123456789", Some(Region::NZ))
        .unwrap();
    assert_eq!(nz_number, test_number);

    test_number = phone_util
        .parse("03 3316005 #123456789#", Some(Region::NZ))
        .unwrap();
    assert_eq!(nz_number, test_number);

    test_number = phone_util
        .parse("03 3316005 ~ 123456789", Some(Region::NZ))
        .unwrap();
    assert_eq!(nz_number, test_number);

    let result = phone_util.parse("03 3316005 ~ 1234567890", Some(Region::NZ));
    assert!(result.is_err());
}

#[test]
fn test_parse_handles_short_extensions_when_not_sure_of_label() {
    let phone_util = get_phone_util();
    // В-третьих, когда нет явной метки расширения, но оно обозначено
    // конечным #: ext_limit_when_not_sure
    let mut us_number = PhoneNumber::default();
    us_number.country_code = 1;
    us_number.national_number = 1234567890;
    us_number.extension = "666666".to_string().into();

    let mut test_number = phone_util
        .parse("+1123-456-7890 666666#", Some(Region::US))
        .unwrap();
    assert_eq!(us_number, test_number);

    us_number.extension = "6".to_string().into();
    test_number = phone_util
        .parse("+11234567890-6#", Some(Region::US))
        .unwrap();
    assert_eq!(us_number, test_number);

    // Слишком длинное расширение.
    let result = phone_util.parse("+1123-456-7890 7777777#", Some(Region::US));
    assert!(result.is_err());
}

#[test]
fn can_be_internationally_dialled() {
    let phone_util = get_phone_util();
    let mut test_number = PhoneNumber::default();
    test_number.country_code = 1;

    // Toll-free in test metadata is marked as not internationally diallable.
    test_number.national_number = 8002530000;
    assert!(
        !phone_util
            .can_be_internationally_dialled(&test_number)
            .unwrap()
    );

    // Regular US number.
    test_number.national_number = 6502530000;
    assert!(
        phone_util
            .can_be_internationally_dialled(&test_number)
            .unwrap()
    );

    // No data for NZ, should default to true.
    test_number.country_code = 64;
    test_number.national_number = 33316005;
    assert!(
        phone_util
            .can_be_internationally_dialled(&test_number)
            .unwrap()
    );
}

#[test]
fn is_alpha_number() {
    let phone_util = get_phone_util();
    assert!(phone_util.is_alpha_number("1800 six-flags"));
    assert!(phone_util.is_alpha_number("1800 six-flags ext. 1234"));
    assert!(phone_util.is_alpha_number("+800 six-flags"));
    assert!(!phone_util.is_alpha_number("1800 123-1234"));
    assert!(!phone_util.is_alpha_number("1 six-flags"));
}
