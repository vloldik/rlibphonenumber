use criterion::{BatchSize, Criterion, black_box, criterion_group, criterion_main};

use rlibphonenumber::{PHONE_NUMBER_UTIL, PhoneNumberFormat, Region};

use phonenumber::{
    self as rlp, Mode,
    country::Id::{self, AR, AU, BS, CA, CH, DE, GB, IL, IT, JP, KR, KZ, MX, NZ, RU, SG, US},
};

use phonelib::{PhoneFormat, PhoneNumber as PhonelibNumber};

type TestEntity = (&'static str, &'static str, Id);

fn setup_numbers() -> Vec<TestEntity> {
    vec![
        ("+44 20 8765 4321", "GB", GB),
        ("020 8765 4321", "GB", GB),
        ("(650) 253-0000", Region::US.as_ref(), US),
        ("02 12345678", "IT", IT),
        ("0011 54 9 11 8765 4321 ext. 1234", Region::AU.as_ref(), AU),
        ("011 15-1234-5678", "AR", AR),
        ("+1 (646) 222-3333 ext. 987", "US", US),
        ("1-800-FLOWERS", "US", US),
        ("1300 FLIGHT", "AU", AU),
        (" + 49 (0) 30 123456-78 ", "DE", DE),
        ("++41-44-668-18-00", "CH", CH),
        ("(03) 1234 5678", "JP", JP),
        // This number breaks rlp
        // ("+55 11 98765-4321", "BR", BR),
        ("+52 1 55 1234 5678", "MX", MX),
        ("054-123-4567", "IL", IL),
        ("+65 9123 4567", "SG", SG),
        ("416-555-0198", "CA", CA),
        ("242-322-1234", "BS", BS),
        ("12345", "DE", DE),
        ("112", "GB", GB),
        ("02-2123-4567", "KR", KR),
        ("0800 83 83 83", "NZ", NZ),
        ("+7 916 123 4567", "RU", RU),
        ("+7 727 250 1234", "KZ", KZ),
    ]
}

fn convert_to_rlp_numbers(numbers: &[TestEntity]) -> Vec<rlp::PhoneNumber> {
    numbers
        .iter()
        .map(|s| rlp::parse(Some(s.2), s.0).expect(s.0.as_str()))
        .collect()
}

fn convert_to_rlibphonenumber_numbers(numbers: &[TestEntity]) -> Vec<rlibphonenumber::PhoneNumber> {
    numbers
        .iter()
        .map(|s| {
            PHONE_NUMBER_UTIL
                .parse_with_default_region(s.0, s.1)
                .unwrap()
        })
        .collect()
}

fn convert_to_phonelib_numbers(numbers: &[TestEntity]) -> Vec<PhonelibNumber> {
    numbers
        .iter()
        .flat_map(|s| PhonelibNumber::parse_with_country(s.0, s.1))
        .collect()
}
fn formatting_benchmark(c: &mut Criterion) {
    let numbers_data = setup_numbers();

    let rlib_numbers = convert_to_rlibphonenumber_numbers(&numbers_data);
    let rlp_numbers = convert_to_rlp_numbers(&numbers_data);
    let phonelib_numbers = convert_to_phonelib_numbers(&numbers_data);

    let mut group = c.benchmark_group("Formatting Comparison");

    for (number_a, number_b) in rlp_numbers.iter().zip(rlib_numbers.iter()) {
        assert_eq!(
            rlp::format(number_a).mode(Mode::E164).to_string(), // Пример валидации
            PHONE_NUMBER_UTIL.format(number_b, PhoneNumberFormat::E164)
        );
    }

    let mut test = |format_a: PhoneNumberFormat, format_b: Mode, format_c: PhoneFormat| {
        // 1. rlibphonenumber
        group.bench_function(format!("rlibphonenumber: format({:?})", format_a), |b| {
            let mut iter = rlib_numbers.iter().cycle();
            b.iter_batched(
                || iter.next().unwrap(),
                |number| PHONE_NUMBER_UTIL.format(black_box(number), black_box(format_a)),
                BatchSize::SmallInput,
            )
        });

        // 2. rust-phonenumber (rlp)
        group.bench_function(format!("rust-phonenumber: format({:?})", format_b), |b| {
            let mut iter = rlp_numbers.iter().cycle();
            b.iter_batched(
                || iter.next().unwrap(),
                |number| {
                    rlp::format(black_box(number))
                        .mode(black_box(format_b))
                        .to_string()
                },
                BatchSize::SmallInput,
            )
        });

        // 3. phonelib
        group.bench_function(format!("phonelib: format({:?})", format_a), |b| {
            let mut iter = phonelib_numbers.iter().cycle();
            b.iter_batched(
                || iter.next().unwrap(),
                |number| number.format(black_box(format_c)),
                BatchSize::SmallInput,
            )
        });
    };

    test(PhoneNumberFormat::E164, Mode::E164, PhoneFormat::E164);
    test(
        PhoneNumberFormat::International,
        Mode::International,
        PhoneFormat::International,
    );
    test(
        PhoneNumberFormat::National,
        Mode::National,
        PhoneFormat::National,
    );
    test(
        PhoneNumberFormat::RFC3966,
        Mode::Rfc3966,
        PhoneFormat::RFC3966,
    );

    group.finish();
}

criterion_group!(benches, formatting_benchmark);
criterion_main!(benches);
