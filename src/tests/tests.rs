#[cfg(test)]
use std::{cell::LazyCell, sync::LazyLock};
use std::collections::{BTreeSet, HashSet};

use dec_from_char::DecimalExtended;
#[cfg(test)]
use env_logger::Logger;
use log::trace;
use protobuf::{Message};

use crate::{
    enums::{PhoneNumberFormat, PhoneNumberType, ValidNumberLenType},
    errors::{ParseError, ValidationResultErr},
    phonemetadata::{PhoneMetadata, PhoneMetadataCollection, NumberFormat},
    phonenumber::{phone_number::CountryCodeSource, PhoneNumber},
    PhoneNumberUtil,
};

use super::region_code::RegionCode;
use crate::phonenumberutil::generated::test_metadata::TEST_METADATA;

fn get_phone_util() -> PhoneNumberUtil {
    let metadata = PhoneMetadataCollection::parse_from_bytes(&TEST_METADATA)
        .expect("Metadata should be valid");
    return PhoneNumberUtil::new_for_metadata(metadata);
}

#[test]
fn interchange_invalid_codepoints() {
    colog::default_builder()
        .filter_level(log::LevelFilter::Trace)
        .init();

    let phone_util = get_phone_util();

    let valid_inputs = vec![
        "+44\u{2013}2087654321", // U+2013, EN DASH
    ];
    for input in valid_inputs {
        assert_eq!(input, dec_from_char::normalize_decimals(input));
        assert!(phone_util.is_viable_phone_number(input));
        phone_util.parse(input, "GB").unwrap();
    }

    let invalid_inputs = vec![
        "+44\u{96}2087654321",     // Invalid sequence
        "+44\u{0096}2087654321", // U+0096
        "+44\u{fffe}2087654321", // U+FFFE
    ];
    for input in invalid_inputs {
        assert!(!phone_util.is_viable_phone_number(input));
        assert!(
            phone_util.parse(input, RegionCode::gb()).is_err_and(| err | matches!(err, ParseError::NotANumber))
        );
    }
}

#[test]
fn get_supported_regions() {
    let phone_util = get_phone_util();
    assert!(phone_util
        .get_supported_regions()
        .count() > 0
    )
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
        let region_code = phone_util.get_region_code_for_country_code(code);
        assert_eq!(RegionCode::un001(), region_code);
    }
}

