use core::core::parse_toml_file;
use criterion::{Criterion, criterion_group, criterion_main};
use std::path::PathBuf;

fn shimcache(path: &str) {
    let _ = parse_toml_file(&path).unwrap();
}

fn bench_shimcache(c: &mut Criterion) {
    let mut test_path = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    test_path.push("tests/test_data/windows/benchmarks/shimcache.toml");

    c.bench_function("Benching Shimcache with Compression", |b| {
        b.iter(|| shimcache(&test_path.display().to_string()))
    });
}

criterion_group!(benches, bench_shimcache);
criterion_main!(benches);
