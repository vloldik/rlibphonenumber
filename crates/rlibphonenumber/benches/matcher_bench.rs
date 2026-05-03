use std::hint::black_box;

use criterion::{Criterion, criterion_group, criterion_main};
use rlibphonenumber::{Region, phonenumber_matcher::FindNumberExt};

fn matcher_bench(c: &mut Criterion) {
    let text_no_numbers = "Hello world! This is a simple test text without any phone numbers. Please email us at test@example.com.";
    let text_with_numbers = "Call me at +1 (415) 555-2671 or my office at +44 20 7946 0958. For RU support call +7 900 123 45 67.";

    let chunk_text = text_with_numbers.repeat(100);

    let mut group = c.benchmark_group("PhoneNumberMatcher");

    group.bench_function("Short text (No matches)", |b| {
        b.iter(|| {
            let matcher = text_no_numbers.find_phone_numbers_with_preferred_region(Region::US);
            for match_obj in matcher {
                black_box(match_obj);
            }
        });
    });

    group.bench_function("Short text (3 matches)", |b| {
        b.iter(|| {
            let matcher = text_with_numbers.find_phone_numbers_with_preferred_region(Region::US);

            for match_obj in matcher {
                black_box(match_obj);
            }
        });
    });

    group.bench_function("Long chunk (300 matches)", |b| {
        b.iter(|| {
            let matcher = chunk_text.find_phone_numbers_with_preferred_region(Region::US);

            for match_obj in matcher {
                black_box(match_obj);
            }
        });
    });

    group.finish();
}

criterion_group!(benches, matcher_bench);
criterion_main!(benches);
