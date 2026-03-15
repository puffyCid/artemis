use common::linux::Ext4Filelist;
use forensics::{
    core::artemis_collection,
    structs::{
        artifacts::os::linux::Ext4Options,
        toml::{ArtemisToml, Artifacts, Output},
    },
};
use glob::glob;
use std::{
    fs::{File, read},
    io::{BufRead, BufReader},
    path::PathBuf,
};

#[test]
fn test_qcow_parser() {
    let mut test_location = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    test_location.push("tests/test_data/disks/qcow/test.qcow");

    let options = Ext4Options {
        start_path: String::from("/"),
        depth: 99,
        device: format!("qcow://{}", test_location.to_str().unwrap()).into(),
        md5: Some(true),
        sha1: None,
        sha256: None,
        path_regex: None,
        filename_regex: None,
    };

    let out = Output {
        name: String::from("qcow_collection"),
        endpoint_id: String::from("ci"),
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
            artifact_name: String::from("rawfiles-ext4"),
            rawfiles_ext4: Some(options),
            ..Default::default()
        }],
        marker: None,
    };

    artemis_collection(&mut data).unwrap();

    let mut output_location = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    output_location.push("tmp/qcow_collection/*");

    let results = glob(output_location.to_str().unwrap()).unwrap();
    for result in results {
        let value = &result.unwrap();
        if value.to_str().unwrap().contains("report_") {
            let bytes = read(value).unwrap();
            let text = String::from_utf8(bytes).unwrap();
            if text.contains("\"total_output_files\":0,") {
                panic!("missing QCOW??");
            }
            continue;
        }
        let output_file = value.to_str().unwrap();

        if output_file.contains("ext4files_") && output_file.ends_with(".jsonl") {
            validate_output(value);
        }
        if value.extension().unwrap() == "log" && !value.to_str().unwrap().contains("status_") {
            check_errors(value);
        }
    }
}

fn validate_output(output: &PathBuf) {
    // Output is in JSONL based on the TOML file above!
    let file = File::open(output).unwrap();
    let reader = BufReader::new(file);
    for (_, line) in reader.lines().enumerate() {
        let value = line.unwrap();
        let info: Ext4Filelist = serde_json::from_str(&value).unwrap();
        if info.full_path.is_empty() {
            println!("{value}");
            panic!("no path?")
        }
        assert!(info.full_path.starts_with("/mnt/vda1/"));
        assert_ne!(info.modified, "1970-01-01T00:00:00.000Z");
    }
}

fn check_errors(output: &PathBuf) {
    let file = File::open(output).unwrap();
    let reader = BufReader::new(file);

    let mut count = 0;
    for (_, line) in reader.lines().enumerate() {
        let value = line.unwrap();
        if value.contains("Did not read expected number of bytes") {
            continue;
        }
        println!("{value}");

        count += 1;
    }

    if count != 0 {
        panic!("error count: {count}");
    }
}
