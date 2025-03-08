use core::core::parse_toml_file;
use criterion::{Criterion, criterion_group, criterion_main};
use std::path::PathBuf;

fn userassist(path: &str) {
    let _ = parse_toml_file(&path).unwrap();
}

fn bench_userassist(c: &mut Criterion) {
    let mut test_path = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    test_path.push("tests/test_data/windows/benchmarks/userassist.toml");

    c.bench_function("Benching Userassist", |b| {
        b.iter(|| userassist(&test_path.display().to_string()))
    });
}

criterion_group!(benches, bench_userassist);
criterion_main!(benches);
