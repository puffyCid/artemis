use criterion::{Criterion, criterion_group, criterion_main};
use forensics::core::parse_toml_file;
use std::path::PathBuf;
use tokio::runtime::Builder;

async fn userassist(path: &str) {
    let _ = parse_toml_file(&path).await.unwrap();
}

fn bench_userassist(c: &mut Criterion) {
    let mut test_path = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    test_path.push("tests/test_data/windows/benchmarks/userassist.toml");

    c.bench_function("Benching Userassist", |b| {
        let rt = Builder::new_current_thread().build().unwrap();

        b.to_async(rt)
            .iter(|| async { userassist(&test_path.display().to_string()).await })
    });
}

criterion_group!(benches, bench_userassist);
criterion_main!(benches);
