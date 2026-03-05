use common::windows::{BitsInfo, JobType};
use std::path::PathBuf;
use std::{
    fs::File,
    io::{BufRead, BufReader},
};

#[test]
#[cfg(target_os = "windows")]
fn test_bits_parser() {
    use forensics::core::parse_toml_file;
    use glob::glob;
    use std::fs::read;

    let mut test_location = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    test_location.push("tests/test_data/windows/bits.toml");

    parse_toml_file(&test_location.display().to_string()).unwrap();
    let mut output_location = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    output_location.push("tmp/bits_collection/*");

    let results = glob(output_location.to_str().unwrap()).unwrap();
    for result in results {
        let value = &result.unwrap();
        if value.to_str().unwrap().contains("report_") {
            let bytes = read(value).unwrap();
            let text = String::from_utf8(bytes).unwrap();
            if text.contains("\"output_count\":0,") {
                panic!("missing BITS??");
            }
            continue;
        }
        let output_file = value.to_str().unwrap();

        if output_file.contains("\\bits_") && output_file.ends_with(".jsonl") {
            validate_output(value);
        }
        if value.ends_with(".log") && !value.starts_with("status_") {
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
        let info: BitsInfo = serde_json::from_str(&value).unwrap();
        if info.carved {
            if !info.job_id.is_empty() {
                assert_ne!(info.created, "1970-01-01T00:00:00.000Z");
                assert!(!info.job_id.is_empty());
                assert_ne!(info.job_type, JobType::Unknown);
            }
            continue;
        }
        assert!(!info.job_name.is_empty());
        assert!(!info.target_path.is_empty());
    }
}

fn check_errors(output: &PathBuf) {
    let file = File::open(output).unwrap();
    let reader = BufReader::new(file);

    let mut count = 0;
    for (_, line) in reader.lines().enumerate() {
        let value = line.unwrap();
        println!("{value}");
        count += 1;
    }

    if count != 0 {
        panic!("error count: {count}");
    }
}
