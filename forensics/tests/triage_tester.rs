use forensics::core::parse_toml_file;
use glob::glob;
use std::{fs::read, path::PathBuf};

#[test]
#[cfg(target_os = "windows")]
fn test_triage_collection_windows() {
    let mut test_location = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    test_location.push("tests/test_data/triage/artemis/windows_collect.toml");

    parse_toml_file(&test_location.display().to_string()).unwrap();

    let assert_glob = "./tmp/SOFTWARE Registry/*";
    let results = glob(&assert_glob).unwrap();
    let mut have_zip = false;
    for result in results {
        let value = result.unwrap();
        if value.extension().unwrap() == "zip" && value.metadata().unwrap().len() > 1000 {
            have_zip = true;
        }

        if value.to_str().unwrap().contains("report_") {
            let bytes = read(value).unwrap();
            let text = String::from_utf8(bytes).unwrap();
            if !text.contains("\"output_count\":1,") {
                panic!("missing SOFTWARE registry??");
            }
        }
    }

    assert!(have_zip);
}

#[test]
#[cfg(target_os = "windows")]
fn test_triage_collection_firefox_windows() {
    let mut test_location = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    test_location.push("tests/test_data/triage/artemis/edge.toml");

    parse_toml_file(&test_location.display().to_string()).unwrap();

    let assert_glob = "./tmp/Edge/*";
    let results = glob(&assert_glob).unwrap();
    let mut have_zip = false;
    for result in results {
        let value = result.unwrap();
        if value.extension().unwrap() == "zip" && value.metadata().unwrap().len() > 1000 {
            have_zip = true;
        }

        if value.to_str().unwrap().contains("report_") {
            let bytes = read(value).unwrap();
            let text = String::from_utf8(bytes).unwrap();
            if text.contains("\"total_output_files\":0,") {
                panic!("missing edge??");
            }
        }
    }

    assert!(have_zip);
}

#[test]
#[cfg(target_os = "linux")]
fn test_triage_collection_linux() {
    let mut test_location = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    test_location.push("tests/test_data/triage/artemis/collect.toml");

    parse_toml_file(&test_location.display().to_string()).unwrap();

    let assert_glob = "./tmp/triage_collection/*";
    let results = glob(&assert_glob).unwrap();
    let mut have_zip = false;
    for result in results {
        let value = result.unwrap();
        if value.extension().unwrap() == "zip" && value.metadata().unwrap().len() > 1000 {
            have_zip = true;
        }

        if value.to_str().unwrap().contains("report_") {
            let bytes = read(value).unwrap();
            let text = String::from_utf8(bytes).unwrap();
            if text.contains("\"total_output_files\":0,") {
                panic!("missing journals??");
            }
        }
    }

    assert!(have_zip);
}
