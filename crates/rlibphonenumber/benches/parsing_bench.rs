use criterion::{BatchSize, Criterion, black_box, criterion_group, criterion_main};

use phonenumber::{self as rlp, country::Id};
use rlibphonenumber::PHONE_NUMBER_UTIL;

fn setup_numbers_to_parse() -> Vec<(&'static str, &'static str, Id)> {
    vec![
        ("0011 54 9 11 8765 4321 ext. 1234", "AU", Id::AU),
        ("(650) 253-0000", "US", Id::US),
        ("+44 20 8765 4321", "GB", Id::GB),
        ("020 8765 4321", "GB", Id::GB),
        ("011 15-1234-5678", "AR", Id::AR),
        ("02 12345678", "IT", Id::IT),
        ("1-800-FLOWERS", "US", Id::US),
        ("12345", "DE", Id::DE),
        (" + 49 (0) 30 123456-78 ", "DE", Id::DE),
        ("++41-44-668-18-00", "CH", Id::CH),
        // Pos overflow on rlp
        // ("+55 11 98765-4321", "BR", Id::BR),
        ("+1 (646) 222-3333 ext. 987", "US", Id::US),
        ("112", "GB", Id::GB),
    ]
}

pub fn parsing_benchmark(c: &mut Criterion) {
    let numbers_to_parse = setup_numbers_to_parse();
    let mut group = c.benchmark_group("Parsing Comparison");

    group.bench_function("rlibphonenumber: parse()", |b| {
        let mut iter = numbers_to_parse.iter().cycle();
        b.iter_batched(
            || iter.next().unwrap(),
            |(number_str, region, _)| {
                let _ = PHONE_NUMBER_UTIL
                    .parse_with_default_region(black_box(number_str), black_box(region))
                    .unwrap();
            },
            BatchSize::SmallInput,
        )
    });

    group.bench_function("rust-phonenumber: parse()", |b| {
        let mut iter = numbers_to_parse.iter().cycle();
        b.iter_batched(
            || iter.next().unwrap(),
            |(number_str, _, region_id)| {
                let _ = rlp::parse(black_box(Some(*region_id)), black_box(number_str))
                    .expect(number_str);
            },
            BatchSize::SmallInput,
        )
    });

    group.bench_function("phonelib: parse()", |b| {
        let mut iter = numbers_to_parse.iter().cycle();
        b.iter_batched(
            || iter.next().unwrap(),
            |(number_str, region, _)| {
                let _ = phonelib::PhoneNumber::parse_with_country(
                    black_box(number_str),
                    black_box(region),
                )
                .expect(number_str);
            },
            BatchSize::SmallInput,
        )
    });

    group.finish();
}

criterion_group!(benches, parsing_benchmark);
criterion_main!(benches);
