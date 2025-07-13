use protobuf::{Message, MessageField};

use crate::{
    enums::{PhoneNumberFormat, PhoneNumberType, NumberLengthType},
    errors::{ParseError, ValidationError},
    phonemetadata::{NumberFormat, PhoneMetadata, PhoneMetadataCollection, PhoneNumberDesc},
    phonenumber::{phone_number::CountryCodeSource, PhoneNumber},
    PhoneNumberUtil,
};

use super::region_code::RegionCode;
use crate::generated::metadata::TEST_METADATA;

static ONCE: std::sync::Once = std::sync::Once::new();

#[cfg(test)]
fn get_phone_util() -> PhoneNumberUtil {
    ONCE.call_once(||colog::default_builder()
        .filter_level(log::LevelFilter::Trace)
        .init()
    );

    let metadata = PhoneMetadataCollection::parse_from_bytes(&TEST_METADATA)
        .expect("Metadata should be valid");
    return PhoneNumberUtil::new_for_metadata(metadata);
}

#[test]
fn interchange_invalid_codepoints() {
    let phone_util = get_phone_util();

    let valid_inputs = vec![
        "+44\u{2013}2087654321", // U+2013, EN DASH
    ];
    for input in valid_inputs {
        assert_eq!(input, dec_from_char::normalize_decimals(input));
        assert!(phone_util.is_viable_phone_number(input));
        phone_util.parse(input, RegionCode::gb()).unwrap();
    }

    let invalid_inputs = vec![
        "+44\u{96}2087654321",     // Invalid sequence
        "+44\u{0096}2087654321", // U+0096
        "+44\u{fffe}2087654321", // U+FFFE
    ];
    for input in invalid_inputs {
        assert!(!phone_util.is_viable_phone_number(input));
        assert!(
            phone_util.parse(input, RegionCode::gb()).is_err_and(| err | matches!(err, ParseError::NotANumber(_)))
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
    assert_eq!(RegionCode::us(), metadata.id());
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
    assert_eq!(RegionCode::de(), metadata.id());
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
    assert_eq!(RegionCode::ar(), metadata.id());
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
fn get_example_number_without_region() {
    let phone_util = get_phone_util();

    // В наших тестовых метаданных мы не покрываем все типы; в реальных метаданных — покрываем.
    // Проверяем, что вызов для получения примера номера завершился успешно,
    // и что номер был изменен.
    let test_number = phone_util.get_example_number_for_type(PhoneNumberType::FixedLine).unwrap();
    assert_ne!(PhoneNumber::new(), test_number);

    let test_number = phone_util.get_example_number_for_type(PhoneNumberType::Mobile).unwrap();
    assert_ne!(PhoneNumber::new(), test_number);

    let test_number = phone_util.get_example_number_for_type(PhoneNumberType::PremiumRate).unwrap();
    assert_ne!(PhoneNumber::new(), test_number);
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
fn format_out_of_country_keeping_alpha_chars() {
    let phone_util = get_phone_util();
    let mut alpha_numeric_number = phone_util.parse_and_keep_raw_input("1800 six-flag", RegionCode::us()).unwrap();
    
    let formatted_number = phone_util.format_out_of_country_keeping_alpha_chars(&alpha_numeric_number, RegionCode::au()).unwrap();
    assert_eq!("0011 1 800 SIX-FLAG", formatted_number);

    // Formatting from within the NANPA region.
    let formatted_number = phone_util.format_out_of_country_keeping_alpha_chars(&alpha_numeric_number, RegionCode::us()).unwrap();
    assert_eq!("1 800 SIX-FLAG", formatted_number);

    // Testing a number with extension.
    let alpha_numeric_number_with_extn = phone_util.parse_and_keep_raw_input("800 SIX-flag ext. 1234", RegionCode::us()).unwrap();
    let formatted_number = phone_util.format_out_of_country_keeping_alpha_chars(&alpha_numeric_number_with_extn, RegionCode::au()).unwrap();
    assert_eq!("0011 1 800 SIX-FLAG extn. 1234", formatted_number);

    // Testing that if the raw input doesn't exist, it is formatted using FormatOutOfCountryCallingNumber.
    alpha_numeric_number.clear_raw_input();
    let formatted_number = phone_util.format_out_of_country_keeping_alpha_chars(&alpha_numeric_number, RegionCode::de()).unwrap();
    assert_eq!("00 1 800 749 3524", formatted_number);
}

#[test]
fn format_with_carrier_code() {
    let phone_util = get_phone_util();
    
    let mut ar_number = PhoneNumber::new();
    ar_number.set_country_code(54);
    ar_number.set_national_number(91234125678);

    let formatted = phone_util.format(&ar_number, PhoneNumberFormat::National).unwrap();
    assert_eq!("01234 12-5678", formatted);

    let formatted = phone_util.format_national_number_with_carrier_code(&ar_number, "15").unwrap();
    assert_eq!("01234 15 12-5678", formatted);
    
    let formatted = phone_util.format_national_number_with_carrier_code(&ar_number, "").unwrap();
    assert_eq!("01234 12-5678", formatted);

    let formatted = phone_util.format(&ar_number, PhoneNumberFormat::E164).unwrap();
    assert_eq!("+5491234125678", formatted);

    let mut us_number = PhoneNumber::new();
    us_number.set_country_code(1);
    us_number.set_national_number(4241231234);

    let formatted = phone_util.format(&us_number, PhoneNumberFormat::National).unwrap();
    assert_eq!("424 123 1234", formatted);
    
    let formatted = phone_util.format_national_number_with_carrier_code(&us_number, "15").unwrap();
    assert_eq!("424 123 1234", formatted);

    let mut invalid_number = PhoneNumber::new();
    invalid_number.set_country_code(0);
    invalid_number.set_national_number(12345);
    
    let formatted = phone_util.format_national_number_with_carrier_code(&invalid_number, "89").unwrap();
    assert_eq!("12345", formatted);
}

// Весь код, который написан - корректен и компилируется
#[test]
fn format_with_preferred_carrier_code() {
    let phone_util = get_phone_util();
    let mut ar_number = PhoneNumber::new();
    ar_number.set_country_code(54);
    ar_number.set_national_number(91234125678);

    // Тестируем форматирование без предпочтительного кода оператора в самом номере.
    let formatted = phone_util.format_national_number_with_preferred_carrier_code(&ar_number, "15").unwrap();
    assert_eq!("01234 15 12-5678", formatted);

    let formatted = phone_util.format_national_number_with_preferred_carrier_code(&ar_number, "").unwrap();
    assert_eq!("01234 12-5678", formatted);

    // Тестируем форматирование с установленным предпочтительным кодом оператора.
    ar_number.set_preferred_domestic_carrier_code("19".to_string());
    let formatted = phone_util.format(&ar_number, PhoneNumberFormat::National).unwrap();
    assert_eq!("01234 12-5678", formatted);

    let formatted = phone_util
        .format_national_number_with_preferred_carrier_code(&ar_number, "15").unwrap();
    assert_eq!("01234 19 12-5678", formatted);

    let formatted = phone_util
        .format_national_number_with_preferred_carrier_code(&ar_number, "").unwrap();
    assert_eq!("01234 19 12-5678", formatted);

    // Если preferred_domestic_carrier_code присутствует (даже если это просто пробел),
    // используется он, а не код оператора по умолчанию.
    ar_number.set_preferred_domestic_carrier_code(" ".to_string());
    let formatted = phone_util.format_national_number_with_preferred_carrier_code(&ar_number, "15").unwrap();
    assert_eq!("01234   12-5678", formatted);

    // Если preferred_domestic_carrier_code присутствует, но пуст, он игнорируется,
    // и используется код оператора по умолчанию.
    ar_number.set_preferred_domestic_carrier_code("".to_string());
    let formatted = phone_util.format_national_number_with_preferred_carrier_code(&ar_number, "15").unwrap();
    assert_eq!("01234 15 12-5678", formatted);

    // Для США эта функция не поддерживается, поэтому изменений быть не должно.
    let mut us_number = PhoneNumber::new();
    us_number.set_country_code(1);
    us_number.set_national_number(4241231234);
    us_number.set_preferred_domestic_carrier_code("99".to_string());

    let formatted = phone_util.format(&us_number, PhoneNumberFormat::National).unwrap();
    assert_eq!("424 123 1234", formatted);

    let formatted = phone_util.format_national_number_with_preferred_carrier_code(&us_number, "15").unwrap();
    assert_eq!("424 123 1234", formatted);
}

#[test]
fn format_number_for_mobile_dialing() {
    let phone_util = get_phone_util();
    let mut test_number = PhoneNumber::new();

    // Номера обычно набираются в национальном формате внутри страны и
    // в международном формате из-за пределов страны.
    test_number.set_country_code(57);
    test_number.set_national_number(6012345678);
    let formatted_number = phone_util.format_number_for_mobile_dialing(&test_number, "CO", false).unwrap();
    assert_eq!("6012345678", formatted_number);

    test_number.set_country_code(49);
    test_number.set_national_number(30123456);
    let formatted_number = phone_util.format_number_for_mobile_dialing(&test_number, "DE", false).unwrap();
    assert_eq!("030123456", formatted_number);
    let formatted_number = phone_util.format_number_for_mobile_dialing(&test_number, "CH", false).unwrap();
    assert_eq!("+4930123456", formatted_number);

    test_number.set_extension("1234".to_string());
    let formatted_number = phone_util.format_number_for_mobile_dialing(&test_number, "DE", false).unwrap();
    assert_eq!("030123456", formatted_number);
    let formatted_number = phone_util.format_number_for_mobile_dialing(&test_number, "CH", false).unwrap();
    assert_eq!("+4930123456", formatted_number);

    test_number.set_country_code(1);
    test_number.clear_extension();
    // Бесплатные номера США помечены как noInternationalDialing в тестовых метаданных
    // для целей тестирования. Для таких номеров мы ожидаем, что ничего не будет
    // возвращено, если код региона не совпадает.
    test_number.set_national_number(8002530000);
    let formatted_number = phone_util.format_number_for_mobile_dialing(&test_number, "US", true).unwrap();
    assert_eq!("800 253 0000", formatted_number);
    let formatted_number = phone_util.format_number_for_mobile_dialing(&test_number, "CN", true).unwrap();
    assert_eq!("", formatted_number);
    let formatted_number = phone_util.format_number_for_mobile_dialing(&test_number, "US", false).unwrap();
    assert_eq!("8002530000", formatted_number);
    let formatted_number = phone_util.format_number_for_mobile_dialing(&test_number, "CN", false).unwrap();
    assert_eq!("", formatted_number);

    test_number.set_national_number(6502530000);
    let formatted_number = phone_util
        .format_number_for_mobile_dialing(&test_number, "US", true)
        .unwrap();
    assert_eq!("+1 650 253 0000", formatted_number);
    let formatted_number = phone_util
        .format_number_for_mobile_dialing(&test_number, "US", false)
        .unwrap();
    assert_eq!("+16502530000", formatted_number);

    test_number.set_extension("1234".to_string());
    let formatted_number = phone_util.format_number_for_mobile_dialing(&test_number, "US", true).unwrap();
    assert_eq!("+1 650 253 0000", formatted_number);
    let formatted_number = phone_util.format_number_for_mobile_dialing(&test_number, "US", false).unwrap();
    assert_eq!("+16502530000", formatted_number);

    // Невалидный номер США, который на одну цифру длиннее.
    test_number.clear_extension();
    test_number.set_national_number(65025300001);
    let formatted_number = phone_util.format_number_for_mobile_dialing(&test_number, "US", true).unwrap();
    assert_eq!("+1 65025300001", formatted_number);
    let formatted_number = phone_util.format_number_for_mobile_dialing(&test_number, "US", false).unwrap();
    assert_eq!("+165025300001", formatted_number);

    // Номера со звёздочкой. В реальности они есть в Израиле, но в наших
    // тестовых метаданных они есть для Японии (JP).
    test_number.set_country_code(81);
    test_number.set_national_number(2345);
    let formatted_number = phone_util.format_number_for_mobile_dialing(&test_number, "JP", true).unwrap();
    assert_eq!("*2345", formatted_number);
    let formatted_number = phone_util.format_number_for_mobile_dialing(&test_number, "JP", false).unwrap();
    assert_eq!("*2345", formatted_number);

    test_number.set_country_code(800);
    test_number.set_national_number(12345678);
    let formatted_number = phone_util.format_number_for_mobile_dialing(&test_number, "JP", false).unwrap();
    assert_eq!("+80012345678", formatted_number);
    let formatted_number = phone_util.format_number_for_mobile_dialing(&test_number, "JP", true).unwrap();
    assert_eq!("+800 1234 5678", formatted_number);

    // Номера ОАЭ, начинающиеся с 600 (классифицируются как UAN), должны набираться
    // без +971 на местном уровне.
    test_number.set_country_code(971);
    test_number.set_national_number(600123456);
    let formatted_number = phone_util.format_number_for_mobile_dialing(&test_number, "JP", false).unwrap();
    assert_eq!("+971600123456", formatted_number);
    let formatted_number = phone_util.format_number_for_mobile_dialing(&test_number, "AE", true).unwrap();
    assert_eq!("600123456", formatted_number);

    test_number.set_country_code(52);
    test_number.set_national_number(3312345678);
    let formatted_number = phone_util.format_number_for_mobile_dialing(&test_number, "MX", false).unwrap();
    assert_eq!("+523312345678", formatted_number);
    let formatted_number = phone_util.format_number_for_mobile_dialing(&test_number, "US", false).unwrap();
    assert_eq!("+523312345678", formatted_number);

    // Проверяем, что узбекские номера возвращаются в международном формате, даже
    // если набираются из того же региона или других регионов.
    // Стационарный номер
    test_number.set_country_code(998);
    test_number.set_national_number(612201234);
    let formatted_number = phone_util.format_number_for_mobile_dialing(&test_number, "UZ", false).unwrap();
    assert_eq!("+998612201234", formatted_number);
    // Мобильный номер
    test_number.set_national_number(950123456);
    let formatted_number = phone_util.format_number_for_mobile_dialing(&test_number, "UZ", false).unwrap();
    assert_eq!("+998950123456", formatted_number);
    let formatted_number = phone_util.format_number_for_mobile_dialing(&test_number, "US", false).unwrap();
    assert_eq!("+998950123456", formatted_number);

    // Негеографические номера всегда должны набираться в международном формате.
    test_number.set_country_code(800);
    test_number.set_national_number(12345678);
    let formatted_number = phone_util.format_number_for_mobile_dialing(&test_number, "US", false).unwrap();
    assert_eq!("+80012345678", formatted_number);
    let formatted_number = phone_util.format_number_for_mobile_dialing(&test_number, "001", false).unwrap();
    assert_eq!("+80012345678", formatted_number);

    // Тестируем, что короткий номер форматируется корректно для мобильного набора
    // внутри региона и не может быть набран из-за его пределов.
    test_number.set_country_code(49);
    test_number.set_national_number(123);
    let formatted_number = phone_util.format_number_for_mobile_dialing(&test_number, "DE", false).unwrap();
    assert_eq!("123", formatted_number);
    let formatted_number = phone_util.format_number_for_mobile_dialing(&test_number, "IT", false).unwrap();
    assert_eq!("", formatted_number);

    // Тестируем специальную логику для стран NANPA, где номера обычной длины
    // всегда выводятся в международном формате, а короткие — в национальном.
    test_number.set_country_code(1);
    test_number.set_national_number(6502530000);
    let formatted_number = phone_util.format_number_for_mobile_dialing(&test_number, "US", false).unwrap();
    assert_eq!("+16502530000", formatted_number);
    let formatted_number = phone_util.format_number_for_mobile_dialing(&test_number, "CA", false).unwrap();
    assert_eq!("+16502530000", formatted_number);
    let formatted_number = phone_util.format_number_for_mobile_dialing(&test_number, "BR", false).unwrap();
    assert_eq!("+16502530000", formatted_number);
    test_number.set_national_number(911);
    let formatted_number = phone_util.format_number_for_mobile_dialing(&test_number, "US", false).unwrap();
    assert_eq!("911", formatted_number);
    let formatted_number = phone_util.format_number_for_mobile_dialing(&test_number, "CA", false).unwrap();
    assert_eq!("", formatted_number);
    let formatted_number = phone_util.format_number_for_mobile_dialing(&test_number, "BR", false).unwrap();
    assert_eq!("", formatted_number);

    // Тестируем, что австралийский номер экстренной службы 000 форматируется корректно.
    test_number.set_country_code(61);
    test_number.set_national_number(0);
    test_number.set_italian_leading_zero(true);
    test_number.set_number_of_leading_zeros(2);
    let formatted_number = phone_util.format_number_for_mobile_dialing(&test_number, "AU", false).unwrap();
    assert_eq!("000", formatted_number);
    let formatted_number = phone_util.format_number_for_mobile_dialing(&test_number, "NZ", false).unwrap();
    assert_eq!("", formatted_number);
}

#[test]
fn format_by_pattern() {
    let phone_util = get_phone_util();
    let mut test_number = PhoneNumber::new();
    let mut number_format = NumberFormat::new();

    test_number.set_country_code(1);
    test_number.set_national_number(6502530000);

    number_format.set_pattern("(\\d{3})(\\d{3})(\\d{4})".to_string());
    number_format.set_format("($1) $2-$3".to_string());
    
    let number_formats = vec![number_format.clone()];

    let formatted_number = phone_util
        .format_by_pattern(&test_number, PhoneNumberFormat::National, &number_formats)
        .unwrap();
    assert_eq!("(650) 253-0000", formatted_number);

    let formatted_number = phone_util
        .format_by_pattern(&test_number, PhoneNumberFormat::International, &number_formats)
        .unwrap();
    assert_eq!("+1 (650) 253-0000", formatted_number);

    let formatted_number = phone_util
        .format_by_pattern(&test_number, PhoneNumberFormat::RFC3966, &number_formats)
        .unwrap();
    assert_eq!("tel:+1-650-253-0000", formatted_number);

    // $NP устанавливается в '1' для США. Здесь мы проверяем, что для других стран
    // NANPA (Североамериканский план нумерации) правила США соблюдаются.
    number_format.set_national_prefix_formatting_rule("$NP ($FG)".to_string());
    number_format.set_format("$1 $2-$3".to_string());
    let number_formats = vec![number_format.clone()];
    
    test_number.set_country_code(1);
    test_number.set_national_number(4168819999);

    let formatted_number = phone_util
        .format_by_pattern(&test_number, PhoneNumberFormat::National, &number_formats)
        .unwrap();
    assert_eq!("1 (416) 881-9999", formatted_number);

    let formatted_number = phone_util
        .format_by_pattern(&test_number, PhoneNumberFormat::International, &number_formats)
        .unwrap();
    assert_eq!("+1 416 881-9999", formatted_number);
    
    test_number.set_country_code(39);
    test_number.set_national_number(236618300);
    test_number.set_italian_leading_zero(true);

    number_format.set_pattern("(\\d{2})(\\d{5})(\\d{3})".to_string());
    number_format.set_format("$1-$2 $3".to_string());
    number_format.clear_national_prefix_formatting_rule();
    let number_formats = vec![number_format.clone()];

    let formatted_number = phone_util
        .format_by_pattern(&test_number, PhoneNumberFormat::National, &number_formats)
        .unwrap();
    assert_eq!("02-36618 300", formatted_number);

    let formatted_number = phone_util
        .format_by_pattern(&test_number, PhoneNumberFormat::International, &number_formats)
        .unwrap();
    assert_eq!("+39 02-36618 300", formatted_number);
    
    test_number.set_country_code(44);
    test_number.set_national_number(2012345678);
    test_number.set_italian_leading_zero(false);
    
    number_format.set_national_prefix_formatting_rule("$NP$FG".to_string());
    number_format.set_pattern("(\\d{2})(\\d{4})(\\d{4})".to_string());
    number_format.set_format("$1 $2 $3".to_string());
    let mut number_formats = vec![number_format]; // mutable vec to modify the element inside

    let formatted_number = phone_util
        .format_by_pattern(&test_number, PhoneNumberFormat::National, &number_formats)
        .unwrap();
    assert_eq!("020 1234 5678", formatted_number);
    
    number_formats[0].set_national_prefix_formatting_rule("($NP$FG)".to_string());
    let formatted_number = phone_util
        .format_by_pattern(&test_number, PhoneNumberFormat::National, &number_formats)
        .unwrap();
    assert_eq!("(020) 1234 5678", formatted_number);
    
    number_formats[0].clear_national_prefix_formatting_rule();
    let formatted_number = phone_util
        .format_by_pattern(&test_number, PhoneNumberFormat::National, &number_formats)
        .unwrap();
    assert_eq!("20 1234 5678", formatted_number);

    let formatted_number = phone_util
        .format_by_pattern(&test_number, PhoneNumberFormat::International, &number_formats)
        .unwrap();
    assert_eq!("+44 20 1234 5678", formatted_number);
}

#[test]
fn format_in_original_format() {
    let phone_util = get_phone_util();

    let mut phone_number = phone_util.parse_and_keep_raw_input("+442087654321", RegionCode::gb()).unwrap();
    let formatted_number = phone_util.format_in_original_format(&phone_number, RegionCode::gb()).unwrap();
    assert_eq!("+44 20 8765 4321", formatted_number);

    phone_number = phone_util.parse_and_keep_raw_input("02087654321", RegionCode::gb()).unwrap();
    let formatted_number = phone_util.format_in_original_format(&phone_number, RegionCode::gb()).unwrap();
    assert_eq!("(020) 8765 4321", formatted_number);

    phone_number = phone_util.parse_and_keep_raw_input("011442087654321", RegionCode::us()).unwrap();
    let formatted_number = phone_util.format_in_original_format(&phone_number, RegionCode::us()).unwrap();
    assert_eq!("011 44 20 8765 4321", formatted_number);

    phone_number = phone_util.parse_and_keep_raw_input("442087654321", RegionCode::gb()).unwrap();
    let formatted_number = phone_util.format_in_original_format(&phone_number, RegionCode::gb()).unwrap();
    assert_eq!("44 20 8765 4321", formatted_number);

    // Если номер парсится без сохранения исходного ввода, `format_in_original_format`
    // должен вернуться к стандартному национальному формату.
    phone_number = phone_util.parse("+442087654321", RegionCode::gb()).unwrap();
    let formatted_number = phone_util.format_in_original_format(&phone_number, RegionCode::gb()).unwrap();
    assert_eq!("(020) 8765 4321", formatted_number);

    // Невалидные номера, для которых есть шаблон форматирования, должны быть отформатированы
    // правильно. Примечание: коды регионов, начинающиеся с 7, намеренно исключены
    // из тестовых метаданных для целей тестирования.
    phone_number = phone_util.parse_and_keep_raw_input("7345678901", RegionCode::us()).unwrap();
    let formatted_number = phone_util.format_in_original_format(&phone_number, RegionCode::us()).unwrap();
    assert_eq!("734 567 8901", formatted_number);

    // США не является страной с ведущим нулём, и его наличие
    // заставляет нас форматировать номер с использованием raw_input.
    phone_number = phone_util.parse_and_keep_raw_input("0734567 8901", RegionCode::us()).unwrap();
    let formatted_number = phone_util.format_in_original_format(&phone_number, RegionCode::us()).unwrap();
    assert_eq!("0734567 8901", formatted_number);

    // Этот номер валиден, но у нас нет для него шаблона форматирования.
    // Возвращаемся к исходному вводу.
    phone_number = phone_util.parse_and_keep_raw_input("02-4567-8900", RegionCode::kr()).unwrap();
    let formatted_number = phone_util.format_in_original_format(&phone_number, RegionCode::kr()).unwrap();
    assert_eq!("02-4567-8900", formatted_number);

    phone_number = phone_util.parse_and_keep_raw_input("01180012345678", RegionCode::us()).unwrap();
    let formatted_number = phone_util.format_in_original_format(&phone_number, RegionCode::us()).unwrap();
    assert_eq!("011 800 1234 5678", formatted_number);

    phone_number = phone_util.parse_and_keep_raw_input("+80012345678", RegionCode::kr()).unwrap();
    let formatted_number = phone_util.format_in_original_format(&phone_number, RegionCode::kr()).unwrap();
    assert_eq!("+800 1234 5678", formatted_number);

    // Местные номера США форматируются корректно, так как у нас есть для них шаблоны.
    phone_number = phone_util.parse_and_keep_raw_input("2530000", RegionCode::us()).unwrap();
    let formatted_number = phone_util.format_in_original_format(&phone_number, RegionCode::us()).unwrap();
    assert_eq!("253 0000", formatted_number);

    // Номер с национальным префиксом в США.
    phone_number = phone_util.parse_and_keep_raw_input("18003456789", RegionCode::us()).unwrap();
    let formatted_number = phone_util.format_in_original_format(&phone_number, RegionCode::us()).unwrap();
    assert_eq!("1 800 345 6789", formatted_number);

    // Номер без национального префикса в Великобритании.
    phone_number = phone_util.parse_and_keep_raw_input("2087654321", RegionCode::gb()).unwrap();
    let formatted_number = phone_util.format_in_original_format(&phone_number, RegionCode::gb()).unwrap();
    assert_eq!("20 8765 4321", formatted_number);

    // Убедимся, что метаданные не были изменены в результате предыдущего вызова.
    phone_number = phone_util.parse("+442087654321", RegionCode::gb()).unwrap();
    let formatted_number = phone_util.format_in_original_format(&phone_number, RegionCode::gb()).unwrap();
    assert_eq!("(020) 8765 4321", formatted_number);

    // Номер с национальным префиксом в Мексике.
    phone_number = phone_util.parse_and_keep_raw_input("013312345678", RegionCode::mx()).unwrap();
    let formatted_number = phone_util.format_in_original_format(&phone_number, RegionCode::mx()).unwrap();
    assert_eq!("01 33 1234 5678", formatted_number);

    // Номер без национального префикса в Мексике.
    phone_number = phone_util.parse_and_keep_raw_input("3312345678", RegionCode::mx()).unwrap();
    let formatted_number = phone_util.format_in_original_format(&phone_number, RegionCode::mx()).unwrap();
    assert_eq!("33 1234 5678", formatted_number);

    // Итальянский стационарный номер.
    phone_number = phone_util.parse_and_keep_raw_input("0212345678", RegionCode::it()).unwrap();
    let formatted_number = phone_util.format_in_original_format(&phone_number, RegionCode::it()).unwrap();
    assert_eq!("02 1234 5678", formatted_number);

    // Номер с национальным префиксом в Японии.
    phone_number = phone_util.parse_and_keep_raw_input("00777012", RegionCode::jp()).unwrap();
    let formatted_number = phone_util.format_in_original_format(&phone_number, RegionCode::jp()).unwrap();
    assert_eq!("0077-7012", formatted_number);

    // Номер без национального префикса в Японии.
    phone_number = phone_util.parse_and_keep_raw_input("0777012", RegionCode::jp()).unwrap();
    let formatted_number = phone_util.format_in_original_format(&phone_number, RegionCode::jp()).unwrap();
    assert_eq!("0777012", formatted_number);

    // Номер с кодом оператора в Бразилии.
    phone_number = phone_util.parse_and_keep_raw_input("012 3121286979", RegionCode::br()).unwrap();
    let formatted_number = phone_util.format_in_original_format(&phone_number, RegionCode::br()).unwrap();
    assert_eq!("012 3121286979", formatted_number);

    // Национальный префикс по умолчанию в этом случае — 045. Когда вводится номер
    // с префиксом 044, мы возвращаем исходный ввод, так как не хотим менять введенный номер.
    phone_number = phone_util.parse_and_keep_raw_input("044(33)1234-5678", RegionCode::mx()).unwrap();
    let formatted_number = phone_util.format_in_original_format(&phone_number, RegionCode::mx()).unwrap();
    assert_eq!("044(33)1234-5678", formatted_number);

    phone_number = phone_util.parse_and_keep_raw_input("045(33)1234-5678", RegionCode::mx()).unwrap();
    let formatted_number = phone_util.format_in_original_format(&phone_number, RegionCode::mx()).unwrap();
    assert_eq!("045 33 1234 5678", formatted_number);

    // Международный префикс по умолчанию в этом случае — 0011. Когда вводится номер
    // с префиксом 0012, мы возвращаем исходный ввод.
    phone_number = phone_util.parse_and_keep_raw_input("0012 16502530000", RegionCode::au()).unwrap();
    let formatted_number = phone_util.format_in_original_format(&phone_number, RegionCode::au()).unwrap();
    assert_eq!("0012 16502530000", formatted_number);

    phone_number = phone_util.parse_and_keep_raw_input("0011 16502530000", RegionCode::au()).unwrap();
    let formatted_number = phone_util.format_in_original_format(&phone_number, RegionCode::au()).unwrap();
    assert_eq!("0011 1 650 253 0000", formatted_number);

    // Проверяем, что знак звёздочки (*) не удаляется и не добавляется к исходному вводу.
    phone_number = phone_util.parse_and_keep_raw_input("*1234", RegionCode::jp()).unwrap();
    let formatted_number = phone_util.format_in_original_format(&phone_number, RegionCode::jp()).unwrap();
    assert_eq!("*1234", formatted_number);

    phone_number = phone_util.parse_and_keep_raw_input("1234", RegionCode::jp()).unwrap();
    let formatted_number = phone_util.format_in_original_format(&phone_number, RegionCode::jp()).unwrap();
    assert_eq!("1234", formatted_number);

    // Проверяем, что невалидный национальный номер без исходного ввода просто
    // форматируется как национальный номер.
    let mut phone_number = PhoneNumber::new();
    phone_number.set_country_code_source(CountryCodeSource::FROM_DEFAULT_COUNTRY);
    phone_number.set_country_code(1);
    phone_number.set_national_number(650253000);
    let formatted_number = phone_util.format_in_original_format(&phone_number, RegionCode::us()).unwrap();
    assert_eq!("650253000", formatted_number);
}

#[test]
fn parse_and_keep_raw() {
    let phone_util = get_phone_util();
    let mut alpha_numeric_number = PhoneNumber::new();
    alpha_numeric_number.set_country_code(1);
    alpha_numeric_number.set_national_number(80074935247);
    alpha_numeric_number.set_raw_input("800 six-flags".to_string());
    alpha_numeric_number.set_country_code_source(CountryCodeSource::FROM_DEFAULT_COUNTRY);

    let test_number = phone_util.parse_and_keep_raw_input("800 six-flags", RegionCode::us()).unwrap();
    assert_eq!(alpha_numeric_number, test_number);
    
    alpha_numeric_number.set_national_number(8007493524);
    alpha_numeric_number.set_raw_input("1800 six-flag".to_string());
    alpha_numeric_number.set_country_code_source(CountryCodeSource::FROM_NUMBER_WITHOUT_PLUS_SIGN);
    let test_number = phone_util.parse_and_keep_raw_input("1800 six-flag", RegionCode::us()).unwrap();
    assert_eq!(alpha_numeric_number, test_number);

    alpha_numeric_number.set_raw_input("+1800 six-flag".to_string());
    alpha_numeric_number.set_country_code_source(CountryCodeSource::FROM_NUMBER_WITH_PLUS_SIGN);
    let test_number = phone_util.parse_and_keep_raw_input("+1800 six-flag", RegionCode::cn()).unwrap();
    assert_eq!(alpha_numeric_number, test_number);

    alpha_numeric_number.set_raw_input("001800 six-flag".to_string());
    alpha_numeric_number.set_country_code_source(CountryCodeSource::FROM_NUMBER_WITH_IDD);
    let test_number = phone_util.parse_and_keep_raw_input("001800 six-flag", RegionCode::nz()).unwrap();
    assert_eq!(alpha_numeric_number, test_number);

    // Попробуем с невалидным регионом - ожидаем ошибку.
    let result = phone_util.parse("123 456 7890", RegionCode::cs());
    assert!(result.is_err());
    
    let mut korean_number = PhoneNumber::new();
    korean_number.set_country_code(82);
    korean_number.set_national_number(22123456);
    korean_number.set_raw_input("08122123456".to_string());
    korean_number.set_country_code_source(CountryCodeSource::FROM_DEFAULT_COUNTRY);
    korean_number.set_preferred_domestic_carrier_code("81".to_string());
    let test_number = phone_util.parse_and_keep_raw_input("08122123456", RegionCode::kr()).unwrap();
    assert_eq!(korean_number, test_number);
}

#[test]
fn parse_italian_leading_zeros() {
    let phone_util = get_phone_util();
    let mut zeros_number = PhoneNumber::new();
    zeros_number.set_country_code(61);

    // Тестируем номер "011".
    zeros_number.set_national_number(11);
    zeros_number.set_italian_leading_zero(true);
    // `number_of_leading_zeros` по умолчанию равен 1, поэтому его не устанавливаем.
    let test_number = phone_util.parse("011", RegionCode::au()).unwrap();
    assert_eq!(zeros_number, test_number);

    // Тестируем номер "001".
    zeros_number.set_national_number(1);
    zeros_number.set_italian_leading_zero(true);
    zeros_number.set_number_of_leading_zeros(2);
    let test_number = phone_util.parse("001", RegionCode::au()).unwrap();
    assert_eq!(zeros_number, test_number);

    // Тестируем номер "000". Этот номер имеет 2 ведущих нуля.
    zeros_number.set_national_number(0);
    zeros_number.set_italian_leading_zero(true);
    zeros_number.set_number_of_leading_zeros(2);
    let test_number = phone_util.parse("000", RegionCode::au()).unwrap();
    assert_eq!(zeros_number, test_number);

    // Тестируем номер "0000". Этот номер имеет 3 ведущих нуля.
    zeros_number.set_national_number(0);
    zeros_number.set_italian_leading_zero(true);
    zeros_number.set_number_of_leading_zeros(3);
    let test_number = phone_util.parse("0000", RegionCode::au()).unwrap();
    assert_eq!(zeros_number, test_number);
}

#[test]
fn maybe_strip_national_prefix_and_carrier_code() {
    let phone_util = get_phone_util();
    let mut metadata = PhoneMetadata::new();
    let general_desc = PhoneNumberDesc::new();
    metadata.general_desc = MessageField::some(general_desc);
    metadata.general_desc
        .as_mut()
        .map(| m | m.set_national_number_pattern("\\d{4,8}".to_string()));
    
    metadata.set_national_prefix_for_parsing("34".to_string());
    let number_to_strip = "34356778".to_string();
    let phone_number_and_carrier_code = phone_util
        .maybe_strip_national_prefix_and_carrier_code(&metadata, &number_to_strip)
        .unwrap();

    assert_eq!("356778", phone_number_and_carrier_code.0, "Should have had national prefix stripped.");
    assert_eq!(None, phone_number_and_carrier_code.1, "Should have had no carrier code stripped.");

    // Повторная попытка удаления - теперь номер не должен начинаться с национального префикса,
    // поэтому дальнейшее удаление не должно происходить.
    let phone_number_and_carrier_code = phone_util
        .maybe_strip_national_prefix_and_carrier_code(&metadata, &number_to_strip)
        .unwrap();

    assert_eq!("356778", phone_number_and_carrier_code.0, "Should have had no change - no national prefix present.");

    // В некоторых странах нет национального префикса. Повторяем тест без указания префикса.
    metadata.clear_national_prefix_for_parsing();
        let phone_number_and_carrier_code = phone_util
        .maybe_strip_national_prefix_and_carrier_code(&metadata, &number_to_strip)
        .unwrap();

    assert!(phone_number_and_carrier_code.1.is_none(), "Should have had no change - empty national prefix.");

    // Если результирующий номер не соответствует национальному правилу, он не должен быть удален.
    metadata.set_national_prefix_for_parsing("3".to_string());
    let number_to_strip = "3123".to_string();
    let phone_number_and_carrier_code = phone_util
        .maybe_strip_national_prefix_and_carrier_code(&metadata, &number_to_strip)
        .unwrap();
    assert_eq!("3123", phone_number_and_carrier_code.0, "Should have had no change - after stripping, it wouldn't have matched the national rule.");

    // Тестируем извлечение кода выбора оператора.
    metadata.set_national_prefix_for_parsing("0(81)?".to_string());
    let number_to_strip = "08122123456".to_string();
let phone_number_and_carrier_code = phone_util
        .maybe_strip_national_prefix_and_carrier_code(&metadata, &number_to_strip)
        .unwrap();
    assert_eq!(Some("81"), phone_number_and_carrier_code.1, "Should have had carrier code stripped.");
    assert_eq!("22123456", phone_number_and_carrier_code.0, "Should have had national prefix and carrier code stripped.");

    // Если было правило преобразования, проверяем, что оно было применено.
    // There is a regex difference how transform do works in rust and cpp.
    // Since patterns in metadata.xml only ends with $\d and no rules like this appears
    // we can do this. But this should be handled on any changes
    metadata.set_national_prefix_transform_rule("5${1}5".to_string());
    // Обратите внимание, что здесь присутствует захватывающая группа.
    metadata.set_national_prefix_for_parsing("0(\\d{2})".to_string());
    let number_to_strip = "031123".to_string();
    let phone_number_and_carrier_code = phone_util
            .maybe_strip_national_prefix_and_carrier_code(&metadata, &number_to_strip)
            .unwrap();
            
    assert_eq!("5315123", phone_number_and_carrier_code.0, "Was not successfully transformed.");
}


#[test]
fn format_out_of_country_with_invalid_region() {
    let phone_util = get_phone_util();
    let mut test_number = PhoneNumber::new();
    test_number.set_country_code(1);
    test_number.set_national_number(6502530000);

    // AQ/Антарктида не является валидным кодом региона для форматирования номеров,
    // поэтому используется международный формат.
    let formatted_number = phone_util
        .format_out_of_country_calling_number(&test_number, RegionCode::aq())
        .unwrap();
    assert_eq!("+1 650 253 0000", formatted_number);

    // Для кода региона 001 формат для звонков из-за пределов страны всегда
    // превращается в международный формат.
    let formatted_number = phone_util
        .format_out_of_country_calling_number(&test_number, RegionCode::un001())
        .unwrap();
    assert_eq!("+1 650 253 0000", formatted_number);
}

#[test]
fn format_out_of_country_with_preferred_intl_prefix() {
    let phone_util = get_phone_util();
    let mut test_number = PhoneNumber::new();
    test_number.set_country_code(39);
    test_number.set_national_number(236618300);
    test_number.set_italian_leading_zero(true);

    // Должен использоваться префикс 0011, так как это предпочтительный международный
    // префикс для Австралии (в наших тестовых метаданных и 0011, и 0012 принимаются
    // как возможные международные префиксы).
    let formatted_number = phone_util
        .format_out_of_country_calling_number(&test_number, RegionCode::au())
        .unwrap();
    assert_eq!("0011 39 02 3661 8300", formatted_number);

    // Тестируем поддержку предпочтительных международных префиксов с символом ~,
    // который обозначает ожидание.
    let formatted_number = phone_util
        .format_out_of_country_calling_number(&test_number, RegionCode::uz())
        .unwrap();
    assert_eq!("8~10 39 02 3661 8300", formatted_number);
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
fn get_length_of_geographical_area_code() {
    let phone_util = get_phone_util();
    let mut number = PhoneNumber::new();

    // Google MTV, с кодом города "650".
    number.set_country_code(1);
    number.set_national_number(6502530000);
    assert_eq!(3, phone_util.get_length_of_geographical_area_code(&number).unwrap());

    // Бесплатный номер в Северной Америке, без кода города.
    number.set_country_code(1);
    number.set_national_number(8002530000);
    assert_eq!(0, phone_util.get_length_of_geographical_area_code(&number).unwrap());

    // Невалидный номер США (на 1 цифру короче), без кода города.
    number.set_country_code(1);
    number.set_national_number(650253000);
    assert_eq!(0, phone_util.get_length_of_geographical_area_code(&number).unwrap());

    // Google London, с кодом города "20".
    number.set_country_code(44);
    number.set_national_number(2070313000);
    assert_eq!(2, phone_util.get_length_of_geographical_area_code(&number).unwrap());

    // Мобильный номер в Великобритании не имеет кода города.
    number.set_country_code(44);
    number.set_national_number(7912345678);
    assert_eq!(0, phone_util.get_length_of_geographical_area_code(&number).unwrap());

    // Google Buenos Aires, с кодом города "11".
    number.set_country_code(54);
    number.set_national_number(1155303000);
    assert_eq!(2, phone_util.get_length_of_geographical_area_code(&number).unwrap());

    // Мобильный номер в Аргентине также имеет код города.
    number.set_country_code(54);
    number.set_national_number(91187654321);
    assert_eq!(3, phone_util.get_length_of_geographical_area_code(&number).unwrap());

    // Google Sydney, с кодом города "2".
    number.set_country_code(61);
    number.set_national_number(293744000);
    assert_eq!(1, phone_util.get_length_of_geographical_area_code(&number).unwrap());

    // Номера Мексики - нет национального префикса, но есть код города.
    number.set_country_code(52);
    number.set_national_number(3312345678);
    assert_eq!(2, phone_util.get_length_of_geographical_area_code(&number).unwrap());

    // Итальянские номера - нет национального префикса, но есть код города.
    number.set_country_code(39);
    number.set_national_number(236618300);
    number.set_italian_leading_zero(true);
    assert_eq!(2, phone_util.get_length_of_geographical_area_code(&number).unwrap());

    // Google Singapore. В Сингапуре нет кода города и национального префикса.
    number.set_country_code(65);
    number.set_national_number(65218000);
    number.set_italian_leading_zero(false);
    assert_eq!(0, phone_util.get_length_of_geographical_area_code(&number).unwrap());

    // Международный бесплатный номер, без кода города.
    number.set_country_code(800);
    number.set_national_number(12345678);
    assert_eq!(0, phone_util.get_length_of_geographical_area_code(&number).unwrap());

    // Мобильный номер из Китая является географическим, но не имеет кода города.
    let mut cn_mobile = PhoneNumber::new();
    cn_mobile.set_country_code(86);
    cn_mobile.set_national_number(18912341234);
    assert_eq!(0, phone_util.get_length_of_geographical_area_code(&cn_mobile).unwrap());
}

#[test]
fn get_length_of_national_destination_code() {
    let phone_util = get_phone_util();
    let mut number = PhoneNumber::new();

    // Google MTV, с национальным кодом назначения (NDC) "650".
    number.set_country_code(1);
    number.set_national_number(6502530000);
    assert_eq!(3, phone_util.get_length_of_national_destination_code(&number).unwrap());

    // Бесплатный номер Северной Америки, с NDC "800".
    number.set_country_code(1);
    number.set_national_number(8002530000);
    assert_eq!(3, phone_util.get_length_of_national_destination_code(&number).unwrap());

    // Google London, с NDC "20".
    number.set_country_code(44);
    number.set_national_number(2070313000);
    assert_eq!(2, phone_util.get_length_of_national_destination_code(&number).unwrap());

    // Мобильный телефон в Великобритании, с NDC "7912".
    number.set_country_code(44);
    number.set_national_number(7912345678);
    assert_eq!(4, phone_util.get_length_of_national_destination_code(&number).unwrap());

    // Google Buenos Aires, с NDC "11".
    number.set_country_code(54);
    number.set_national_number(1155303000);
    assert_eq!(2, phone_util.get_length_of_national_destination_code(&number).unwrap());

    // Аргентинский мобильный, с NDC "911".
    number.set_country_code(54);
    number.set_national_number(91187654321);
    assert_eq!(3, phone_util.get_length_of_national_destination_code(&number).unwrap());

    // Google Sydney, с NDC "2".
    number.set_country_code(61);
    number.set_national_number(293744000);
    assert_eq!(1, phone_util.get_length_of_national_destination_code(&number).unwrap());

    // Google Singapore. Сингапур имеет NDC "6521".
    number.set_country_code(65);
    number.set_national_number(65218000);
    assert_eq!(4, phone_util.get_length_of_national_destination_code(&number).unwrap());

    // Невалидный номер США (на 1 цифру короче), без NDC.
    number.set_country_code(1);
    number.set_national_number(650253000);
    assert_eq!(0, phone_util.get_length_of_national_destination_code(&number).unwrap());

    // Номер с невалидным кодом страны, не должен иметь NDC.
    number.set_country_code(123);
    number.set_national_number(650253000);
    assert_eq!(0, phone_util.get_length_of_national_destination_code(&number).unwrap());

    // Номер, который имеет только одну группу цифр после кода страны при
    // форматировании в международном формате.
    number.set_country_code(376);
    number.set_national_number(12345);
    assert_eq!(0, phone_util.get_length_of_national_destination_code(&number).unwrap());

    // Тот же номер, но с добавочным.
    number.set_extension("321".to_string());
    assert_eq!(0, phone_util.get_length_of_national_destination_code(&number).unwrap());
    
    // Международный бесплатный номер, с NDC "1234".
    number = PhoneNumber::new();
    number.set_country_code(800);
    number.set_national_number(12345678);
    assert_eq!(4, phone_util.get_length_of_national_destination_code(&number).unwrap());

    // Мобильный номер из Китая является географическим, но не имеет кода города,
    // однако у него может быть национальный код назначения.
    let mut cn_mobile = PhoneNumber::new();
    cn_mobile.set_country_code(86);
    cn_mobile.set_national_number(18912341234);
    assert_eq!(3, phone_util.get_length_of_national_destination_code(&cn_mobile).unwrap());
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
    let extracted_number = phone_util.
        extract_possible_number("Num-\u{FF11}\u{FF12}\u{FF13}")
        .unwrap(); // "Num-１２３"
    assert_eq!("\u{FF11}\u{FF12}\u{FF13}", extracted_number);

    // Если возможный номер отсутствует, возвращается пустая строка.
    let extracted_number = phone_util
        .extract_possible_number("Num-....");
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
fn is_possible_number_for_type_different_type_lengths() {
    let phone_util = get_phone_util();
    // Мы используем аргентинские номера, так как у них разная возможная длина для
    // разных типов.
    let mut number = PhoneNumber::new();
    number.set_country_code(54);
    number.set_national_number(12345);

    // Слишком короткий для любого аргентинского номера, включая стационарный.
    assert!(!phone_util.is_possible_number_for_type(&number, PhoneNumberType::FixedLine));
    assert!(!phone_util.is_possible_number_for_type(&number, PhoneNumberType::Unknown));

    // 6-значные номера подходят для стационарных телефонов.
    number.set_national_number(123456);
    assert!(phone_util.is_possible_number_for_type(&number, PhoneNumberType::Unknown));
    assert!(phone_util.is_possible_number_for_type(&number, PhoneNumberType::FixedLine));
    // Но слишком короткие для мобильных.
    assert!(!phone_util.is_possible_number_for_type(&number, PhoneNumberType::Mobile));
    // И слишком короткие для бесплатных номеров.
    assert!(!phone_util.is_possible_number_for_type(&number, PhoneNumberType::TollFree));

    // То же самое относится к 9-значным номерам.
    number.set_national_number(123456789);
    assert!(phone_util.is_possible_number_for_type(&number, PhoneNumberType::Unknown));
    assert!(phone_util.is_possible_number_for_type(&number, PhoneNumberType::FixedLine));
    assert!(!phone_util.is_possible_number_for_type(&number, PhoneNumberType::Mobile));
    assert!(!phone_util.is_possible_number_for_type(&number, PhoneNumberType::TollFree));

    // 10-значные номера возможны для всех типов.
    number.set_national_number(1234567890);
    assert!(phone_util.is_possible_number_for_type(&number, PhoneNumberType::Unknown));
    assert!(phone_util.is_possible_number_for_type(&number, PhoneNumberType::FixedLine));
    assert!(phone_util.is_possible_number_for_type(&number, PhoneNumberType::Mobile));
    assert!(phone_util.is_possible_number_for_type(&number, PhoneNumberType::TollFree));

    // 11-значные номера возможны только для мобильных номеров. Обратите внимание, что мы не
    // требуем ведущую 9, с которой начинаются все мобильные номера и которая
    // была бы необходима для действительного мобильного номера.
    number.set_national_number(12345678901);
    assert!(phone_util.is_possible_number_for_type(&number, PhoneNumberType::Unknown));
    assert!(!phone_util.is_possible_number_for_type(&number, PhoneNumberType::FixedLine));
    assert!(phone_util.is_possible_number_for_type(&number, PhoneNumberType::Mobile));
    assert!(!phone_util.is_possible_number_for_type(&number, PhoneNumberType::TollFree));
}

#[test]
fn is_possible_number_for_type_local_only() {
    let phone_util = get_phone_util();
    let mut number = PhoneNumber::new();
    // Здесь мы тестируем длину номера, которая соответствует длине только для местных номеров.
    number.set_country_code(49);
    number.set_national_number(12);
    assert!(phone_util.is_possible_number_for_type(&number, PhoneNumberType::Unknown));
    assert!(phone_util.is_possible_number_for_type(&number, PhoneNumberType::FixedLine));
    // Мобильные номера должны состоять из 10 или 11 цифр, и для них нет длин,
    // предназначенных только для местных номеров.
    assert!(!phone_util.is_possible_number_for_type(&number, PhoneNumberType::Mobile));
}

#[test]
fn is_possible_number_for_type_data_missing_for_size_reasons() {
    let phone_util = get_phone_util();
    let mut number = PhoneNumber::new();
    // Здесь мы тестируем случай, когда возможные длины соответствуют возможным
    // длинам страны в целом и, следовательно, отсутствуют в бинарных данных
    // по соображениям размера - это все равно должно работать.
    // Номер только для местного использования.
    number.set_country_code(55);
    number.set_national_number(12345678);
    assert!(phone_util.is_possible_number_for_type(&number, PhoneNumberType::Unknown));
    assert!(phone_util.is_possible_number_for_type(&number, PhoneNumberType::FixedLine));

    number.set_national_number(1234567890);
    assert!(phone_util.is_possible_number_for_type(&number, PhoneNumberType::Unknown));
    assert!(phone_util.is_possible_number_for_type(&number, PhoneNumberType::FixedLine));
}

#[test]
fn is_possible_number_for_type_number_type_not_supported_for_region() {
    let phone_util = get_phone_util();
    let mut number = PhoneNumber::new();
    // Для этого региона вообще нет мобильных номеров, поэтому мы возвращаем false.
    number.set_country_code(55);
    number.set_national_number(12345678);
    assert!(!phone_util.is_possible_number_for_type(&number, PhoneNumberType::Mobile));
    // Однако это соответствует длине стационарного номера.
    assert!(phone_util.is_possible_number_for_type(&number, PhoneNumberType::FixedLine));
    assert!(phone_util.is_possible_number_for_type(&number, PhoneNumberType::FixedLineOrMobile));

    // Для этого кода страны вообще нет ни стационарных, ни мобильных номеров,
    // поэтому мы возвращаем false для них.
    number.set_country_code(979);
    number.set_national_number(123456789);
    assert!(!phone_util.is_possible_number_for_type(&number, PhoneNumberType::Mobile));
    assert!(!phone_util.is_possible_number_for_type(&number, PhoneNumberType::FixedLine));
    assert!(!phone_util.is_possible_number_for_type(&number, PhoneNumberType::FixedLineOrMobile));
    assert!(phone_util.is_possible_number_for_type(&number, PhoneNumberType::PremiumRate));
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
    assert_eq!(Ok(NumberLengthType::IsPossible), phone_util.is_possible_number_with_reason(&number));

    number.set_national_number(2530000);
    assert_eq!(Ok(NumberLengthType::IsPossibleLocalOnly), phone_util.is_possible_number_with_reason(&number));
    
    number.set_country_code(0);
    assert_eq!(Err(ValidationError::InvalidCountryCode), phone_util.is_possible_number_with_reason(&number));

    number.set_country_code(1);
    number.set_national_number(253000);
    assert_eq!(Err(ValidationError::TooShort), phone_util.is_possible_number_with_reason(&number));

    number.set_national_number(65025300000);
    assert_eq!(Err(ValidationError::TooLong), phone_util.is_possible_number_with_reason(&number));

    number.set_country_code(44);
    number.set_national_number(2070310000);
    assert_eq!(Ok(NumberLengthType::IsPossible), phone_util.is_possible_number_with_reason(&number));

    number.set_country_code(49);
    number.set_national_number(30123456);
    assert_eq!(Ok(NumberLengthType::IsPossible), phone_util.is_possible_number_with_reason(&number));

    number.set_country_code(65);
    number.set_national_number(1234567890);
    assert_eq!(Ok(NumberLengthType::IsPossible), phone_util.is_possible_number_with_reason(&number));

    number.set_country_code(800);
    number.set_national_number(123456789);
    assert_eq!(Err(ValidationError::TooLong), phone_util.is_possible_number_with_reason(&number));
}

#[test]
fn is_possible_number_for_type_with_reason() {
    let phone_util = get_phone_util();
    let mut ar_number = PhoneNumber::new();
    ar_number.set_country_code(54);

    ar_number.set_national_number(12345);
    assert_eq!(Err(ValidationError::TooShort), phone_util.is_possible_number_for_type_with_reason(&ar_number, PhoneNumberType::Unknown));
    assert_eq!(Err(ValidationError::TooShort), phone_util.is_possible_number_for_type_with_reason(&ar_number, PhoneNumberType::FixedLine));

    ar_number.set_national_number(123456);
    assert_eq!(Ok(NumberLengthType::IsPossible), phone_util.is_possible_number_for_type_with_reason(&ar_number, PhoneNumberType::Unknown));
    assert_eq!(Ok(NumberLengthType::IsPossible), phone_util.is_possible_number_for_type_with_reason(&ar_number, PhoneNumberType::FixedLine));
    assert_eq!(Err(ValidationError::TooShort), phone_util.is_possible_number_for_type_with_reason(&ar_number, PhoneNumberType::Mobile));
    assert_eq!(Err(ValidationError::TooShort), phone_util.is_possible_number_for_type_with_reason(&ar_number, PhoneNumberType::TollFree));

    ar_number.set_national_number(12345678901);
    assert_eq!(Ok(NumberLengthType::IsPossible), phone_util.is_possible_number_for_type_with_reason(&ar_number, PhoneNumberType::Unknown));
    assert_eq!(Err(ValidationError::TooLong), phone_util.is_possible_number_for_type_with_reason(&ar_number, PhoneNumberType::FixedLine));
    assert_eq!(Ok(NumberLengthType::IsPossible), phone_util.is_possible_number_for_type_with_reason(&ar_number, PhoneNumberType::Mobile));
    assert_eq!(Err(ValidationError::TooLong), phone_util.is_possible_number_for_type_with_reason(&ar_number, PhoneNumberType::TollFree));
    
    let mut de_number = PhoneNumber::new();
    de_number.set_country_code(49);
    de_number.set_national_number(12);
    assert_eq!(Ok(NumberLengthType::IsPossibleLocalOnly), phone_util.is_possible_number_for_type_with_reason(&de_number, PhoneNumberType::Unknown));
    assert_eq!(Ok(NumberLengthType::IsPossibleLocalOnly), phone_util.is_possible_number_for_type_with_reason(&de_number, PhoneNumberType::FixedLine));
    assert_eq!(Err(ValidationError::TooShort), phone_util.is_possible_number_for_type_with_reason(&de_number, PhoneNumberType::Mobile));

    let mut br_number = PhoneNumber::new();
    br_number.set_country_code(55);
    br_number.set_national_number(12345678);
    assert_eq!(Err(ValidationError::InvalidLength), phone_util.is_possible_number_for_type_with_reason(&br_number, PhoneNumberType::Mobile));
    assert_eq!(Ok(NumberLengthType::IsPossibleLocalOnly), phone_util.is_possible_number_for_type_with_reason(&br_number, PhoneNumberType::FixedLineOrMobile));
}

#[test]
fn is_possible_number_for_type_with_reason_different_type_lengths() {
    // Мы используем аргентинские номера, так как у них разная возможная длина для разных типов.
    let phone_util = get_phone_util();
    let mut number = PhoneNumber::new();
    number.set_country_code(54);
    number.set_national_number(12345);

    assert_eq!(Err(ValidationError::TooShort), phone_util.is_possible_number_for_type_with_reason(&number, PhoneNumberType::Unknown));
    assert_eq!(Err(ValidationError::TooShort), phone_util.is_possible_number_for_type_with_reason(&number, PhoneNumberType::FixedLine));

    // 6-значные номера подходят для стационарных телефонов.
    number.set_national_number(123456);
    assert_eq!(Ok(NumberLengthType::IsPossible), phone_util.is_possible_number_for_type_with_reason(&number, PhoneNumberType::Unknown));
    assert_eq!(Ok(NumberLengthType::IsPossible), phone_util.is_possible_number_for_type_with_reason(&number, PhoneNumberType::FixedLine));
    // Но слишком коротки для мобильных.
    assert_eq!(Err(ValidationError::TooShort), phone_util.is_possible_number_for_type_with_reason(&number, PhoneNumberType::Mobile));
    // И слишком коротки для бесплатных номеров.
    assert_eq!(Err(ValidationError::TooShort), phone_util.is_possible_number_for_type_with_reason(&number, PhoneNumberType::TollFree));

    // То же самое касается 9-значных номеров.
    number.set_national_number(123456789);
    assert_eq!(Ok(NumberLengthType::IsPossible), phone_util.is_possible_number_for_type_with_reason(&number, PhoneNumberType::Unknown));
    assert_eq!(Ok(NumberLengthType::IsPossible), phone_util.is_possible_number_for_type_with_reason(&number, PhoneNumberType::FixedLine));
    assert_eq!(Err(ValidationError::TooShort), phone_util.is_possible_number_for_type_with_reason(&number, PhoneNumberType::Mobile));
    assert_eq!(Err(ValidationError::TooShort), phone_util.is_possible_number_for_type_with_reason(&number, PhoneNumberType::TollFree));

    // 10-значные номера возможны для всех типов.
    number.set_national_number(1234567890);
    assert_eq!(Ok(NumberLengthType::IsPossible), phone_util.is_possible_number_for_type_with_reason(&number, PhoneNumberType::Unknown));
    assert_eq!(Ok(NumberLengthType::IsPossible), phone_util.is_possible_number_for_type_with_reason(&number, PhoneNumberType::FixedLine));
    assert_eq!(Ok(NumberLengthType::IsPossible), phone_util.is_possible_number_for_type_with_reason(&number, PhoneNumberType::Mobile));
    assert_eq!(Ok(NumberLengthType::IsPossible), phone_util.is_possible_number_for_type_with_reason(&number, PhoneNumberType::TollFree));

    // 11-значные номера возможны для мобильных номеров. Обратите внимание, что мы не требуем ведущую 9,
    // с которой начинаются все мобильные номера и которая была бы необходима для действительного мобильного номера.
    number.set_national_number(12345678901);
    assert_eq!(Ok(NumberLengthType::IsPossible), phone_util.is_possible_number_for_type_with_reason(&number, PhoneNumberType::Unknown));
    assert_eq!(Err(ValidationError::TooLong), phone_util.is_possible_number_for_type_with_reason(&number, PhoneNumberType::FixedLine));
    assert_eq!(Ok(NumberLengthType::IsPossible), phone_util.is_possible_number_for_type_with_reason(&number, PhoneNumberType::Mobile));
    assert_eq!(Err(ValidationError::TooLong), phone_util.is_possible_number_for_type_with_reason(&number, PhoneNumberType::TollFree));
}

#[test]
fn is_possible_number_for_type_with_reason_local_only() {
    let phone_util = get_phone_util();
    let mut number = PhoneNumber::new();
    // Здесь мы тестируем длину номера, которая соответствует только местной длине.
    number.set_country_code(49);
    number.set_national_number(12);
    assert_eq!(Ok(NumberLengthType::IsPossibleLocalOnly), phone_util.is_possible_number_for_type_with_reason(&number, PhoneNumberType::Unknown));
    assert_eq!(Ok(NumberLengthType::IsPossibleLocalOnly), phone_util.is_possible_number_for_type_with_reason(&number, PhoneNumberType::FixedLine));
    // Мобильные номера должны состоять из 10 или 11 цифр, и для них нет только местных длин.
    assert_eq!(Err(ValidationError::TooShort), phone_util.is_possible_number_for_type_with_reason(&number, PhoneNumberType::Mobile));
}

#[test]
fn is_possible_number_for_type_with_reason_data_missing_for_size_reasons() {
    let phone_util = get_phone_util();
    let mut number = PhoneNumber::new();
    // Здесь мы тестируем случай, когда возможные длины соответствуют возможным длинам страны в целом
    // и поэтому отсутствуют в бинарных данных по соображениям размера - это все равно должно работать.
    // Номер только для местного использования.
    number.set_country_code(55);
    number.set_national_number(12345678);
    assert_eq!(Ok(NumberLengthType::IsPossibleLocalOnly), phone_util.is_possible_number_for_type_with_reason(&number, PhoneNumberType::Unknown));
    assert_eq!(Ok(NumberLengthType::IsPossibleLocalOnly), phone_util.is_possible_number_for_type_with_reason(&number, PhoneNumberType::FixedLine));
    // Номер нормальной длины.
    number.set_national_number(1234567890);
    assert_eq!(Ok(NumberLengthType::IsPossible), phone_util.is_possible_number_for_type_with_reason(&number, PhoneNumberType::Unknown));
    assert_eq!(Ok(NumberLengthType::IsPossible), phone_util.is_possible_number_for_type_with_reason(&number, PhoneNumberType::FixedLine));
}

#[test]
fn is_possible_number_for_type_with_reason_number_type_not_supported_for_region() {
    let phone_util = get_phone_util();
    let mut number = PhoneNumber::new();
    // В этом регионе вообще *нет* мобильных номеров, поэтому мы возвращаем INVALID_LENGTH.
    number.set_country_code(55);
    number.set_national_number(12345678);
    assert_eq!(Err(ValidationError::InvalidLength), phone_util.is_possible_number_for_type_with_reason(&number, PhoneNumberType::Mobile));
    // Однако это соответствует длине стационарного номера.
    assert_eq!(Ok(NumberLengthType::IsPossibleLocalOnly), phone_util.is_possible_number_for_type_with_reason(&number, PhoneNumberType::FixedLineOrMobile));
    // Этот номер слишком короткий для стационарного, а мобильных номеров не существует.
    number.set_national_number(1234567);
    assert_eq!(Err(ValidationError::InvalidLength), phone_util.is_possible_number_for_type_with_reason(&number, PhoneNumberType::Mobile));
    assert_eq!(Err(ValidationError::TooShort), phone_util.is_possible_number_for_type_with_reason(&number, PhoneNumberType::FixedLineOrMobile));
    assert_eq!(Err(ValidationError::TooShort), phone_util.is_possible_number_for_type_with_reason(&number, PhoneNumberType::FixedLine));
    // Этот номер слишком короткий для мобильного, а стационарных номеров не существует.
    number.set_country_code(882);
    number.set_national_number(1234567);
    assert_eq!(Err(ValidationError::TooShort), phone_util.is_possible_number_for_type_with_reason(&number, PhoneNumberType::Mobile));
    assert_eq!(Err(ValidationError::TooShort), phone_util.is_possible_number_for_type_with_reason(&number, PhoneNumberType::FixedLineOrMobile));
    assert_eq!(Err(ValidationError::InvalidLength), phone_util.is_possible_number_for_type_with_reason(&number, PhoneNumberType::FixedLine));

    // Для этого кода страны вообще *нет* ни стационарных, ни мобильных номеров,
    // поэтому мы возвращаем INVALID_LENGTH.
    number.set_country_code(979);
    number.set_national_number(123456789);
    assert_eq!(Err(ValidationError::InvalidLength), phone_util.is_possible_number_for_type_with_reason(&number, PhoneNumberType::Mobile));
    assert_eq!(Err(ValidationError::InvalidLength), phone_util.is_possible_number_for_type_with_reason(&number, PhoneNumberType::FixedLine));
    assert_eq!(Err(ValidationError::InvalidLength), phone_util.is_possible_number_for_type_with_reason(&number, PhoneNumberType::FixedLineOrMobile));
    assert_eq!(Ok(NumberLengthType::IsPossible), phone_util.is_possible_number_for_type_with_reason(&number, PhoneNumberType::PremiumRate));
}

#[test]
fn is_possible_number_for_type_with_reason_fixed_line_or_mobile() {
    let phone_util = get_phone_util();
    let mut number = PhoneNumber::new();
    // Для FIXED_LINE_OR_MOBILE номер должен считаться действительным, если он соответствует
    // возможным длинам для мобильных *или* стационарных номеров.
    number.set_country_code(290);
    number.set_national_number(1234);
    assert_eq!(Err(ValidationError::TooShort), phone_util.is_possible_number_for_type_with_reason(&number, PhoneNumberType::FixedLine));
    assert_eq!(Ok(NumberLengthType::IsPossible), phone_util.is_possible_number_for_type_with_reason(&number, PhoneNumberType::Mobile));
    assert_eq!(Ok(NumberLengthType::IsPossible), phone_util.is_possible_number_for_type_with_reason(&number, PhoneNumberType::FixedLineOrMobile));

    number.set_national_number(12345);
    assert_eq!(Err(ValidationError::TooShort), phone_util.is_possible_number_for_type_with_reason(&number, PhoneNumberType::FixedLine));
    assert_eq!(Err(ValidationError::TooLong), phone_util.is_possible_number_for_type_with_reason(&number, PhoneNumberType::Mobile));
    assert_eq!(Err(ValidationError::InvalidLength), phone_util.is_possible_number_for_type_with_reason(&number, PhoneNumberType::FixedLineOrMobile));

    number.set_national_number(123456);
    assert_eq!(Ok(NumberLengthType::IsPossible), phone_util.is_possible_number_for_type_with_reason(&number, PhoneNumberType::FixedLine));
    assert_eq!(Err(ValidationError::TooLong), phone_util.is_possible_number_for_type_with_reason(&number, PhoneNumberType::Mobile));
    assert_eq!(Ok(NumberLengthType::IsPossible), phone_util.is_possible_number_for_type_with_reason(&number, PhoneNumberType::FixedLineOrMobile));

    number.set_national_number(1234567);
    assert_eq!(Err(ValidationError::TooLong), phone_util.is_possible_number_for_type_with_reason(&number, PhoneNumberType::FixedLine));
    assert_eq!(Err(ValidationError::TooLong), phone_util.is_possible_number_for_type_with_reason(&number, PhoneNumberType::Mobile));
    assert_eq!(Err(ValidationError::TooLong), phone_util.is_possible_number_for_type_with_reason(&number, PhoneNumberType::FixedLineOrMobile));

    number.set_national_number(12345678);
    assert_eq!(Ok(NumberLengthType::IsPossible), phone_util.is_possible_number_for_type_with_reason(&number, PhoneNumberType::TollFree));
    assert_eq!(Err(ValidationError::TooLong), phone_util.is_possible_number_for_type_with_reason(&number, PhoneNumberType::FixedLineOrMobile));
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
    
    // С национальным префиксом.
    let test_number = phone_util.parse("033316005", RegionCode::nz()).unwrap();
    assert_eq!(nz_number, test_number);

    // Без национального префикса.
    let test_number = phone_util.parse("33316005", RegionCode::nz()).unwrap();
    assert_eq!(nz_number, test_number);

    // С национальным префиксом и форматированием.
    let test_number = phone_util.parse("03-331 6005", RegionCode::nz()).unwrap();
    assert_eq!(nz_number, test_number);
    let test_number = phone_util.parse("03 331 6005", RegionCode::nz()).unwrap();
    assert_eq!(nz_number, test_number);

    // Тестирование парсинга формата RFC3966 с phone-context.
    let test_number = phone_util.parse("tel:03-331-6005;phone-context=+64", RegionCode::nz()).unwrap();
    assert_eq!(nz_number, test_number);
    let test_number = phone_util.parse("tel:331-6005;phone-context=+64-3", RegionCode::nz()).unwrap();
    assert_eq!(nz_number, test_number);
    let test_number = phone_util.parse("tel:331-6005;phone-context=+64-3", RegionCode::us()).unwrap();
    assert_eq!(nz_number, test_number);
    let test_number = phone_util.parse("My number is tel:03-331-6005;phone-context=+64", RegionCode::nz()).unwrap();
    assert_eq!(nz_number, test_number);

    // Тестирование парсинга RFC3966 с опциональными параметрами.
    let test_number = phone_util.parse("tel:03-331-6005;phone-context=+64;a=%A1", RegionCode::nz()).unwrap();
    assert_eq!(nz_number, test_number);

    // Тестирование парсинга RFC3966 с ISDN-субадресом.
    let test_number = phone_util.parse("tel:03-331-6005;isub=12345;phone-context=+64", RegionCode::nz()).unwrap();
    assert_eq!(nz_number, test_number);
    let test_number = phone_util.parse("tel:+64-3-331-6005;isub=12345", RegionCode::us()).unwrap();
    assert_eq!(nz_number, test_number);
    let test_number = phone_util.parse("03-331-6005;phone-context=+64", RegionCode::nz()).unwrap();
    assert_eq!(nz_number, test_number);

    // Тестирование международных префиксов.
    // Код страны должен быть удалён.
    let test_number = phone_util.parse("0064 3 d331 6005", RegionCode::nz()).unwrap();
    assert_eq!(nz_number, test_number);

    // Попробуем снова, но на этот раз с международным номером для региона US.
    // Код страны должен быть распознан и обработан корректно.
    let test_number = phone_util.parse("01164 3 331 6005", RegionCode::us()).unwrap();
    assert_eq!(nz_number, test_number);
    let test_number = phone_util.parse("+64 3 331 6005", RegionCode::us()).unwrap();
    assert_eq!(nz_number, test_number);

    // Ведущий плюс должен игнорироваться, т.к. за ним следует не код страны, а IDD для США.
    let test_number = phone_util.parse("+01164 3 331 6005", RegionCode::us()).unwrap();
    assert_eq!(nz_number, test_number);
    let test_number = phone_util.parse("+0064 3 331 6005", RegionCode::nz()).unwrap();
    assert_eq!(nz_number, test_number);
    let test_number = phone_util.parse("+ 00 64 3 331 6005", RegionCode::nz()).unwrap();
    assert_eq!(nz_number, test_number);

    let mut us_local_number = PhoneNumber::new();
    us_local_number.set_country_code(1);
    us_local_number.set_national_number(2530000);
    let test_number = phone_util.parse("tel:253-0000;phone-context=www.google.com", RegionCode::us()).unwrap();
    assert_eq!(us_local_number, test_number);
    let test_number = phone_util.parse("tel:253-0000;isub=12345;phone-context=www.google.com", RegionCode::us()).unwrap();
    assert_eq!(us_local_number, test_number);
    let test_number = phone_util.parse("tel:2530000;isub=12345;phone-context=1234.com", RegionCode::us()).unwrap();
    assert_eq!(us_local_number, test_number);

    // Тест для http://b/issue?id=2247493
    let mut nz_number_issue = PhoneNumber::new();
    nz_number_issue.set_country_code(64);
    nz_number_issue.set_national_number(64123456);
    let test_number = phone_util.parse("+64(0)64123456", RegionCode::us()).unwrap();
    assert_eq!(nz_number_issue, test_number);

    // Проверка, что "/" в номере телефона обрабатывается корректно.
    let mut de_number = PhoneNumber::new();
    de_number.set_country_code(49);
    de_number.set_national_number(12345678);
    let test_number = phone_util.parse("123/45678", RegionCode::de()).unwrap();
    assert_eq!(de_number, test_number);

    let mut us_number = PhoneNumber::new();
    us_number.set_country_code(1);
    // Проверка, что '1' не используется как код страны при парсинге, если номер уже валиден.
    us_number.set_national_number(1234567890);
    let test_number = phone_util.parse("123-456-7890", RegionCode::us()).unwrap();
    assert_eq!(us_number, test_number);

    // Тестирование номеров со звездочкой.
    let mut star_number = PhoneNumber::new();
    star_number.set_country_code(81);
    star_number.set_national_number(2345);
    let test_number = phone_util.parse("+81 *2345", RegionCode::jp()).unwrap();
    assert_eq!(star_number, test_number);

    let mut short_number = PhoneNumber::new();
    short_number.set_country_code(64);
    short_number.set_national_number(12);
    let test_number = phone_util.parse("12", RegionCode::nz()).unwrap();
    assert_eq!(short_number, test_number);

    // Тест для короткого номера с ведущим нулём для страны, где 0 - национальный префикс.
    // Убедиться, что он не интерпретируется как национальный префикс, если
    // оставшаяся длина номера соответствует только местному номеру.
    let mut short_number = PhoneNumber::new();
    short_number.set_country_code(44);
    short_number.set_national_number(123456);
    short_number.set_italian_leading_zero(true);
    let test_number = phone_util.parse("0123456", RegionCode::gb()).unwrap();
    assert_eq!(short_number, test_number);
}

#[test]
fn parse_with_phone_context() {
    fn assert_throws_for_invalid_phone_context(phone_util: &PhoneNumberUtil, number_to_parse: &str) {
        let result = phone_util.parse(number_to_parse, RegionCode::zz());
        assert!(result.is_err(), "Expected an error for: {}", number_to_parse);
    }
    let phone_util = get_phone_util();
    let mut expected_number = PhoneNumber::new();
    expected_number.set_country_code(64);
    expected_number.set_national_number(33316005);
    
    // context    = ";phone-context=" descriptor
    // descriptor = domainname / global-number-digits
    
    // Валидные global-phone-digits
    let mut actual_number = phone_util.parse("tel:033316005;phone-context=+64", RegionCode::zz()).unwrap();
    assert_eq!(expected_number, actual_number);

    actual_number = phone_util.parse("tel:033316005;phone-context=+64;{this isn't part of phone-context anymore!}", RegionCode::zz()).unwrap();
    assert_eq!(expected_number, actual_number);

    expected_number.set_national_number(3033316005);
    actual_number = phone_util.parse("tel:033316005;phone-context=+64-3", RegionCode::zz()).unwrap();
    assert_eq!(expected_number, actual_number);
    
    expected_number.set_country_code(55);
    expected_number.set_national_number(5033316005);
    actual_number = phone_util.parse("tel:033316005;phone-context=+(555)", RegionCode::zz()).unwrap();
    assert_eq!(expected_number, actual_number);

    expected_number.set_country_code(1);
    expected_number.set_national_number(23033316005);
    actual_number = phone_util.parse("tel:033316005;phone-context=+-1-2.3()", RegionCode::zz()).unwrap();
    assert_eq!(expected_number, actual_number);

    // Валидный domainname
    expected_number.set_country_code(64);
    expected_number.set_national_number(33316005);
    actual_number = phone_util.parse("tel:033316005;phone-context=abc.nz", RegionCode::nz()).unwrap();
    assert_eq!(expected_number, actual_number);

    actual_number = phone_util.parse("tel:033316005;phone-context=www.PHONE-numb3r.com", RegionCode::nz()).unwrap();
    assert_eq!(expected_number, actual_number);

    actual_number = phone_util.parse("tel:033316005;phone-context=a", RegionCode::nz()).unwrap();
    assert_eq!(expected_number, actual_number);

    actual_number = phone_util.parse("tel:033316005;phone-context=3phone.J.", RegionCode::nz()).unwrap();
    assert_eq!(expected_number, actual_number);
    
    actual_number = phone_util.parse("tel:033316005;phone-context=a--z", RegionCode::nz()).unwrap();
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
        phone_util.parse("This is not a phone number", RegionCode::nz()).unwrap_err(),
        ParseError::NotANumber(_)
    ));
    assert!(matches!(
        phone_util.parse("1 Still not a number", RegionCode::nz()).unwrap_err(),
        ParseError::NotANumber(_)
    ));
    assert!(matches!(
        phone_util.parse("1 MICROSOFT", RegionCode::nz()).unwrap_err(),
        ParseError::NotANumber(_)
    ));
    assert!(matches!(
        phone_util.parse("12 MICROSOFT", RegionCode::nz()).unwrap_err(),
        ParseError::NotANumber(_)
    ));
    assert_eq!(
        phone_util.parse("01495 72553301873 810104", RegionCode::gb()).unwrap_err(),
        ParseError::TooLongNsn
    );
    assert!(matches!(
        phone_util.parse("+---", RegionCode::de()).unwrap_err(),
        ParseError::NotANumber(_)
    ));
    assert!(matches!(
        phone_util.parse("+***", RegionCode::de()).unwrap_err(),
        ParseError::NotANumber(_)
    ));
    assert!(matches!(
        phone_util.parse("+*******91", RegionCode::de()).unwrap_err(),
        ParseError::NotANumber(_)
    ));
    assert_eq!(
        phone_util.parse("+49 0", RegionCode::de()).unwrap_err(),
        ParseError::TooShortNsn
    );
    assert_eq!(
        phone_util.parse("+210 3456 56789", RegionCode::nz()).unwrap_err(),
        ParseError::InvalidCountryCode
    );
    // 00 - правильный МНН, но 210 - невалидный код страны.
    assert_eq!(
        phone_util.parse("+ 00 210 3 331 6005", RegionCode::nz()).unwrap_err(),
        ParseError::InvalidCountryCode
    );
    assert_eq!(
        phone_util.parse("123 456 7890", RegionCode::zz()).unwrap_err(),
        ParseError::InvalidCountryCode
    );
    assert_eq!(
        phone_util.parse("123 456 7890", RegionCode::cs()).unwrap_err(),
        ParseError::InvalidCountryCode
    );
    assert_eq!(
        phone_util.parse("0044-----", RegionCode::gb()).unwrap_err(),
        ParseError::TooShortAfterIdd
    );
    assert_eq!(
        phone_util.parse("0044", RegionCode::gb()).unwrap_err(),
        ParseError::TooShortAfterIdd
    );
    assert_eq!(
        phone_util.parse("011", RegionCode::us()).unwrap_err(),
        ParseError::TooShortAfterIdd
    );
    assert_eq!(
        phone_util.parse("0119", RegionCode::us()).unwrap_err(),
        ParseError::TooShortAfterIdd
    );
    // RFC3966 phone-context является веб-сайтом.
    assert_eq!(
        phone_util.parse("tel:555-1234;phone-context=www.google.com", RegionCode::zz()).unwrap_err(),
        ParseError::InvalidCountryCode
    );
    // Это невалидно, так как отсутствует знак "+" в phone-context.
    assert!(matches!(
        phone_util.parse("tel:555-1234;phone-context=1-331", RegionCode::zz()).unwrap_err(),
        ParseError::NotANumber(_)
    ));
    // Присутствует только символ phone-context, но нет данных.
    assert!(matches!(
        phone_util.parse(";phone-context=", RegionCode::zz()).unwrap_err(),
        ParseError::NotANumber(_)
    ));
}

#[test]
fn parse_numbers_with_plus_with_no_region() {
    let phone_util = get_phone_util();
    let mut nz_number = PhoneNumber::new();
    nz_number.set_country_code(64);
    nz_number.set_national_number(33316005);
    // RegionCode::zz() (неизвестный регион) разрешен только если номер начинается с "+",
    // тогда код страны можно определить.
    let mut result_proto = phone_util.parse("+64 3 331 6005", RegionCode::zz()).unwrap();
    assert_eq!(nz_number, result_proto);

    // Тестируем с полноширинным плюсом.
    result_proto = phone_util.parse("\u{FF0B}64 3 331 6005", RegionCode::zz()).unwrap();
    assert_eq!(nz_number, result_proto);
    // Тестируем с обычным плюсом, но с начальными символами, которые нужно удалить.
    result_proto = phone_util.parse("  +64 3 331 6005", RegionCode::zz()).unwrap();
    assert_eq!(nz_number, result_proto);

    let mut toll_free_number = PhoneNumber::new();
    toll_free_number.set_country_code(800);
    toll_free_number.set_national_number(12345678);
    result_proto = phone_util.parse("+800 1234 5678", RegionCode::zz()).unwrap();
    assert_eq!(toll_free_number, result_proto);

    let mut universal_premium_rate = PhoneNumber::new();
    universal_premium_rate.set_country_code(979);
    universal_premium_rate.set_national_number(123456789);
    result_proto = phone_util.parse("+979 123 456 789", RegionCode::zz()).unwrap();
    assert_eq!(universal_premium_rate, result_proto);

    // Тестируем парсинг формата RFC3966 с phone context.
    result_proto = phone_util.parse("tel:03-331-6005;phone-context=+64", RegionCode::zz()).unwrap();
    assert_eq!(nz_number, result_proto);

    result_proto = phone_util.parse("  tel:03-331-6005;phone-context=+64", RegionCode::zz()).unwrap();
    assert_eq!(nz_number, result_proto);
    
    result_proto = phone_util.parse("tel:03-331-6005;isub=12345;phone-context=+64", RegionCode::zz()).unwrap();
    assert_eq!(nz_number, result_proto);

    nz_number.set_raw_input("+64 3 331 6005".to_string());
    nz_number.set_country_code_source(CountryCodeSource::FROM_NUMBER_WITH_PLUS_SIGN);
    result_proto = phone_util.parse_and_keep_raw_input("+64 3 331 6005", RegionCode::zz()).unwrap();
    assert_eq!(nz_number, result_proto);
}

#[test]
fn parse_number_too_short_if_national_prefix_stripped() {
    let phone_util = get_phone_util();

    // Тестируем, что у номера, первые цифры которого совпадают с национальным префиксом,
    // они не удаляются, если это приведет к тому, что номер станет слишком коротким,
    // чтобы быть возможным (стандартной длины) телефонным номером для этого региона.
    let mut by_number = PhoneNumber::new();
    by_number.set_country_code(375);
    by_number.set_national_number(8123);
    let mut test_number = phone_util.parse("8123", RegionCode::by()).unwrap();
    assert_eq!(by_number, test_number);

    by_number.set_national_number(81234);
    test_number = phone_util.parse("81234", RegionCode::by()).unwrap();
    assert_eq!(by_number, test_number);

    // Префикс не удаляется, так как ввод является валидным 6-значным номером,
    // в то время как результат удаления - всего 5 цифр.
    by_number.set_national_number(812345);
    test_number = phone_util.parse("812345", RegionCode::by()).unwrap();
    assert_eq!(by_number, test_number);

    // Префикс удаляется, так как возможны только 6-значные номера.
    by_number.set_national_number(123456);
    test_number = phone_util.parse("8123456", RegionCode::by()).unwrap();
    assert_eq!(by_number, test_number);
}

#[test]
fn parse_extensions() {
    let phone_util = get_phone_util();

    let mut nz_number = PhoneNumber::new();
    nz_number.set_country_code(64);
    nz_number.set_national_number(33316005);
    nz_number.set_extension("3456".to_string());

    let mut test_number = phone_util.parse("03 331 6005 ext 3456", RegionCode::nz()).unwrap();
    assert_eq!(nz_number, test_number);

    test_number = phone_util.parse("03 331 6005x3456", RegionCode::nz()).unwrap();
    assert_eq!(nz_number, test_number);

    test_number = phone_util.parse("03-331 6005 int.3456", RegionCode::nz()).unwrap();
    assert_eq!(nz_number, test_number);

    test_number = phone_util.parse("03 331 6005 #3456", RegionCode::nz()).unwrap();
    assert_eq!(nz_number, test_number);

    // Тестируем, что следующие номера не извлекают добавочные номера:
    let mut non_extn_number = PhoneNumber::new();
    non_extn_number.set_country_code(1);
    non_extn_number.set_national_number(80074935247);

    test_number = phone_util.parse("1800 six-flags", RegionCode::us()).unwrap();
    assert_eq!(non_extn_number, test_number);

    test_number = phone_util.parse("1800 SIX-FLAGS", RegionCode::us()).unwrap();
    assert_eq!(non_extn_number, test_number);

    test_number = phone_util.parse("0~0 1800 7493 5247", RegionCode::pl()).unwrap();
    assert_eq!(non_extn_number, test_number);

    test_number = phone_util.parse("(1800) 7493.5247", RegionCode::us()).unwrap();
    assert_eq!(non_extn_number, test_number);

    // Проверяем, что соответствует последний экземпляр токена расширения.
    let mut extn_number = PhoneNumber::new();
    extn_number.set_country_code(1);
    extn_number.set_national_number(80074935247);
    extn_number.set_extension("1234".to_string());
    test_number = phone_util.parse("0~0 1800 7493 5247 ~1234", RegionCode::pl()).unwrap();
    assert_eq!(extn_number, test_number);

    // Проверяем исправление ошибки, когда последняя цифра номера ранее опускалась,
    // если это был 0 при извлечении расширения. Также проверяем несколько различных
    // случаев расширений.
    let mut uk_number = PhoneNumber::new();
    uk_number.set_country_code(44);
    uk_number.set_national_number(2034567890);
    uk_number.set_extension("456".to_string());

    test_number = phone_util.parse("+44 2034567890x456", RegionCode::nz()).unwrap();
    assert_eq!(uk_number, test_number);
    test_number = phone_util.parse("+44 2034567890x456", RegionCode::gb()).unwrap();
    assert_eq!(uk_number, test_number);
    test_number = phone_util.parse("+44 2034567890 x456", RegionCode::gb()).unwrap();
    assert_eq!(uk_number, test_number);
    test_number = phone_util.parse("+44 2034567890 X456", RegionCode::gb()).unwrap();
    assert_eq!(uk_number, test_number);
    test_number = phone_util.parse("+44 2034567890 X 456", RegionCode::gb()).unwrap();
    assert_eq!(uk_number, test_number);
    test_number = phone_util.parse("+44 2034567890 X   456", RegionCode::gb()).unwrap();
    assert_eq!(uk_number, test_number);
    test_number = phone_util.parse("+44 2034567890 x 456  ", RegionCode::gb()).unwrap();
    assert_eq!(uk_number, test_number);
    test_number = phone_util.parse("+44 2034567890  X 456", RegionCode::gb()).unwrap();
    assert_eq!(uk_number, test_number);
    test_number = phone_util.parse("+44-2034567890;ext=456", RegionCode::gb()).unwrap();
    assert_eq!(uk_number, test_number);
    test_number = phone_util.parse("tel:2034567890;ext=456;phone-context=+44", RegionCode::zz()).unwrap();
    assert_eq!(uk_number, test_number);

    // Полноширинное расширение, только "extn".
    test_number = phone_util.parse("+442034567890ｅｘｔｎ456", RegionCode::gb()).unwrap();
    assert_eq!(uk_number, test_number);
    // Только "xtn".
    test_number = phone_util.parse("+44-2034567890ｘｔｎ456", RegionCode::gb()).unwrap();
    assert_eq!(uk_number, test_number);
    // Только "xt".
    test_number = phone_util.parse("+44-2034567890ｘｔ456", RegionCode::gb()).unwrap();
    assert_eq!(uk_number, test_number);

    let mut us_with_extension = PhoneNumber::new();
    us_with_extension.set_country_code(1);
    us_with_extension.set_national_number(8009013355);
    us_with_extension.set_extension("7246433".to_string());

    test_number = phone_util.parse("(800) 901-3355 x 7246433", RegionCode::us()).unwrap();
    assert_eq!(us_with_extension, test_number);
    test_number = phone_util.parse("(800) 901-3355 , ext 7246433", RegionCode::us()).unwrap();
    assert_eq!(us_with_extension, test_number);
    test_number = phone_util.parse("(800) 901-3355 ; 7246433", RegionCode::us()).unwrap();
    assert_eq!(us_with_extension, test_number);
    // Тестирование символа расширения без окружающих пробелов.
    test_number = phone_util.parse("(800) 901-3355;7246433", RegionCode::us()).unwrap();
    assert_eq!(us_with_extension, test_number);
    test_number = phone_util.parse("(800) 901-3355 ,extension 7246433", RegionCode::us()).unwrap();
    assert_eq!(us_with_extension, test_number);
    test_number = phone_util.parse("(800) 901-3355 ,extensión 7246433", RegionCode::us()).unwrap();
    assert_eq!(us_with_extension, test_number);
    // Повтор с маленькой буквой o с акутом, созданной с помощью комбинированных символов.
    test_number = phone_util.parse("(800) 901-3355 ,extensión 7246433", RegionCode::us()).unwrap();
    assert_eq!(us_with_extension, test_number);
    test_number = phone_util.parse("(800) 901-3355 , 7246433", RegionCode::us()).unwrap();
    assert_eq!(us_with_extension, test_number);
    test_number = phone_util.parse("(800) 901-3355 ext: 7246433", RegionCode::us()).unwrap();
    assert_eq!(us_with_extension, test_number);
    // Тестирование русского расширения "доб" с вариантами, найденными в интернете.
    let mut ru_with_extension = PhoneNumber::new();
    ru_with_extension.set_country_code(7);
    ru_with_extension.set_national_number(4232022511);
    ru_with_extension.set_extension("100".to_string());
    test_number = phone_util.parse("8 (423) 202-25-11, доб. 100", RegionCode::ru()).unwrap();
    assert_eq!(ru_with_extension, test_number);
    test_number = phone_util.parse("8 (423) 202-25-11 доб. 100", RegionCode::ru()).unwrap();
    assert_eq!(ru_with_extension, test_number);
    test_number = phone_util.parse("8 (423) 202-25-11, доб 100", RegionCode::ru()).unwrap();
    assert_eq!(ru_with_extension, test_number);
    test_number = phone_util.parse("8 (423) 202-25-11 доб 100", RegionCode::ru()).unwrap();
    assert_eq!(ru_with_extension, test_number);
    test_number = phone_util.parse("8 (423) 202-25-11доб 100", RegionCode::ru()).unwrap();
    assert_eq!(ru_with_extension, test_number);
    // В верхнем регистре
    test_number = phone_util.parse("8 (423) 202-25-11 ДОБ 100", RegionCode::ru()).unwrap();
    assert_eq!(ru_with_extension, test_number);
    
    // Тестируем, что если у номера два расширения, мы игнорируем второе.
    let mut us_with_two_extensions_number = PhoneNumber::new();
    us_with_two_extensions_number.set_country_code(1);
    us_with_two_extensions_number.set_national_number(2121231234);
    us_with_two_extensions_number.set_extension("508".to_string());

    test_number = phone_util.parse("(212)123-1234 x508/x1234", RegionCode::us()).unwrap();
    assert_eq!(us_with_two_extensions_number, test_number);
    test_number = phone_util.parse("(212)123-1234 x508/ x1234", RegionCode::us()).unwrap();
    assert_eq!(us_with_two_extensions_number, test_number);
    test_number = phone_util.parse("(212)123-1234 x508\\x1234", RegionCode::us()).unwrap();
    assert_eq!(us_with_two_extensions_number, test_number);

    // Тестируем парсинг номеров вида (645) 123-1234-910#, где последние 3 цифры
    // перед # - это расширение.
    us_with_extension.clear();
    us_with_extension.set_country_code(1);
    us_with_extension.set_national_number(6451231234);
    us_with_extension.set_extension("910".to_string());
    test_number = phone_util.parse("+1 (645) 123 1234-910#", RegionCode::us()).unwrap();
    assert_eq!(us_with_extension, test_number);
}

#[test]
fn test_parse_handles_long_extensions_with_explicit_labels() {
    let phone_util = get_phone_util();
    // Тестируем верхние и нижние пределы длины добавочного номера для каждого типа метки.
    let mut nz_number = PhoneNumber::new();
    nz_number.set_country_code(64);
    nz_number.set_national_number(33316005);
    
    // Сначала в формате RFC: ext_limit_after_explicit_label
    nz_number.set_extension("0".to_string());
    let test_number = phone_util.parse("tel:+6433316005;ext=0", RegionCode::nz()).unwrap();
    assert_eq!(nz_number, test_number);

    nz_number.set_extension("01234567890123456789".to_string());
    let test_number = phone_util.parse("tel:+6433316005;ext=01234567890123456789", RegionCode::nz()).unwrap();
    assert_eq!(nz_number, test_number);

    // Слишком длинное расширение.
    let result = phone_util.parse("tel:+6433316005;ext=012345678901234567890", RegionCode::nz());
    assert!(result.is_err());

    // Явная метка расширения: ext_limit_after_explicit_label
    nz_number.set_extension("1".to_string());
    let test_number = phone_util.parse("03 3316005ext:1", RegionCode::nz()).unwrap();
    assert_eq!(nz_number, test_number);

    nz_number.set_extension("12345678901234567890".to_string());
    let test_number = phone_util.parse("03 3316005 xtn:12345678901234567890", RegionCode::nz()).unwrap();
    assert_eq!(nz_number, test_number);

    let test_number = phone_util.parse("03 3316005 extension\t12345678901234567890", RegionCode::nz()).unwrap();
    assert_eq!(nz_number, test_number);

    let test_number = phone_util.parse("03 3316005 xtensio:12345678901234567890", RegionCode::nz()).unwrap();
    assert_eq!(nz_number, test_number);

    let test_number = phone_util.parse("03 3316005 xtensión, 12345678901234567890#", RegionCode::nz()).unwrap();
    assert_eq!(nz_number, test_number);

    let test_number = phone_util.parse("03 3316005extension.12345678901234567890", RegionCode::nz()).unwrap();
    assert_eq!(nz_number, test_number);

    let test_number = phone_util.parse("03 3316005 доб:12345678901234567890", RegionCode::nz()).unwrap();
    assert_eq!(nz_number, test_number);

    // Слишком длинное расширение.
    let result = phone_util.parse("03 3316005 extension 123456789012345678901", RegionCode::nz());
    assert!(result.is_err());
}


#[test]
fn test_parse_handles_long_extensions_with_auto_dialling_labels() {
    let phone_util = get_phone_util();
    // Во-вторых, случаи автодозвона и других стандартных меток добавочных номеров:
    // ext_limit_after_likely_label
    let mut us_number_user_input = PhoneNumber::new();
    us_number_user_input.set_country_code(1);
    us_number_user_input.set_national_number(2679000000);
    us_number_user_input.set_extension("123456789012345".to_string());

    let mut test_number = phone_util.parse("+12679000000,,123456789012345#", RegionCode::us()).unwrap();
    assert_eq!(us_number_user_input, test_number);

    test_number = phone_util.parse("+12679000000;123456789012345#", RegionCode::us()).unwrap();
    assert_eq!(us_number_user_input, test_number);

    let mut uk_number_user_input = PhoneNumber::new();
    uk_number_user_input.set_country_code(44);
    uk_number_user_input.set_national_number(2034000000);
    uk_number_user_input.set_extension("123456789".to_string());

    let test_number = phone_util.parse("+442034000000,,123456789#", RegionCode::gb()).unwrap();
    assert_eq!(uk_number_user_input, test_number);

    // Слишком длинное расширение.
    let result = phone_util.parse("+12679000000,,1234567890123456#", RegionCode::us());
    assert!(result.is_err());
}

#[test]
fn test_parse_handles_short_extensions_with_ambiguous_char() {
    let phone_util = get_phone_util();
    // В-третьих, для единичных и нестандартных случаев: ext_limit_after_ambiguous_char
    let mut nz_number = PhoneNumber::new();
    nz_number.set_country_code(64);
    nz_number.set_national_number(33316005);
    nz_number.set_extension("123456789".to_string());

    let mut test_number = phone_util.parse("03 3316005 x 123456789", RegionCode::nz()).unwrap();
    assert_eq!(nz_number, test_number);

    test_number = phone_util.parse("03 3316005 x. 123456789", RegionCode::nz()).unwrap();
    assert_eq!(nz_number, test_number);

    test_number = phone_util.parse("03 3316005 #123456789#", RegionCode::nz()).unwrap();
    assert_eq!(nz_number, test_number);

    test_number = phone_util.parse("03 3316005 ~ 123456789", RegionCode::nz()).unwrap();
    assert_eq!(nz_number, test_number);

    let result = phone_util.parse("03 3316005 ~ 1234567890", RegionCode::nz());
    assert!(result.is_err());
}

#[test]
fn test_parse_handles_short_extensions_when_not_sure_of_label() {
    let phone_util = get_phone_util();
    // В-третьих, когда нет явной метки расширения, но оно обозначено
    // конечным #: ext_limit_when_not_sure
    let mut us_number = PhoneNumber::new();
    us_number.set_country_code(1);
    us_number.set_national_number(1234567890);
    us_number.set_extension("666666".to_string());

    let mut test_number = phone_util.parse("+1123-456-7890 666666#", RegionCode::us()).unwrap();
    assert_eq!(us_number, test_number);

    us_number.set_extension("6".to_string());
    test_number = phone_util.parse("+11234567890-6#", RegionCode::us()).unwrap();
    assert_eq!(us_number, test_number);

    // Слишком длинное расширение.
    let result = phone_util.parse("+1123-456-7890 7777777#", RegionCode::us());
    assert!(result.is_err());
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