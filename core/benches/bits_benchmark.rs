use core::core::parse_toml_file;
use criterion::{criterion_group, criterion_main, Criterion};
use std::path::PathBuf;

fn bits(path: &str) {
    let _ = parse_toml_file(&path).unwrap();
}

fn bench_bits(c: &mut Criterion) {
    let mut test_path = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    test_path.push("tests/test_data/windows/benchmarks/bits.toml");

    c.bench_function("Benching BITS with Carving", |b| {
        b.iter(|| bits(&test_path.display().to_string()))
    });
}

criterion_group!(benches, bench_bits);
criterion_main!(benches);
