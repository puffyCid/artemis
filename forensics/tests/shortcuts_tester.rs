use common::windows::ShortcutInfo;
use std::path::PathBuf;
use std::{
    fs::File,
    io::{BufRead, BufReader},
};

#[test]
#[cfg(target_os = "windows")]
fn test_shortcuts_parser() {
    use forensics::core::parse_toml_file;
    use glob::glob;
    use std::fs::read;

    let mut test_location = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    test_location.push("tests/test_data/windows/shortcuts.toml");

    parse_toml_file(&test_location.display().to_string()).unwrap();
    let mut output_location = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    output_location.push("tmp/shortcuts_collection/*");

    let results = glob(output_location.to_str().unwrap()).unwrap();
    for result in results {
        let value = &result.unwrap();
        if value.to_str().unwrap().contains("report_") {
            let bytes = read(value).unwrap();
            let text = String::from_utf8(bytes).unwrap();
            // Some systems may not have any Shortcut files in common locations
            if text.contains("\"total_output_files\":0,") && text.contains("failed") {
                panic!("missing Shortcuts??");
            }
            continue;
        }
        let output_file = value.to_str().unwrap();

        if output_file.contains("\\shortcuts_") && output_file.ends_with(".jsonl") {
            validate_output(value);
        }
        if value.extension().unwrap() == "log" && !value.to_str().unwrap().contains("status_") {
            check_errors(value);
        }
    }
}

#[cfg(target_os = "windows")]
fn validate_output(output: &PathBuf) {
    // Output is in JSONL based on the TOML file above!
    let file = File::open(output).unwrap();
    let reader = BufReader::new(file);
    for (_, line) in reader.lines().enumerate() {
        let value = line.unwrap();
        println!("{value}");
        let info: ShortcutInfo = serde_json::from_str(&value).unwrap();
        if info.created.is_empty() && info.path.is_empty() {
            println!("Missing timestamp and path?");
            panic!("{info:?}");
        }
        assert!(!info.path.is_empty())
    }
}

#[cfg(target_os = "windows")]
fn check_errors(output: &PathBuf) {
    let file = File::open(output).unwrap();
    let reader = BufReader::new(file);

    let mut count = 0;
    for (_, line) in reader.lines().enumerate() {
        let value = line.unwrap();

        // Common file encountered in Shortcut directories like Startup folder and Recent items
        if value.contains("desktop.ini") {
            continue;
        }
        println!("{value}");
        count += 1;
    }

    if count != 0 {
        panic!("error count: {count}");
    }
}
