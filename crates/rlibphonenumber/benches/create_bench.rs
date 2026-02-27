use std::hint::black_box;

use criterion::{Criterion, criterion_group, criterion_main};
use rlibphonenumber::PhoneNumberUtil;

fn create_bench(c: &mut Criterion) {
    c.bench_function("Rlibphonenumber create::new", |b| {
        b.iter(|| {
            let _ = black_box(PhoneNumberUtil::new());
        });
    });
}

criterion_group!(benches, create_bench);
criterion_main!(benches);