#[test]
fn get_supported_calling_codes() {
    let phone_util = get_phone_util();
    let calling_codes = phone_util
        .get_supported_calling_codes()
        .collect::<Vec<_>>();
    assert!(!calling_codes.is_empty());
    for &code in &calling_codes {
        assert!(code > 0);
        let region_code = phone_util.get_region_code_for_country_code(code);
        assert_ne!(RegionCode::zz(), region_code);
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
        .get_supported_types_for_region(RegionCode::br())
        .expect("region should exist");
    assert!(types.contains(&PhoneNumberType::FixedLine));
    assert!(!types.contains(&PhoneNumberType::Mobile));
    assert!(!types.contains(&PhoneNumberType::Unknown));

    let types = phone_util
        .get_supported_types_for_region(RegionCode::us())
        .expect("region should exist");
    assert!(types.contains(&PhoneNumberType::FixedLine));
    assert!(types.contains(&PhoneNumberType::Mobile));
    assert!(!types.contains(&PhoneNumberType::FixedLineOrMobile));
    
    assert!(
        phone_util
        .get_supported_types_for_region(RegionCode::zz())
        .is_none()
    );
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
fn get_region_codes_for_country_calling_code() {
    let phone_util = get_phone_util();
    let expect_regions = |code| {
        phone_util
            .get_region_codes_for_country_calling_code(code)
            .expect("Codes should exist")
            .collect::<Vec<_>>()
    };

    let regions = expect_regions(1);
    assert!(regions.contains(&RegionCode::us()));
    assert!(regions.contains(&RegionCode::bs()));

    let regions = expect_regions(44);
    assert!(regions.contains(&RegionCode::gb()));

    let regions = expect_regions(49);
    assert!(regions.contains(&RegionCode::de()));

    let regions = expect_regions(800);
    assert!(regions.contains(&RegionCode::un001()));

    const INVALID_COUNTRY_CODE: i32 = 2;
    assert!(
        phone_util
            .get_region_codes_for_country_calling_code(INVALID_COUNTRY_CODE)
            .is_none()
    );
}

#[test]
fn get_instance_load_us_metadata() {
    let phone_util = get_phone_util();
    let metadata = phone_util.get_metadata_for_region(RegionCode::us()).unwrap();
    assert_eq!("US", metadata.id());
    assert_eq!(1, metadata.country_code());
    assert_eq!("011", metadata.international_prefix());
    assert!(metadata.has_national_prefix());
    assert_eq!(2, metadata.number_format.len());
    assert_eq!("(\\d{3})(\\d{3})(\\d{4})", metadata.number_format[1].pattern());
    assert_eq!("$1 $2 $3", metadata.number_format[1].format());
    assert_eq!("[13-689]\\d{9}|2[0-35-9]\\d{8}", metadata.general_desc.national_number_pattern());
    assert_eq!("[13-689]\\d{9}|2[0-35-9]\\d{8}", metadata.fixed_line.national_number_pattern());
    assert_eq!(1, metadata.general_desc.possible_length.len());
    assert_eq!(10, metadata.general_desc.possible_length[0]);
    assert_eq!(0, metadata.toll_free.possible_length.len());
    assert_eq!("900\\d{7}", metadata.premium_rate.national_number_pattern());
    assert!(!metadata.shared_cost.has_national_number_pattern());
}

#[test]
fn get_instance_load_de_metadata() {
    let phone_util = get_phone_util();
    let metadata = phone_util.get_metadata_for_region(RegionCode::de()).unwrap();
    assert_eq!("DE", metadata.id());
    assert_eq!(49, metadata.country_code());
    assert_eq!("00", metadata.international_prefix());
    assert_eq!("0", metadata.national_prefix());
    assert_eq!(6, metadata.number_format.len());
    assert_eq!(1, metadata.number_format[5].leading_digits_pattern.len());
    assert_eq!("900", metadata.number_format[5].leading_digits_pattern[0]);
    assert_eq!("(\\d{3})(\\d{3,4})(\\d{4})", metadata.number_format[5].pattern());
    assert_eq!(2, metadata.general_desc.possible_length_local_only.len());
    assert_eq!(8, metadata.general_desc.possible_length.len());
    assert_eq!(0, metadata.fixed_line.possible_length.len());
    assert_eq!(2, metadata.mobile.possible_length.len());
    assert_eq!("$1 $2 $3", metadata.number_format[5].format());
    assert_eq!("(?:[24-6]\\d{2}|3[03-9]\\d|[789](?:0[2-9]|[1-9]\\d))\\d{1,8}", metadata.fixed_line.national_number_pattern());
    assert_eq!("30123456", metadata.fixed_line.example_number());
    assert_eq!(10, metadata.toll_free.possible_length[0]);
    assert_eq!("900([135]\\d{6}|9\\d{7})", metadata.premium_rate.national_number_pattern());
}

#[test]
fn get_instance_load_ar_metadata() {
    let phone_util = get_phone_util();
    let metadata = phone_util.get_metadata_for_region(RegionCode::ar()).unwrap();
    assert_eq!("AR", metadata.id());
    assert_eq!(54, metadata.country_code());
    assert_eq!("00", metadata.international_prefix());
    assert_eq!("0", metadata.national_prefix());
    assert_eq!("0(?:(11|343|3715)15)?", metadata.national_prefix_for_parsing());
    assert_eq!("9$1", metadata.national_prefix_transform_rule());
    assert_eq!(5, metadata.number_format.len());
    assert_eq!("$2 15 $3-$4", metadata.number_format[2].format());
    assert_eq!("(\\d)(\\d{4})(\\d{2})(\\d{4})", metadata.number_format[3].pattern());
    assert_eq!("(\\d)(\\d{4})(\\d{2})(\\d{4})", metadata.intl_number_format[3].pattern());
    assert_eq!("$1 $2 $3 $4", metadata.intl_number_format[3].format());
}


#[test]
fn get_national_significant_number() {
    let phone_util = get_phone_util();
    let mut number = PhoneNumber::new();
    number.set_country_code(1);
    number.set_national_number(6502530000);
    let national_significant_number = phone_util.get_national_significant_number(&number);
    assert_eq!("6502530000", national_significant_number);

    number.clear();
    number.set_country_code(39);
    number.set_national_number(312345678);
    let national_significant_number = phone_util.get_national_significant_number(&number);
    assert_eq!("312345678", national_significant_number);

    number.clear();
    number.set_country_code(39);
    number.set_national_number(236618300);
    number.set_italian_leading_zero(true);
    let national_significant_number = phone_util.get_national_significant_number(&number);
    assert_eq!("0236618300", national_significant_number);

    number.clear();
    number.set_country_code(800);
    number.set_national_number(12345678);
    let national_significant_number = phone_util.get_national_significant_number(&number);
    assert_eq!("12345678", national_significant_number);
}

#[test]
fn get_national_significant_number_many_leading_zeros() {
    let phone_util = get_phone_util();
    let mut number = PhoneNumber::new();
    number.set_country_code(1);
    number.set_national_number(650);
    number.set_italian_leading_zero(true);
    number.set_number_of_leading_zeros(2);
    let national_significant_number = phone_util.get_national_significant_number(&number);
    assert_eq!("00650", national_significant_number);

    number.set_number_of_leading_zeros(-3);
    let national_significant_number = phone_util.get_national_significant_number(&number);
    assert_eq!("650", national_significant_number);
}

#[test]
fn get_example_number() {
    let phone_util = get_phone_util();
    let mut de_number = PhoneNumber::new();
    de_number.set_country_code(49);
    de_number.set_national_number(30123456);
    let test_number = phone_util.get_example_number(RegionCode::de()).unwrap();
    assert_eq!(de_number, test_number);

    let test_number = phone_util.get_example_number_for_type_and_region_code(RegionCode::de(), PhoneNumberType::FixedLine).unwrap();
    assert_eq!(de_number, test_number);
    
    let test_number = phone_util.get_example_number_for_type_and_region_code(RegionCode::de(), PhoneNumberType::FixedLineOrMobile).unwrap();
    assert_eq!(de_number, test_number);

    phone_util.get_example_number_for_type_and_region_code(RegionCode::de(), PhoneNumberType::Mobile).unwrap();
    
    let test_number = phone_util.get_example_number_for_type_and_region_code(RegionCode::us(), PhoneNumberType::VoiceMail);
    assert!(test_number.is_err());

    let test_number = phone_util
        .get_example_number_for_type_and_region_code(RegionCode::us(), PhoneNumberType::FixedLine);
    assert!(test_number.is_ok());
    assert_ne!(&PhoneNumber::new(), test_number.as_ref().unwrap());
    
    let test_number = phone_util
        .get_example_number_for_type_and_region_code(RegionCode::us(), PhoneNumberType::Mobile);
    assert!(test_number.is_ok());
    assert_ne!(&PhoneNumber::new(), test_number.as_ref().unwrap());

    assert!(phone_util.get_example_number_for_type_and_region_code(RegionCode::cs(), PhoneNumberType::Mobile).is_err());

    assert!(phone_util.get_example_number(RegionCode::un001()).is_err());
}

#[test]
fn get_invalid_example_number() {
    let phone_util = get_phone_util();
    assert!(phone_util.get_invalid_example_number(RegionCode::un001()).is_err());
    assert!(phone_util.get_invalid_example_number(RegionCode::cs()).is_err());
    
    let test_number = phone_util.get_invalid_example_number(RegionCode::us()).unwrap();
    assert_eq!(1, test_number.country_code());
    assert!(test_number.national_number() != 0);
}

#[test]
fn get_example_number_for_non_geo_entity() {
    let phone_util = get_phone_util();
    
    let mut toll_free_number = PhoneNumber::new();
    toll_free_number.set_country_code(800);
    toll_free_number.set_national_number(12345678);
    let test_number = phone_util.get_example_number_for_non_geo_entity(800).unwrap();
    assert_eq!(toll_free_number, test_number);
    
    let mut universal_premium_rate = PhoneNumber::new();
    universal_premium_rate.set_country_code(979);
    universal_premium_rate.set_national_number(123456789);
    let test_number = phone_util.get_example_number_for_non_geo_entity(979).unwrap();
    assert_eq!(universal_premium_rate, test_number);
}

#[test]
fn format_us_number() {
    let phone_util = get_phone_util();
    let mut test_number = PhoneNumber::new();
    test_number.set_country_code(1);
    test_number.set_national_number(6502530000);
    assert_eq!("650 253 0000", phone_util.format(&test_number, PhoneNumberFormat::National).unwrap());
    assert_eq!("+1 650 253 0000", phone_util.format(&test_number, PhoneNumberFormat::International).unwrap());

    test_number.set_national_number(8002530000);
    assert_eq!("800 253 0000", phone_util.format(&test_number, PhoneNumberFormat::National).unwrap());
    assert_eq!("+1 800 253 0000", phone_util.format(&test_number, PhoneNumberFormat::International).unwrap());

    test_number.set_national_number(9002530000);
    assert_eq!("900 253 0000", phone_util.format(&test_number, PhoneNumberFormat::National).unwrap());
    assert_eq!("+1 900 253 0000", phone_util.format(&test_number, PhoneNumberFormat::International).unwrap());
    assert_eq!("tel:+1-900-253-0000", phone_util.format(&test_number, PhoneNumberFormat::RFC3966).unwrap());

    test_number.set_national_number(0);
    assert_eq!("0", phone_util.format(&test_number, PhoneNumberFormat::National).unwrap());

    test_number.set_raw_input("000-000-0000".to_owned());
    assert_eq!("000-000-0000", phone_util.format(&test_number, PhoneNumberFormat::National).unwrap());
}

#[test]
fn format_bs_number() {
    let phone_util = get_phone_util();
    let mut test_number = PhoneNumber::new();
    test_number.set_country_code(1);
    test_number.set_national_number(2421234567);
    assert_eq!("242 123 4567", phone_util.format(&test_number, PhoneNumberFormat::National).unwrap());
    assert_eq!("+1 242 123 4567", phone_util.format(&test_number, PhoneNumberFormat::International).unwrap());

    test_number.set_national_number(8002530000);
    assert_eq!("800 253 0000", phone_util.format(&test_number, PhoneNumberFormat::National).unwrap());
    assert_eq!("+1 800 253 0000", phone_util.format(&test_number, PhoneNumberFormat::International).unwrap());

    test_number.set_national_number(9002530000);
    assert_eq!("900 253 0000", phone_util.format(&test_number, PhoneNumberFormat::National).unwrap());
    assert_eq!("+1 900 253 0000", phone_util.format(&test_number, PhoneNumberFormat::International).unwrap());
}

#[test]
fn format_gb_number() {
    let phone_util = get_phone_util();
    let mut test_number = PhoneNumber::new();
    test_number.set_country_code(44);
    test_number.set_national_number(2087389353);
    assert_eq!("(020) 8738 9353", phone_util.format(&test_number, PhoneNumberFormat::National).unwrap());
    assert_eq!("+44 20 8738 9353", phone_util.format(&test_number, PhoneNumberFormat::International).unwrap());
    
    test_number.set_national_number(7912345678);
    assert_eq!("(07912) 345 678", phone_util.format(&test_number, PhoneNumberFormat::National).unwrap());
    assert_eq!("+44 7912 345 678", phone_util.format(&test_number, PhoneNumberFormat::International).unwrap());
}

#[test]
fn format_de_number() {
    let phone_util = get_phone_util();
    let mut test_number = PhoneNumber::new();
    test_number.set_country_code(49);

    test_number.set_national_number(301234);
    assert_eq!("030/1234", phone_util.format(&test_number, PhoneNumberFormat::National).unwrap());
    assert_eq!("+49 30/1234", phone_util.format(&test_number, PhoneNumberFormat::International).unwrap());
    assert_eq!("tel:+49-30-1234", phone_util.format(&test_number, PhoneNumberFormat::RFC3966).unwrap());

    test_number.set_national_number(291123);
    assert_eq!("0291 123", phone_util.format(&test_number, PhoneNumberFormat::National).unwrap());
    assert_eq!("+49 291 123", phone_util.format(&test_number, PhoneNumberFormat::International).unwrap());

    test_number.set_national_number(29112345678);
    assert_eq!("0291 12345678", phone_util.format(&test_number, PhoneNumberFormat::National).unwrap());
    assert_eq!("+49 291 12345678", phone_util.format(&test_number, PhoneNumberFormat::International).unwrap());

    test_number.set_national_number(9123123);
    assert_eq!("09123 123", phone_util.format(&test_number, PhoneNumberFormat::National).unwrap());
    assert_eq!("+49 9123 123", phone_util.format(&test_number, PhoneNumberFormat::International).unwrap());

    test_number.set_national_number(80212345);
    assert_eq!("08021 2345", phone_util.format(&test_number, PhoneNumberFormat::National).unwrap());
    assert_eq!("+49 8021 2345", phone_util.format(&test_number, PhoneNumberFormat::International).unwrap());

    test_number.set_national_number(1234);
    assert_eq!("1234", phone_util.format(&test_number, PhoneNumberFormat::National).unwrap());
    assert_eq!("+49 1234", phone_util.format(&test_number, PhoneNumberFormat::International).unwrap());
}

#[test]
fn format_it_number() {
    let phone_util = get_phone_util();
    let mut test_number = PhoneNumber::new();
    test_number.set_country_code(39);
    
    test_number.set_national_number(236618300);
    test_number.set_italian_leading_zero(true);
    assert_eq!("02 3661 8300", phone_util.format(&test_number, PhoneNumberFormat::National).unwrap());
    assert_eq!("+39 02 3661 8300", phone_util.format(&test_number, PhoneNumberFormat::International).unwrap());
    assert_eq!("+390236618300", phone_util.format(&test_number, PhoneNumberFormat::E164).unwrap());

    test_number.set_national_number(345678901);
    test_number.set_italian_leading_zero(false);
    assert_eq!("345 678 901", phone_util.format(&test_number, PhoneNumberFormat::National).unwrap());
    assert_eq!("+39 345 678 901", phone_util.format(&test_number, PhoneNumberFormat::International).unwrap());
    assert_eq!("+39345678901", phone_util.format(&test_number, PhoneNumberFormat::E164).unwrap());
}

#[test]
fn format_au_number() {
    let phone_util = get_phone_util();
    let mut test_number = PhoneNumber::new();
    test_number.set_country_code(61);
    
    test_number.set_national_number(236618300);
    assert_eq!("02 3661 8300", phone_util.format(&test_number, PhoneNumberFormat::National).unwrap());
    assert_eq!("+61 2 3661 8300", phone_util.format(&test_number, PhoneNumberFormat::International).unwrap());
    assert_eq!("+61236618300", phone_util.format(&test_number, PhoneNumberFormat::E164).unwrap());
    
    test_number.set_national_number(1800123456);
    assert_eq!("1800 123 456", phone_util.format(&test_number, PhoneNumberFormat::National).unwrap());
    assert_eq!("+61 1800 123 456", phone_util.format(&test_number, PhoneNumberFormat::International).unwrap());
    assert_eq!("+611800123456", phone_util.format(&test_number, PhoneNumberFormat::E164).unwrap());
}

#[test]
fn format_ar_number() {
    let phone_util = get_phone_util();
    let mut test_number = PhoneNumber::new();
    test_number.set_country_code(54);
    
    test_number.set_national_number(1187654321);
    assert_eq!("011 8765-4321", phone_util.format(&test_number, PhoneNumberFormat::National).unwrap());
    assert_eq!("+54 11 8765-4321", phone_util.format(&test_number, PhoneNumberFormat::International).unwrap());
    assert_eq!("+541187654321", phone_util.format(&test_number, PhoneNumberFormat::E164).unwrap());
    
    test_number.set_national_number(91187654321);
    assert_eq!("011 15 8765-4321", phone_util.format(&test_number, PhoneNumberFormat::National).unwrap());
    assert_eq!("+54 9 11 8765 4321", phone_util.format(&test_number, PhoneNumberFormat::International).unwrap());
    assert_eq!("+5491187654321", phone_util.format(&test_number, PhoneNumberFormat::E164).unwrap());
}

#[test]
fn format_mx_number() {
    let phone_util = get_phone_util();
    let mut test_number = PhoneNumber::new();
    test_number.set_country_code(52);
    
    test_number.set_national_number(12345678900);
    assert_eq!("045 234 567 8900", phone_util.format(&test_number, PhoneNumberFormat::National).unwrap());
    assert_eq!("+52 1 234 567 8900", phone_util.format(&test_number, PhoneNumberFormat::International).unwrap());
    assert_eq!("+5212345678900", phone_util.format(&test_number, PhoneNumberFormat::E164).unwrap());
    
    test_number.set_national_number(15512345678);
    assert_eq!("045 55 1234 5678", phone_util.format(&test_number, PhoneNumberFormat::National).unwrap());
    assert_eq!("+52 1 55 1234 5678", phone_util.format(&test_number, PhoneNumberFormat::International).unwrap());
    assert_eq!("+5215512345678", phone_util.format(&test_number, PhoneNumberFormat::E164).unwrap());

    test_number.set_national_number(3312345678);
    assert_eq!("01 33 1234 5678", phone_util.format(&test_number, PhoneNumberFormat::National).unwrap());
    assert_eq!("+52 33 1234 5678", phone_util.format(&test_number, PhoneNumberFormat::International).unwrap());
    assert_eq!("+523312345678", phone_util.format(&test_number, PhoneNumberFormat::E164).unwrap());

    test_number.set_national_number(8211234567);
    assert_eq!("01 821 123 4567", phone_util.format(&test_number, PhoneNumberFormat::National).unwrap());
    assert_eq!("+52 821 123 4567", phone_util.format(&test_number, PhoneNumberFormat::International).unwrap());
    assert_eq!("+528211234567", phone_util.format(&test_number, PhoneNumberFormat::E164).unwrap());
}

#[test]
fn format_out_of_country_calling_number() {
    let phone_util = get_phone_util();
    let mut test_number = PhoneNumber::new();

    test_number.set_country_code(1);
    test_number.set_national_number(9002530000);
    assert_eq!("00 1 900 253 0000", phone_util.format_out_of_country_calling_number(&test_number, RegionCode::de()).unwrap());

    test_number.set_national_number(6502530000);
    assert_eq!("1 650 253 0000", phone_util.format_out_of_country_calling_number(&test_number, RegionCode::bs()).unwrap());
    assert_eq!("00 1 650 253 0000", phone_util.format_out_of_country_calling_number(&test_number, RegionCode::pl()).unwrap());

    test_number.set_country_code(44);
    test_number.set_national_number(7912345678);
    assert_eq!("011 44 7912 345 678", phone_util.format_out_of_country_calling_number(&test_number, RegionCode::us()).unwrap());

    test_number.set_country_code(49);
    test_number.set_national_number(1234);
    assert_eq!("00 49 1234", phone_util.format_out_of_country_calling_number(&test_number, RegionCode::gb()).unwrap());
    assert_eq!("1234", phone_util.format_out_of_country_calling_number(&test_number, RegionCode::de()).unwrap());

    test_number.set_country_code(39);
    test_number.set_national_number(236618300);
    test_number.set_italian_leading_zero(true);
    assert_eq!("011 39 02 3661 8300", phone_util.format_out_of_country_calling_number(&test_number, RegionCode::us()).unwrap());
    assert_eq!("02 3661 8300", phone_util.format_out_of_country_calling_number(&test_number, RegionCode::it()).unwrap());
    assert_eq!("+39 02 3661 8300", phone_util.format_out_of_country_calling_number(&test_number, RegionCode::sg()).unwrap());

    test_number.set_country_code(65);
    test_number.set_national_number(94777892);
    test_number.set_italian_leading_zero(false);
    assert_eq!("9477 7892", phone_util.format_out_of_country_calling_number(&test_number, RegionCode::sg()).unwrap());

    test_number.set_country_code(800);
    test_number.set_national_number(12345678);
    assert_eq!("011 800 1234 5678", phone_util.format_out_of_country_calling_number(&test_number, RegionCode::us()).unwrap());

    test_number.set_country_code(54);
    test_number.set_national_number(91187654321);
    assert_eq!("011 54 9 11 8765 4321", phone_util.format_out_of_country_calling_number(&test_number, RegionCode::us()).unwrap());

    test_number.set_extension("1234".to_owned());
    assert_eq!("011 54 9 11 8765 4321 ext. 1234", phone_util.format_out_of_country_calling_number(&test_number, RegionCode::us()).unwrap());
    assert_eq!("0011 54 9 11 8765 4321 ext. 1234", phone_util.format_out_of_country_calling_number(&test_number, RegionCode::au()).unwrap());
    assert_eq!("011 15 8765-4321 ext. 1234", phone_util.format_out_of_country_calling_number(&test_number, RegionCode::ar()).unwrap());
}

#[test]
fn format_out_of_country_with_invalid_region() {
    let phone_util = get_phone_util();
    let mut test_number = PhoneNumber::new();
    test_number.set_country_code(1);
    test_number.set_national_number(6502530000);
    // AQ/Antarctica is invalid, fall back to international format.
    assert_eq!("+1 650 253 0000", phone_util.format_out_of_country_calling_number(&test_number, RegionCode::aq()).unwrap());
    // For region 001, fall back to international format.
    assert_eq!("+1 650 253 0000", phone_util.format_out_of_country_calling_number(&test_number, RegionCode::un001()).unwrap());
}

#[test]
fn format_out_of_country_with_preferred_intl_prefix() {
    let phone_util = get_phone_util();
    let mut test_number = PhoneNumber::new();
    test_number.set_country_code(39);
    test_number.set_national_number(236618300);
    test_number.set_italian_leading_zero(true);

    // Should use 0011, preferred for AU.
    assert_eq!("0011 39 02 3661 8300", phone_util.format_out_of_country_calling_number(&test_number, RegionCode::au()).unwrap());
    
    // Testing preferred international prefixes with ~ (wait).
    assert_eq!("8~10 39 02 3661 8300", phone_util.format_out_of_country_calling_number(&test_number, RegionCode::uz()).unwrap());
}


#[test]
fn format_e164_number() {
    let phone_util = get_phone_util();
    let mut test_number = PhoneNumber::new();
    
    test_number.set_country_code(1);
    test_number.set_national_number(6502530000);
    assert_eq!("+16502530000", phone_util.format(&test_number, PhoneNumberFormat::E164).unwrap());
    
    test_number.set_country_code(49);
    test_number.set_national_number(301234);
    assert_eq!("+49301234", phone_util.format(&test_number, PhoneNumberFormat::E164).unwrap());
    
    test_number.set_country_code(800);
    test_number.set_national_number(12345678);
    assert_eq!("+80012345678", phone_util.format(&test_number, PhoneNumberFormat::E164).unwrap());
}

#[test]
fn format_number_with_extension() {
    let phone_util = get_phone_util();
    let mut nz_number = PhoneNumber::new();
    nz_number.set_country_code(64);
    nz_number.set_national_number(33316005);
    nz_number.set_extension("1234".to_owned());
    assert_eq!("03-331 6005 ext. 1234", phone_util.format(&nz_number, PhoneNumberFormat::National).unwrap());
    assert_eq!("tel:+64-3-331-6005;ext=1234", phone_util.format(&nz_number, PhoneNumberFormat::RFC3966).unwrap());

    let mut us_number_with_extension = PhoneNumber::new();
    us_number_with_extension.set_country_code(1);
    us_number_with_extension.set_national_number(6502530000);
    us_number_with_extension.set_extension("4567".to_owned());
    assert_eq!("650 253 0000 extn. 4567", phone_util.format(&us_number_with_extension, PhoneNumberFormat::National).unwrap());
}

#[test]
fn is_valid_number() {
    let phone_util = get_phone_util();
    let mut number = PhoneNumber::new();

    number.set_country_code(1);
    number.set_national_number(6502530000);
    assert!(phone_util.is_valid_number(&number).unwrap());

    number.clear();
    number.set_country_code(39);
    number.set_national_number(236618300);
    number.set_italian_leading_zero(true);
    assert!(phone_util.is_valid_number(&number).unwrap());
    
    number.clear();
    number.set_country_code(44);
    number.set_national_number(7912345678);
    assert!(phone_util.is_valid_number(&number).unwrap());
    
    number.clear();
    number.set_country_code(64);
    number.set_national_number(21387835);
    assert!(phone_util.is_valid_number(&number).unwrap());
    
    number.clear();
    number.set_country_code(800);
    number.set_national_number(12345678);
    assert!(phone_util.is_valid_number(&number).unwrap());

    number.clear();
    number.set_country_code(979);
    number.set_national_number(123456789);
    assert!(phone_util.is_valid_number(&number).unwrap());
}

#[test]
fn is_valid_number_for_region() {
    let phone_util = get_phone_util();
    let mut number = PhoneNumber::new();
    number.set_country_code(1);
    number.set_national_number(2423232345);
    assert!(phone_util.is_valid_number(&number).unwrap());
    assert!(phone_util.is_valid_number_for_region(&number, RegionCode::bs()));
    assert!(!phone_util.is_valid_number_for_region(&number, RegionCode::us()));
    
    // Now an invalid number for BS
    number.set_national_number(2421232345);
    assert!(!phone_util.is_valid_number(&number).unwrap());

    // La Mayotte and Réunion
    let mut re_number = PhoneNumber::new();
    re_number.set_country_code(262);
    re_number.set_national_number(262123456);
    assert!(phone_util.is_valid_number(&re_number).unwrap());
    assert!(phone_util.is_valid_number_for_region(&re_number, RegionCode::re()));
    assert!(!phone_util.is_valid_number_for_region(&re_number, RegionCode::yt()));
    
    re_number.set_national_number(269601234);
    assert!(phone_util.is_valid_number_for_region(&re_number, RegionCode::yt()));
    assert!(!phone_util.is_valid_number_for_region(&re_number, RegionCode::re()));

    // This number is valid in both.
    re_number.set_national_number(800123456);
    assert!(phone_util.is_valid_number_for_region(&re_number, RegionCode::yt()));
    assert!(phone_util.is_valid_number_for_region(&re_number, RegionCode::re()));

    let mut intl_toll_free = PhoneNumber::new();
    intl_toll_free.set_country_code(800);
    intl_toll_free.set_national_number(12345678);
    assert!(phone_util.is_valid_number_for_region(&intl_toll_free, RegionCode::un001()));
    assert!(!phone_util.is_valid_number_for_region(&intl_toll_free, RegionCode::us()));
    assert!(!phone_util.is_valid_number_for_region(&intl_toll_free, RegionCode::zz()));

    let mut invalid_number = PhoneNumber::new();
    invalid_number.set_country_code(3923);
    invalid_number.set_national_number(2366);
    assert!(!phone_util.is_valid_number_for_region(&invalid_number, RegionCode::zz()));
    assert!(!phone_util.is_valid_number_for_region(&invalid_number, RegionCode::un001()));
    
    invalid_number.set_country_code(0);
    assert!(!phone_util.is_valid_number_for_region(&invalid_number, RegionCode::un001()));
    assert!(!phone_util.is_valid_number_for_region(&invalid_number, RegionCode::zz()));
}

#[test]
fn is_not_valid_number() {
    let phone_util = get_phone_util();
    let mut number = PhoneNumber::new();
    
    number.set_country_code(1);
    number.set_national_number(2530000);
    assert!(!phone_util.is_valid_number(&number).unwrap());

    number.clear();
    number.set_country_code(39);
    number.set_national_number(23661830000);
    number.set_italian_leading_zero(true);
    assert!(!phone_util.is_valid_number(&number).unwrap());

    number.clear();
    number.set_country_code(44);
    number.set_national_number(791234567);
    assert!(!phone_util.is_valid_number(&number).unwrap());

    number.clear();
    number.set_country_code(49);
    number.set_national_number(1234);
    assert!(!phone_util.is_valid_number(&number).unwrap());

    number.clear();
    number.set_country_code(64);
    number.set_national_number(3316005);
    assert!(!phone_util.is_valid_number(&number).unwrap());

    number.clear();
    number.set_country_code(3923);
    number.set_national_number(2366);
    assert!(!phone_util.is_valid_number(&number).unwrap());
    
    number.set_country_code(0);
    assert!(!phone_util.is_valid_number(&number).unwrap());
    
    number.clear();
    number.set_country_code(800);
    number.set_national_number(123456789);
    assert!(!phone_util.is_valid_number(&number).unwrap());
}

#[test]
fn get_region_code_for_number() {
    let phone_util = get_phone_util();
    let mut number = PhoneNumber::new();
    
    number.set_country_code(1);
    number.set_national_number(2423232345);
    assert_eq!(RegionCode::bs(), phone_util.get_region_code_for_number(&number).unwrap());
    
    number.set_national_number(4241231234);
    assert_eq!(RegionCode::us(), phone_util.get_region_code_for_number(&number).unwrap());
    
    number.set_country_code(44);
    number.set_national_number(7912345678);
    assert_eq!(RegionCode::gb(), phone_util.get_region_code_for_number(&number).unwrap());
    
    number.set_country_code(800);
    number.set_national_number(12345678);
    assert_eq!(RegionCode::un001(), phone_util.get_region_code_for_number(&number).unwrap());
    
    number.set_country_code(979);
    number.set_national_number(123456789);
    assert_eq!(RegionCode::un001(), phone_util.get_region_code_for_number(&number).unwrap());
}


#[test]
fn is_possible_number() {
    let phone_util = get_phone_util();
    let mut number = PhoneNumber::new();
    number.set_country_code(1);
    number.set_national_number(6502530000);
    assert!(phone_util.is_possible_number(&number));
    number.set_national_number(2530000);
    assert!(phone_util.is_possible_number(&number));
    
    number.set_country_code(44);
    number.set_national_number(2070313000);
    assert!(phone_util.is_possible_number(&number));
    
    number.set_country_code(800);
    number.set_national_number(12345678);
    assert!(phone_util.is_possible_number(&number));

    assert!(phone_util.is_possible_number_for_string("+1 650 253 0000", RegionCode::us()));
    assert!(phone_util.is_possible_number_for_string("+1 650 GOO OGLE", RegionCode::us()));
    assert!(phone_util.is_possible_number_for_string("(650) 253-0000", RegionCode::us()));
    assert!(phone_util.is_possible_number_for_string("253-0000", RegionCode::us()));
    assert!(phone_util.is_possible_number_for_string("+1 650 253 0000", RegionCode::gb()));
    assert!(phone_util.is_possible_number_for_string("+44 20 7031 3000", RegionCode::gb()));
    assert!(phone_util.is_possible_number_for_string("(020) 7031 300", RegionCode::gb()));
    assert!(phone_util.is_possible_number_for_string("7031 3000", RegionCode::gb()));
    assert!(phone_util.is_possible_number_for_string("3331 6005", RegionCode::nz()));
    assert!(phone_util.is_possible_number_for_string("+800 1234 5678", RegionCode::un001()));
}

#[test]
fn is_not_possible_number() {
    let phone_util = get_phone_util();
    let mut number = PhoneNumber::new();
    
    number.set_country_code(1);
    number.set_national_number(65025300000);
    assert!(!phone_util.is_possible_number(&number));
    
    number.set_country_code(800);
    number.set_national_number(123456789);
    assert!(!phone_util.is_possible_number(&number));

    number.set_country_code(1);
    number.set_national_number(253000);
    assert!(!phone_util.is_possible_number(&number));

    number.set_country_code(44);
    number.set_national_number(300);
    assert!(!phone_util.is_possible_number(&number));

    assert!(!phone_util.is_possible_number_for_string("+1 650 253 00000", RegionCode::us()));
    assert!(!phone_util.is_possible_number_for_string("(650) 253-00000", RegionCode::us()));
    assert!(!phone_util.is_possible_number_for_string("I want a Pizza", RegionCode::us()));
    assert!(!phone_util.is_possible_number_for_string("253-000", RegionCode::us()));
    assert!(!phone_util.is_possible_number_for_string("1 3000", RegionCode::gb()));
    assert!(!phone_util.is_possible_number_for_string("+44 300", RegionCode::gb()));
    assert!(!phone_util.is_possible_number_for_string("+800 1234 5678 9", RegionCode::un001()));
}


#[test]
fn is_possible_number_with_reason() {
    let phone_util = get_phone_util();
    let mut number = PhoneNumber::new();

    number.set_country_code(1);
    number.set_national_number(6502530000);
    assert_eq!(Ok(ValidNumberLenType::IsPossible), phone_util.is_possible_number_with_reason(&number));

    number.set_national_number(2530000);
    assert_eq!(Ok(ValidNumberLenType::IsPossibleLocalOnly), phone_util.is_possible_number_with_reason(&number));
    
    number.set_country_code(0);
    assert_eq!(Err(ValidationResultErr::InvalidCountryCode), phone_util.is_possible_number_with_reason(&number));

    number.set_country_code(1);
    number.set_national_number(253000);
    assert_eq!(Err(ValidationResultErr::TooShort), phone_util.is_possible_number_with_reason(&number));

    number.set_national_number(65025300000);
    assert_eq!(Err(ValidationResultErr::TooLong), phone_util.is_possible_number_with_reason(&number));

    number.set_country_code(44);
    number.set_national_number(2070310000);
    assert_eq!(Ok(ValidNumberLenType::IsPossible), phone_util.is_possible_number_with_reason(&number));

    number.set_country_code(49);
    number.set_national_number(30123456);
    assert_eq!(Ok(ValidNumberLenType::IsPossible), phone_util.is_possible_number_with_reason(&number));

    number.set_country_code(65);
    number.set_national_number(1234567890);
    assert_eq!(Ok(ValidNumberLenType::IsPossible), phone_util.is_possible_number_with_reason(&number));

    number.set_country_code(800);
    number.set_national_number(123456789);
    assert_eq!(Err(ValidationResultErr::TooLong), phone_util.is_possible_number_with_reason(&number));
}

#[test]
fn is_possible_number_for_type_with_reason() {
    let phone_util = get_phone_util();
    let mut ar_number = PhoneNumber::new();
    ar_number.set_country_code(54);

    ar_number.set_national_number(12345);
    assert_eq!(Err(ValidationResultErr::TooShort), phone_util.is_possible_number_for_type_with_reason(&ar_number, PhoneNumberType::Unknown));
    assert_eq!(Err(ValidationResultErr::TooShort), phone_util.is_possible_number_for_type_with_reason(&ar_number, PhoneNumberType::FixedLine));

    ar_number.set_national_number(123456);
    assert_eq!(Ok(ValidNumberLenType::IsPossible), phone_util.is_possible_number_for_type_with_reason(&ar_number, PhoneNumberType::Unknown));
    assert_eq!(Ok(ValidNumberLenType::IsPossible), phone_util.is_possible_number_for_type_with_reason(&ar_number, PhoneNumberType::FixedLine));
    assert_eq!(Err(ValidationResultErr::TooShort), phone_util.is_possible_number_for_type_with_reason(&ar_number, PhoneNumberType::Mobile));
    assert_eq!(Err(ValidationResultErr::TooShort), phone_util.is_possible_number_for_type_with_reason(&ar_number, PhoneNumberType::TollFree));

    ar_number.set_national_number(12345678901);
    assert_eq!(Ok(ValidNumberLenType::IsPossible), phone_util.is_possible_number_for_type_with_reason(&ar_number, PhoneNumberType::Unknown));
    assert_eq!(Err(ValidationResultErr::TooLong), phone_util.is_possible_number_for_type_with_reason(&ar_number, PhoneNumberType::FixedLine));
    assert_eq!(Ok(ValidNumberLenType::IsPossible), phone_util.is_possible_number_for_type_with_reason(&ar_number, PhoneNumberType::Mobile));
    assert_eq!(Err(ValidationResultErr::TooLong), phone_util.is_possible_number_for_type_with_reason(&ar_number, PhoneNumberType::TollFree));
    
    let mut de_number = PhoneNumber::new();
    de_number.set_country_code(49);
    de_number.set_national_number(12);
    assert_eq!(Ok(ValidNumberLenType::IsPossibleLocalOnly), phone_util.is_possible_number_for_type_with_reason(&de_number, PhoneNumberType::Unknown));
    assert_eq!(Ok(ValidNumberLenType::IsPossibleLocalOnly), phone_util.is_possible_number_for_type_with_reason(&de_number, PhoneNumberType::FixedLine));
    assert_eq!(Err(ValidationResultErr::TooShort), phone_util.is_possible_number_for_type_with_reason(&de_number, PhoneNumberType::Mobile));

    let mut br_number = PhoneNumber::new();
    br_number.set_country_code(55);
    br_number.set_national_number(12345678);
    assert_eq!(Err(ValidationResultErr::InvalidLength), phone_util.is_possible_number_for_type_with_reason(&br_number, PhoneNumberType::Mobile));
    assert_eq!(Ok(ValidNumberLenType::IsPossibleLocalOnly), phone_util.is_possible_number_for_type_with_reason(&br_number, PhoneNumberType::FixedLineOrMobile));
}

#[test]
fn truncate_too_long_number() {
    let phone_util = get_phone_util();

    let mut too_long_number = phone_util.parse("+165025300001", RegionCode::us()).unwrap();
    let valid_number = phone_util.parse("+16502530000", RegionCode::us()).unwrap();
    assert!(phone_util.truncate_too_long_number(&mut too_long_number).unwrap());
    assert_eq!(valid_number, too_long_number);
    
    let mut valid_number_copy = valid_number.clone();
    assert!(phone_util.truncate_too_long_number(&mut valid_number_copy).unwrap());
    assert_eq!(valid_number, valid_number_copy);

    let mut too_short_number = phone_util.parse("+11234", RegionCode::us()).unwrap();
    let too_short_number_copy = too_short_number.clone();
    assert!(!phone_util.truncate_too_long_number(&mut too_short_number).unwrap());
    assert_eq!(too_short_number_copy, too_short_number);
}

#[test]
fn normalise_remove_punctuation() {
    let phone_util = get_phone_util();
    let input_number = "034-56&+#2\u{ad}34".to_string();
    let normalized_number = phone_util.normalize(&input_number);
    let expected_output = "03456234";
    assert_eq!(expected_output, normalized_number, "Conversion did not correctly remove punctuation");
}

#[test]
fn normalise_replace_alpha_characters() {
    let phone_util = get_phone_util();
    let input_number = "034-I-am-HUNGRY".to_string();
    let normalized_number = phone_util.normalize(&input_number);
    let expected_output = "034426486479";
    assert_eq!(expected_output, normalized_number, "Conversion did not correctly replace alpha characters");
}

#[test]
fn normalise_other_digits() {
    let phone_util = get_phone_util();
    // Full-width 2, Arabic-indic 5
    let input = "\u{ff12}5\u{0665}"; // "２5٥"
    assert_eq!("255", phone_util.normalize(&input));
    
    // Eastern-Arabic 5 and 0
    let input = "\u{06f5}2\u{06f0}"; // "۵2۰"
    assert_eq!("520", phone_util.normalize(&input));
}

#[test]
fn normalise_strip_alpha_characters() {
    let phone_util = get_phone_util();
    let input_number = "034-56&+a#234".to_string();
    let normalized_number = phone_util.normalize_digits_only(&input_number);
    let expected_output = "03456234";
    assert_eq!(expected_output, normalized_number, "Conversion did not correctly remove alpha characters");
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
    assert_eq!(expected_extension, extension.unwrap());
}

#[test]
fn get_number_type() {
    let phone_util = get_phone_util();
    let mut number = PhoneNumber::new();
    
    // PREMIUM_RATE
    number.set_country_code(1); number.set_national_number(9004433030);
    assert_eq!(PhoneNumberType::PremiumRate, phone_util.get_number_type(&number).unwrap());
    number.set_country_code(44); number.set_national_number(9187654321);
    assert_eq!(PhoneNumberType::PremiumRate, phone_util.get_number_type(&number).unwrap());

    // TOLL_FREE
    number.set_country_code(1); number.set_national_number(8881234567);
    assert_eq!(PhoneNumberType::TollFree, phone_util.get_number_type(&number).unwrap());
    number.set_country_code(44); number.set_national_number(8012345678);
    assert_eq!(PhoneNumberType::TollFree, phone_util.get_number_type(&number).unwrap());
    number.set_country_code(800); number.set_national_number(12345678);
    assert_eq!(PhoneNumberType::TollFree, phone_util.get_number_type(&number).unwrap());

    // MOBILE
    number.set_country_code(1); number.set_national_number(2423570000);
    assert_eq!(PhoneNumberType::Mobile, phone_util.get_number_type(&number).unwrap());
    number.set_country_code(44); number.set_national_number(7912345678);
    assert_eq!(PhoneNumberType::Mobile, phone_util.get_number_type(&number).unwrap());

    // FIXED_LINE
    number.set_country_code(1); number.set_national_number(2423651234);
    assert_eq!(PhoneNumberType::FixedLine, phone_util.get_number_type(&number).unwrap());
    number.clear(); number.set_country_code(39); number.set_national_number(236618300); number.set_italian_leading_zero(true);
    assert_eq!(PhoneNumberType::FixedLine, phone_util.get_number_type(&number).unwrap());
    number.clear(); number.set_country_code(44); number.set_national_number(2012345678);
    assert_eq!(PhoneNumberType::FixedLine, phone_util.get_number_type(&number).unwrap());

    // FIXED_LINE_OR_MOBILE
    number.clear(); number.set_country_code(1); number.set_national_number(6502531111);
    assert_eq!(PhoneNumberType::FixedLineOrMobile, phone_util.get_number_type(&number).unwrap());
    
    // SHARED_COST
    number.clear(); number.set_country_code(44); number.set_national_number(8431231234);
    assert_eq!(PhoneNumberType::SharedCost, phone_util.get_number_type(&number).unwrap());
    
    // VOIP
    number.clear(); number.set_country_code(44); number.set_national_number(5631231234);
    assert_eq!(PhoneNumberType::VoIP, phone_util.get_number_type(&number).unwrap());
    
    // PERSONAL_NUMBER
    number.clear(); number.set_country_code(44); number.set_national_number(7031231234);
    assert_eq!(PhoneNumberType::PersonalNumber, phone_util.get_number_type(&number).unwrap());
    
    // UNKNOWN
    number.clear(); number.set_country_code(1); number.set_national_number(65025311111);
    assert_eq!(PhoneNumberType::Unknown, phone_util.get_number_type(&number).unwrap());
}

#[test]
fn parse_national_number() {
    let phone_util = get_phone_util();
    let mut nz_number = PhoneNumber::new();
    nz_number.set_country_code(64);
    nz_number.set_national_number(33316005);
    
    let test_number = phone_util
        .parse("033316005", RegionCode::nz())
        .unwrap();
    assert_eq!(nz_number, test_number);
    assert!(!test_number.has_country_code_source());
    assert_eq!(CountryCodeSource::UNSPECIFIED, test_number.country_code_source());

    let test_number = phone_util.parse("33316005", RegionCode::nz()).unwrap();
    assert_eq!(nz_number, test_number);
    
    let test_number = phone_util.parse("03-331 6005", RegionCode::nz()).unwrap();
    assert_eq!(nz_number, test_number);

    let test_number = phone_util.parse("tel:03-331-6005;phone-context=+64", RegionCode::nz()).unwrap();
    assert_eq!(nz_number, test_number);
}

#[test]
fn failed_parse_on_invalid_numbers() {
    let phone_util = get_phone_util();
    assert!(matches!(phone_util.parse("This is not a phone number", RegionCode::nz()), Err(ParseError::ExtractNumberError(_))));
    assert!(matches!(phone_util.parse("01495 72553301873 810104", RegionCode::gb()), Err(ParseError::TooLongNsn)));
    assert!(matches!(phone_util.parse("123 456 7890", RegionCode::get_unknown()), Err(ParseError::InvalidCountryCodeError)));
    assert!(matches!(phone_util.parse("+---", RegionCode::de()), Err(ParseError::ExtractNumberError(_))));
    assert!(matches!(phone_util.parse("+49 0", RegionCode::de()), Err(ParseError::TooShortNsn)));
    assert!(matches!(phone_util.parse("0044", RegionCode::gb()), Err(ParseError::TooShortAfterIdd)));
}

#[test]
fn parse_extensions() {
    let phone_util = get_phone_util();
    let mut nz_number = PhoneNumber::new();
    nz_number.set_country_code(64);
    nz_number.set_national_number(33316005);
    nz_number.set_extension("3456".to_owned());
    
    let test_number = phone_util
        .parse("03 331 6005 ext 3456", RegionCode::nz()).unwrap();
    assert_eq!(nz_number, test_number);
    
    let test_number = phone_util
        .parse("03 331 6005 #3456", RegionCode::nz()).unwrap();
    assert_eq!(nz_number, test_number);

    let test_number = phone_util
        .parse("03 331 6005x3456", RegionCode::nz()).unwrap();
    assert_eq!(nz_number, test_number);
}

#[test]
fn can_be_internationally_dialled() {
    let phone_util = get_phone_util();
    let mut test_number = PhoneNumber::new();
    test_number.set_country_code(1);

    // Toll-free in test metadata is marked as not internationally diallable.
    test_number.set_national_number(8002530000);
    assert!(!phone_util.can_be_internationally_dialled(&test_number).unwrap());

    // Regular US number.
    test_number.set_national_number(6502530000);
    assert!(phone_util.can_be_internationally_dialled(&test_number).unwrap());

    // No data for NZ, should default to true.
    test_number.set_country_code(64);
    test_number.set_national_number(33316005);
    assert!(phone_util.can_be_internationally_dialled(&test_number).unwrap());
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