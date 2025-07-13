use criterion::{Criterion, black_box, criterion_group, criterion_main};

use rlibphonenumber::{PhoneNumberFormat, PHONE_NUMBER_UTIL};

use phonenumber::{
    self as rlp,
    country::Id::{self, AR, AU, DE, GB, IT, US}, Mode,
};

type TestEntity = (&'static str, &'static str, Id);

fn setup_numbers() -> Vec<TestEntity> {
    vec![
        ("0011 54 9 11 8765 4321 ext. 1234", "AU", AU),
        ("(650) 253-0000", "US", US),
        ("+44 20 8765 4321", "GB", GB),
        ("020 8765 4321", "GB", GB),
        ("011 15-1234-5678", "AR", AR),
        ("02 12345678", "IT", IT),
        ("1-800-FLOWERS", "US", US),
        ("12345", "DE", DE),
    ]
}

fn convert_to_rlp_numbers(numbers: &[TestEntity]) -> Vec<rlp::PhoneNumber> {
    numbers
        .iter()
        .map(|s| rlp::parse(Some(s.2), s.0).unwrap())
        .collect()
}

fn convert_to_rlibphonenumber_numbers(
    numbers: &[TestEntity],
) -> Vec<rlibphonenumber::PhoneNumber> {
    numbers
        .iter()
        .map(|s| PHONE_NUMBER_UTIL.parse(s.0, s.1).unwrap())
        .collect()
}

fn formatting_benchmark(c: &mut Criterion) {
    let numbers = setup_numbers();
    let rlp_numbers = convert_to_rlp_numbers(&numbers);
    let numbers = convert_to_rlibphonenumber_numbers(&numbers);

    let mut group = c.benchmark_group("Formatting Comparison");

    let mut test = |format_a: PhoneNumberFormat, format_b: Mode| {
        group.bench_function(format!("rlibphonenumber: format({:?})", format_a), |b| {
            b.iter(|| {
                for number in &numbers {
                    PHONE_NUMBER_UTIL
                        .format(black_box(number), black_box(format_a))
                            .unwrap();
                }
            })
        });

        group.bench_function(format!("rust-phonenumber: format({:?})", format_b), |b| {
            b.iter(|| {
                for number in &rlp_numbers {
                    rlp::format(black_box(number)).mode(format_b).to_string();
                }
            })
        });

        for (number_a, number_b) in rlp_numbers.iter().zip(numbers.iter()) {
            assert_eq!(
                rlp::format(number_a).mode(format_b).to_string(),
                PHONE_NUMBER_UTIL
                    .format(number_b, format_a)
                    .unwrap()
            );
        }
    };

    test(PhoneNumberFormat::E164, Mode::E164);
    test(PhoneNumberFormat::International, Mode::International);
    test(PhoneNumberFormat::National, Mode::National);
    test(PhoneNumberFormat::RFC3966, Mode::Rfc3966);
    group.finish();
}

criterion_group!(benches, formatting_benchmark);
criterion_main!(benches);
