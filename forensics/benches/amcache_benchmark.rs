use criterion::{Criterion, criterion_group, criterion_main};
use forensics::{
    core::artemis_collection,
    structs::{
        artifacts::os::windows::AmcacheOptions,
        toml::{ArtemisToml, Artifacts, Output},
    },
};
use std::path::PathBuf;

fn amcache(data: &mut ArtemisToml) {
    artemis_collection(data).unwrap();
}

fn bench_amcache(c: &mut Criterion) {
    let mut test_location = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    test_location.push("tests/test_data/windows/amcache/win81/Amcache.hve");

    let options = AmcacheOptions {
        alt_file: Some(test_location.to_string_lossy().to_string()),
    };

    let out = Output {
        name: String::from("amcache_benchmark"),
        endpoint_id: String::from("benchmark_amcache"),
        collection_id: 0,
        directory: String::from("./tmp"),
        output: String::from("local"),
        format: String::from("jsonl"),
        compress: false,
        timeline: false,
        ..Default::default()
    };

    let mut data = ArtemisToml {
        output: out,
        artifacts: vec![Artifacts {
            artifact_name: String::from("amcache"),
            amcache: Some(options),
            ..Default::default()
        }],
        marker: None,
    };

    c.bench_function("Benching Amcache", |b| b.iter(|| amcache(&mut data)));
}

criterion_group!(benches, bench_amcache);
criterion_main!(benches);
