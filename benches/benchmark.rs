use criterion::{black_box, criterion_group, criterion_main, Criterion};

use os_display::Quotable;

fn benchmark(c: &mut Criterion) {
    let mut group = c.benchmark_group("small");
    group.bench_function("trivial small string", |b| {
        b.iter(|| black_box("foobar.barbaz").maybe_quote().to_string())
    });
    group.bench_function("small unicode string", |b| {
        b.iter(|| black_box("µ—ßə€≠→←").maybe_quote().to_string())
    });
}

criterion_group!(benches, benchmark);
criterion_main!(benches);
