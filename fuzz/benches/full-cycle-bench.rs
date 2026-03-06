use criterion::{Criterion, black_box, criterion_group, criterion_main};
use rlibphonenumber::{PHONE_NUMBER_UTIL, PhoneNumberFormat};
use rlibphonenumber_fuzz::ffi::{self, CppResult};

fn bench_rust_pure(number_str: &str, region_str: &str) -> bool {
    let util = &PHONE_NUMBER_UTIL;
    if let Ok(rust_num) = util.parse_with_default_region(number_str, region_str) {
        if util.is_valid_number(&rust_num) {
            let fmt_e164 = util.format(&rust_num, PhoneNumberFormat::E164);
            return !fmt_e164.is_empty();
        }
    }
    false
}

fn test_rust_impl(number_str: &str, region_str: &str) -> CppResult {
    let util = &PHONE_NUMBER_UTIL;
    let parsed = util.parse_with_default_region(number_str, region_str);

    let mut res = CppResult {
        is_parsed: parsed.is_ok(),
        is_valid: false,
        is_possible: false,
        region_code: String::from("ZZ"),
        nsn: String::new(),
        format_e164: String::new(),
        format_intl: String::new(),
        format_natl: String::new(),
        format_rfc3966: String::new(),
        format_mobile: String::new(),
        out_of_country_keeping_alpha: String::new(),
        error: String::new(),
    };

    match parsed {
        Ok(rust_num) => {
            res.is_valid = util.is_valid_number(&rust_num);
            res.is_possible = util.is_possible_number(&rust_num);

            res.region_code = util
                .get_region_code_for_number(&rust_num)
                .unwrap_or("ZZ")
                .to_string();

            res.nsn = PHONE_NUMBER_UTIL
                .get_national_significant_number(&rust_num)
                .to_string();

            if res.is_valid {
                res.format_e164 = util.format(&rust_num, PhoneNumberFormat::E164).to_string();
                res.format_intl = util
                    .format(&rust_num, PhoneNumberFormat::International)
                    .to_string();
                res.format_natl = util
                    .format(&rust_num, PhoneNumberFormat::National)
                    .to_string();
                res.format_rfc3966 = util
                    .format(&rust_num, PhoneNumberFormat::RFC3966)
                    .to_string();

                res.format_mobile = util
                    .format_number_for_mobile_dialing(&rust_num, region_str, true)
                    .unwrap_or_default()
                    .to_string();

                res.out_of_country_keeping_alpha = util
                    .format_out_of_country_keeping_alpha_chars(&rust_num, region_str)
                    .to_string();
            }
        }
        Err(e) => {
            res.error = e.to_string();
        }
    }

    res
}

fn bench_phonenumbers(c: &mut Criterion) {
    let mut group = c.benchmark_group("Phonenumber_Parse_And_Format");

    let test_cases = vec![
        ("+14155552671", "US"),
        ("07400123456", "GB"),
        ("88005553535", "RU"),
        ("12345", "US"),
        ("invalid_alpha", "DE"),
    ];

    for (number, region) in test_cases {
        let test_name = format!("Num: '{}', Reg: '{}'", number, region);

        group.bench_function(format!("C++  | {}", test_name), |b| {
            b.iter(|| ffi::test_cpp_impl(black_box(number), black_box(region)))
        });

        group.bench_function(format!("Rust | {}", test_name), |b| {
            b.iter(|| test_rust_impl(black_box(number), black_box(region)))
        });
    }

    group.finish();
}

fn bench_pure_core(c: &mut Criterion) {
    let mut group = c.benchmark_group("Pure_Core_No_FFI_Overhead");

    let test_cases = vec![
        ("+14155552671", "US"),
        ("07400123456", "GB"),
        ("88005553535", "RU"),
        ("12345", "US"),
        ("invalid_alpha", "DE"),
    ];

    for (number, region) in test_cases {
        let test_name = format!("Num: '{}', Reg: '{}'", number, region);

        group.bench_function(format!("C++  | {}", test_name), |b| {
            b.iter(|| ffi::bench_cpp_pure(black_box(number), black_box(region)))
        });

        group.bench_function(format!("Rust | {}", test_name), |b| {
            b.iter(|| bench_rust_pure(black_box(number), black_box(region)))
        });
    }

    group.finish();
}

criterion_group!(benches, bench_phonenumbers, bench_pure_core);
criterion_main!(benches);
