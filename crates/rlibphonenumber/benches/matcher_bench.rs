use std::hint::black_box;

use criterion::{BenchmarkId, Criterion, criterion_group, criterion_main};
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

/// Benchmarks the automatic region detection (`auto_region`) on its own, across
/// the same kinds of input used for the fixed-region matcher.
fn matcher_auto_region_bench(c: &mut Criterion) {
    let text_no_numbers = "Hello world! This is a simple test text without any phone numbers. Please email us at test@example.com.";
    // National-format numbers from several different regions: this is the case
    // auto detection is designed for and where it does the most work, since a
    // candidate may have to be tried against many regions before one matches.
    let text_with_numbers =
        "GB: 020 7946 0958, FR: 01 70 18 99 00, DE: 030 901820, US: (415) 555-2671.";

    let chunk_text = text_with_numbers.repeat(100);

    let mut group = c.benchmark_group("PhoneNumberMatcher/AutoRegion");

    group.bench_function("Short text (No matches)", |b| {
        b.iter(|| {
            let matcher = text_no_numbers.find_phone_numbers_auto_region();
            for match_obj in matcher {
                black_box(match_obj);
            }
        });
    });

    group.bench_function("Short text (multi-region matches)", |b| {
        b.iter(|| {
            let matcher = text_with_numbers.find_phone_numbers_auto_region();
            for match_obj in matcher {
                black_box(match_obj);
            }
        });
    });

    group.bench_function("Long chunk (multi-region matches)", |b| {
        b.iter(|| {
            let matcher = chunk_text.find_phone_numbers_auto_region();
            for match_obj in matcher {
                black_box(match_obj);
            }
        });
    });

    group.finish();
}

/// Head-to-head comparison of a pre-set fixed region versus automatic region
/// detection on identical inputs, so the overhead of auto detection is directly
/// visible.
fn region_strategy_comparison_bench(c: &mut Criterion) {
    // Numbers that are all valid for the fixed region (US), so the fixed-region
    // matcher resolves them on its first try. Auto detection still has to probe
    // regions until it finds a valid one.
    let same_region_text =
        "Call (415) 555-2671, then (650) 253-0000, then (202) 456-1111 for info.";
    // International (`+`) numbers: auto detection short-circuits to a regionless
    // parse, so the two strategies should be very close here.
    let intl_text =
        "Reach us on +1 415 555 2671, +44 20 7946 0958, +49 30 901820, +33 1 70 18 99 00.";
    // The MRU cache should make a long run of same-region numbers cheap for the
    // auto matcher after the first match.
    let long_same_region = same_region_text.repeat(100);

    let mut group = c.benchmark_group("RegionStrategy");

    for (label, text) in [
        ("same_region_national", same_region_text),
        ("international", intl_text),
        ("long_same_region", long_same_region.as_str()),
    ] {
        group.bench_with_input(BenchmarkId::new("fixed_region", label), text, |b, text| {
            b.iter(|| {
                let matcher = text.find_phone_numbers_with_preferred_region(Region::US);
                for match_obj in matcher {
                    black_box(match_obj);
                }
            });
        });

        group.bench_with_input(BenchmarkId::new("auto_region", label), text, |b, text| {
            b.iter(|| {
                let matcher = text.find_phone_numbers_auto_region();
                for match_obj in matcher {
                    black_box(match_obj);
                }
            });
        });
    }

    group.finish();
}

criterion_group!(
    benches,
    matcher_bench,
    matcher_auto_region_bench,
    region_strategy_comparison_bench
);
criterion_main!(benches);
