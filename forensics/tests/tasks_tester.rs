use common::windows::TaskInfo;
use std::path::PathBuf;
use std::{
    fs::File,
    io::{BufRead, BufReader},
};

#[test]
#[cfg(target_os = "windows")]
fn test_tasks_parser() {
    use forensics::core::parse_toml_file;
    use glob::glob;
    use std::fs::read;

    let mut test_location = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    test_location.push("tests/test_data/windows/tasks.toml");

    parse_toml_file(&test_location.display().to_string()).unwrap();
    let mut output_location = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    output_location.push("tmp/tasks_collection/*");

    let results = glob(output_location.to_str().unwrap()).unwrap();
    for result in results {
        let value = &result.unwrap();
        if value.to_str().unwrap().contains("report_") {
            let bytes = read(value).unwrap();
            let text = String::from_utf8(bytes).unwrap();
            if text.contains("\"total_output_files\":0,") {
                panic!("missing Tasks??");
            }
        }
        let output_file = value.to_str().unwrap();

        if output_file.contains("\\tasks_") && output_file.ends_with(".jsonl") {
            validate_output(value);
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
        let info: TaskInfo = serde_json::from_str(&value).unwrap();
        assert!(!info.name.is_empty());
        if !info.action.contains("VSIXConfigurationUpdater") && !info.path.contains("S-1-5-21-") {
            assert!(!info.registry_tree_path.is_empty());
            assert!(!info.id.is_empty());
        }
        assert!(!info.action.ends_with(" "));
        assert!(!info.action.is_empty());
        assert_ne!(info.action_count, 0);

        assert!(info.path.starts_with("\\"));
        assert_ne!(info.created, "1970-01-01T00:00:00Z");
        if info.name.contains("OneDrive") {
            assert!(info.action.contains("\\OneDrive"))
        }
        if info.name.contains("OneDrive Reporting") {
            assert!(
                info.action
                    .contains("OneDriveStandaloneUpdater.exe /reporting")
            )
        }
    }
}

#[test]
fn read_sample_output() {
    let mut test_location = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    test_location.push("tests/test_data/samples/windows/tasks.jsonl");

    let file = File::open(&test_location).unwrap();
    let reader = BufReader::new(file);
    for (_, line) in reader.lines().enumerate() {
        let value = line.unwrap();
        let info: TaskInfo = serde_json::from_str(&value).unwrap();
        if !info.action.contains("VSIXConfigurationUpdater") && !info.path.contains("S-1-5-21-") {
            assert!(!info.registry_tree_path.is_empty());
            assert!(!info.id.is_empty());
        }
        assert!(!info.action.ends_with(" "));
        assert!(!info.action.is_empty());
        assert_ne!(info.action_count, 0);

        assert!(info.path.starts_with("\\"));
        assert_ne!(info.created, "1970-01-01T00:00:00Z");
        if info.name.contains("OneDrive") {
            assert!(info.action.contains("\\OneDrive"))
        }
        if info.name.contains("OneDrive Reporting") {
            assert!(
                info.action
                    .contains("OneDriveStandaloneUpdater.exe /reporting")
            )
        }
    }
}
