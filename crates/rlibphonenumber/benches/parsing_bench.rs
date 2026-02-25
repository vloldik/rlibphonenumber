use criterion::{Criterion, black_box, criterion_group, criterion_main};

use phonelib::PhoneNumber as PhonelibNumber;
use phonenumber::{self as rlp, country::Id};
use rlibphonenumber::{PHONE_NUMBER_UTIL, PhoneNumberUtil};

type TestEntity = (&'static str, &'static str, Id);

fn setup_parsing_data() -> Vec<TestEntity> {
    use phonenumber::country::Id::*;
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

fn parsing_benchmark(c: &mut Criterion) {
    let numbers_to_parse = setup_parsing_data();

    let mut group = c.benchmark_group("Parsing Comparison");

    group.bench_function("create::new", |b| {
        b.iter(|| {
            let _ = black_box(PhoneNumberUtil::new());
        });
    });
    group.bench_function("rlibphonenumber: parse()", |b| {
        b.iter(|| {
            for (number_str, region, _) in &numbers_to_parse {
                let _ = PHONE_NUMBER_UTIL
                    .parse_with_default_region(black_box(number_str), black_box(region));
            }
        })
    });

    group.bench_function("rust-phonenumber: parse()", |b| {
        b.iter(|| {
            for (number_str, _, region_id) in &numbers_to_parse {
                let _ = rlp::parse(black_box(Some(*region_id)), black_box(number_str));
            }
        })
    });

    group.bench_function("phonelib: parse()", |b| {
        b.iter(|| {
            for (number_str, region, _) in &numbers_to_parse {
                let _ =
                    PhonelibNumber::parse_with_country(black_box(number_str), black_box(region));
            }
        })
    });

    group.finish();
}

criterion_group!(benches, parsing_benchmark);
criterion_main!(benches);
