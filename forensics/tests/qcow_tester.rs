use common::files::FileExt4Info;
use forensics::{
    core::artemis_collection,
    structs::{
        artifacts::os::files::FileOptions,
        toml::{ArtemisToml, Artifacts, OutputConfig, OutputDestination, OutputFormat},
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

    let options = FileOptions {
        start_path: String::from("/"),
        depth: Some(99),
        source: format!("qcow://{}", test_location.to_str().unwrap()).into(),
        md5: true,
        sha1: false,
        sha256: false,
        path_regex: None,
        filename_regex: None,
        metadata: false,
        yara: None,
        exclude_directories: None,
        verbose: false,
    };

    let out = OutputConfig {
        name: String::from("qcow_collection"),
        endpoint_id: String::from("ci"),
        collection_id: 0,
        directory: PathBuf::from("./tmp"),
        destination: OutputDestination::Local,
        format: OutputFormat::Jsonl,
        compress: false,
        ..Default::default()
    };

    let data = ArtemisToml {
        output: out,
        artifacts: vec![Artifacts {
            artifact_name: String::from("files"),
            files: Some(options),
            ..Default::default()
        }],
        marker: None,
    };

    artemis_collection(data).unwrap();

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

        if output_file.contains("files_qcow")
            && output_file.ends_with(".jsonl")
            && !output_file.contains("/artemis_")
        {
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
        let info: FileExt4Info = serde_json::from_str(&value).unwrap();
        if info.full_path.is_empty() {
            println!("{value}");
            panic!("no path?")
        }
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
        println!("End2End test has error: {value}");

        count += 1;
    }

    if count != 0 {
        panic!("error count: {count}");
    }
}
