use common::windows::EventMessage;
use std::path::PathBuf;
use std::{
    fs::File,
    io::{BufRead, BufReader},
};

#[test]
#[cfg(target_os = "windows")]
fn test_eventlogs_parser() {
    use forensics::core::parse_toml_file;
    use glob::glob;
    use std::fs::read;

    let mut test_location = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    test_location.push("tests/test_data/windows/eventlogs.toml");

    parse_toml_file(&test_location.display().to_string()).unwrap();
    let mut output_location = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    output_location.push("tmp/eventlog_collection/*");

    let results = glob(output_location.to_str().unwrap()).unwrap();
    for result in results {
        let value = &result.unwrap();
        if value.to_str().unwrap().contains("report_") {
            let bytes = read(value).unwrap();
            let text = String::from_utf8(bytes).unwrap();
            if text.contains("\"output_count\":0,") {
                panic!("missing EventLogs??");
            }
            continue;
        }
        let output_file = value.to_str().unwrap();

        if output_file.contains("\\eventlog_") && output_file.ends_with(".jsonl") {
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
        let info: EventMessage = serde_json::from_str(&value).unwrap();
        if info.provider.is_empty() && info.guid.is_empty() {
            println!("{value}");
            panic!("no provider?")
        }
        assert_ne!(info.generated, "1970-01-01T00:00:00.000Z");
        if info.message.contains("%%")
            || info.message.contains("TEMP_ARTEMIS_VALUE") && info.event_id != 4674
        {
            println!("EventLog with parameter string value?: {value}");
            panic!("Message still contains parameter value?")
        }

        if info.evidence.contains("Application.evtx") {
            println!("{value}");
            assert!(!info.message.is_empty())
        }
    }
}

fn check_errors(output: &PathBuf) {
    let file = File::open(output).unwrap();
    let reader = BufReader::new(file);

    let mut count = 0;
    let mut missing_mui = 0;
    for (_, line) in reader.lines().enumerate() {
        let value = line.unwrap();

        // Warnings from the evtx library
        if value.contains("[WARN] invalid boolean value") {
            continue;
        }
        // Strings may be either UTF8 or UTF16. We check for both
        if value.contains("[strings] Failed to get UTF8 string") {
            continue;
        }
        println!("{value}");

        // Not uncommon for Windows to not have all expected MUI files
        if value.contains("No MUI file at") {
            missing_mui += 1;
            continue;
        }

        count += 1;
    }

    if count != 0 {
        panic!("error count: {count}");
    }
    if missing_mui > 10 {
        panic!("Lots of missing MUI files?: {missing_mui}");
    }
}

#[test]
fn read_ci_output() {
    let mut test_location = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    test_location.push("tests/test_data/github_ci/windows/eventlogs.jsonl");

    let file = File::open(&test_location).unwrap();
    let reader = BufReader::new(file);
    for (_, line) in reader.lines().enumerate() {
        let value = line.unwrap();
        let info: EventMessage = serde_json::from_str(&value).unwrap();
        if info.provider.is_empty() && info.guid.is_empty() {
            panic!("no provider?")
        }
        assert_ne!(info.generated, "1970-01-01T00:00:00.000Z");

        if info.evidence.contains("Application.evtx") {
            assert!(!info.message.is_empty())
        }
    }
}
