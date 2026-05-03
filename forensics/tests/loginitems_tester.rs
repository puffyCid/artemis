use common::macos::LoginItemsData;
use std::path::PathBuf;
use std::{
    fs::File,
    io::{BufRead, BufReader},
};

#[test]
#[cfg(target_os = "macos")]
fn test_loginitems_parser() {
    use forensics::core::parse_toml_file;
    use glob::glob;
    use std::fs::read;

    let mut test_location = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    test_location.push("tests/test_data/macos/loginitems.toml");

    parse_toml_file(&test_location.display().to_string()).unwrap();
    let mut output_location = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    output_location.push("tmp/loginitems_collection/*");

    let results = glob(output_location.to_str().unwrap()).unwrap();
    for result in results {
        let value = &result.unwrap();
        if value.to_str().unwrap().contains("report_") {
            let bytes = read(value).unwrap();
            let text = String::from_utf8(bytes).unwrap();
            if text.contains("\"total_output_files\":0,") && text.contains("failed") {
                panic!("missing loginitems??");
            }
            continue;
        }
        let output_file = value.to_str().unwrap();

        if output_file.contains("/loginitems_") && output_file.ends_with(".jsonl") {
            validate_output(value);
        }
        if value.extension().unwrap() == "log" && !value.to_str().unwrap().contains("status_") {
            check_errors(value);
        }
    }
}

#[cfg(target_os = "macos")]
fn validate_output(output: &PathBuf) {
    // Output is in JSONL based on the TOML file above!
    let file = File::open(output).unwrap();
    let reader = BufReader::new(file);
    for (_, line) in reader.lines().enumerate() {
        let value = line.unwrap();
        println!("{value}");
        let info: LoginItemsData = serde_json::from_str(&value).unwrap();
        if info.path.is_empty() {
            panic!("no path?")
        }
        assert!(!info.path.is_empty());
    }
}

#[cfg(target_os = "macos")]
fn check_errors(output: &PathBuf) {
    let file = File::open(output).unwrap();
    let reader = BufReader::new(file);

    let mut count = 0;
    for (_, line) in reader.lines().enumerate() {
        let value = line.unwrap();
        if value.contains("not a bookmark got") {
            continue;
        }
        println!("{value}");
        count += 1;
    }

    if count != 0 {
        panic!("error count: {count}");
    }
}
