use criterion::{Criterion, criterion_group, criterion_main};
use forensics::core::parse_toml_file;
use std::path::PathBuf;
use tokio::runtime::Builder;

async fn bits(path: &str) {
    let _ = parse_toml_file(&path).await.unwrap();
}

fn bench_bits(c: &mut Criterion) {
    let mut test_path = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    test_path.push("tests/test_data/windows/benchmarks/bits.toml");

    c.bench_function("Benching BITS with Carving", |b| {
        let rt = Builder::new_current_thread().build().unwrap();

        b.to_async(rt)
            .iter(|| async { bits(&test_path.display().to_string()).await })
    });
}

criterion_group!(benches, bench_bits);
criterion_main!(benches);
