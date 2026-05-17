use criterion::{Criterion, criterion_group, criterion_main};
use forensics::{
    core::artemis_collection,
    structs::{
        artifacts::os::windows::JumplistsOptions,
        toml::{ArtemisToml, Artifacts, Output},
    },
};
use std::path::PathBuf;

fn jumplists(data: &mut ArtemisToml) {
    artemis_collection(data).unwrap();
}

fn bench_custom_jumplists(c: &mut Criterion) {
    let mut test_location = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    test_location.push(
        "tests/test_data/windows/jumplists/win10/custom/1ced32d74a95c7bc.customDestinations-ms",
    );

    let options = JumplistsOptions {
        alt_dir: Some(test_location.display().to_string()),
    };

    let out = Output {
        name: String::from("jumplist_benchmark"),
        endpoint_id: String::from("benchmark_jumplists"),
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
            artifact_name: String::from("jumplists"),
            jumplists: Some(options),
            ..Default::default()
        }],
        marker: None,
    };

    c.bench_function("Benching Custom Jumplists", |b| {
        b.iter(|| jumplists(&mut data))
    });
}

fn bench_automatic_jumplists(c: &mut Criterion) {
    let mut test_location = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    test_location.push(
        "tests/test_data/windows/jumplists/win11/automatic/3d2110c4a0cb6d15.automaticDestinations-ms",
    );

    let options = JumplistsOptions {
        alt_dir: Some(test_location.display().to_string()),
    };

    let out = Output {
        name: String::from("jumplist_benchmark"),
        endpoint_id: String::from("benchmark_jumplists"),
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
            artifact_name: String::from("jumplists"),
            jumplists: Some(options),
            ..Default::default()
        }],
        marker: None,
    };

    c.bench_function("Benching Automatic Jumplists", |b| {
        b.iter(|| jumplists(&mut data))
    });
}

criterion_group!(benches, bench_custom_jumplists, bench_automatic_jumplists);
criterion_main!(benches);
