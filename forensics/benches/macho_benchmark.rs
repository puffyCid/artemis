use forensics::core::parse_toml_file;
use criterion::{Criterion, criterion_group, criterion_main};
use std::path::PathBuf;

fn macho_files(path: &str) {
    let _ = parse_toml_file(&path).unwrap();
}

fn bench_macho_files(c: &mut Criterion) {
    let mut test_path = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    test_path.push("tests/test_data/macos/benchmarks/macho_files.toml");

    c.bench_function("Benching Macho parsing Filelisting", |b| {
        b.iter(|| macho_files(&test_path.display().to_string()))
    });
}

criterion_group!(benches, bench_macho_files);
criterion_main!(benches);
