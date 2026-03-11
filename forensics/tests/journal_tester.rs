use common::linux::Journal;
use std::path::PathBuf;
use std::{
    fs::File,
    io::{BufRead, BufReader},
};

#[test]
#[cfg(target_os = "linux")]
fn test_journal_parser() {
    use forensics::core::parse_toml_file;
    use glob::glob;

    let mut test_location = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    test_location.push("tests/test_data/linux/journal.toml");

    parse_toml_file(&test_location.display().to_string()).unwrap();
    let mut output_location = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    output_location.push("tmp/journal_collection/*");

    let results = glob(output_location.to_str().unwrap()).unwrap();
    for result in results {
        let value = &result.unwrap();
        if value.to_str().unwrap().contains("report_") {
            use std::fs::read;

            let bytes = read(value).unwrap();
            let text = String::from_utf8(bytes).unwrap();
            if text.contains("\"total_output_files\":0,") {
                panic!("missing Journals??");
            }
            continue;
        }
        let output_file = value.to_str().unwrap();

        if output_file.contains("/journal_") && output_file.ends_with(".jsonl") {
            validate_output(value);
        }
        if value.extension().unwrap() == "log" && !value.to_str().unwrap().contains("status_") {
            check_errors(value);
        }
    }
}

#[cfg(target_os = "linux")]
fn validate_output(output: &PathBuf) {
    // Output is in JSONL based on the TOML file above!
    let file = File::open(output).unwrap();
    let reader = BufReader::new(file);
    for (_, line) in reader.lines().enumerate() {
        let value = line.unwrap();
        let info: Journal = serde_json::from_str(&value).unwrap();
        if info.message.is_empty() && info.machine_id.is_empty() {
            println!("{value}");
            panic!("no message?")
        }
        assert_ne!(info.realtime, "1970-01-01T00:00:00.000Z");
    }
}

#[cfg(target_os = "linux")]
fn check_errors(output: &PathBuf) {
    let file = File::open(output).unwrap();
    let reader = BufReader::new(file);
    let mut count = 0;
    for (_, line) in reader.lines().enumerate() {
        let value = line.unwrap();
        if value.contains("Failed to get UTF8 string") {
            continue;
        }
        if value.contains("Unused") {
            continue;
        }
        println!("{value}");
        count += 1;
    }

    if count > 0 {
        panic!("Got errors: {count}");
    }
}

#[test]
fn read_sample_output() {
    let mut test_location = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    test_location.push("tests/test_data/samples/linux/journals.jsonl");

    let file = File::open(&test_location).unwrap();
    let reader = BufReader::new(file);
    for (_, line) in reader.lines().enumerate() {
        let value = line.unwrap();
        let info: Journal = serde_json::from_str(&value).unwrap();
        if info.message.is_empty() && info.machine_id.is_empty() {
            panic!("no message?")
        }
        assert_ne!(info.realtime, "1970-01-01T00:00:00.000Z");
    }
}
