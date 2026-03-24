use forensics::core::parse_toml_file;
use glob::glob;
use std::fs::{File, read};
use std::io::{BufRead, BufReader};
use std::path::PathBuf;

#[test]
fn test_report_parser() {
    let mut test_location = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    test_location.push("tests/test_data/windows/quick.toml");

    parse_toml_file(&test_location.display().to_string()).unwrap();
    let mut output_location = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    output_location.push("tmp/windows_quick/*");

    let results = glob(output_location.to_str().unwrap()).unwrap();

    for result in results {
        let value = &result.unwrap();
        if value.to_str().unwrap().contains("report_") {
            let bytes = read(value).unwrap();
            let text = String::from_utf8(bytes).unwrap();
            if !text.contains("\"total_output_files\":2,") {
                panic!("missing Quick report??");
            }
            continue;
        }
        let output_file = value.to_str().unwrap();

        if output_file.contains("systeminfo_") && output_file.ends_with(".jsonl") {
            validate_output(value);
        }
        if output_file.contains("processes_") && output_file.ends_with(".jsonl") {
            validate_output(value);
        }
    }
}

fn validate_output(output: &PathBuf) {
    // Output is in JSONL based on the TOML file above!
    let file = File::open(output).unwrap();
    let reader = BufReader::new(file);
    for (_, line) in reader.lines().enumerate() {
        let value = line.unwrap();
        if value.is_empty() {
            panic!("Empty resutls for {output:?}?");
        }
    }
}
