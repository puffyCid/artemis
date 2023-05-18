use std::path::PathBuf;

use artemis_core::core::parse_toml_file;

#[test]
#[cfg(target_os = "macos")]
fn test_firefox_parser_macos() {
    let mut test_location = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    test_location.push("tests/test_data/browser/firefox.toml");

    let results = parse_toml_file(&test_location.display().to_string()).unwrap();
    assert_eq!(results, ())
}
#[test]
#[cfg(target_os = "windows")]
fn test_firefox_parser_windows() {
    let mut test_location = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    test_location.push("tests/test_data/browser/firefoxwin.toml");

    let results = parse_toml_file(&test_location.display().to_string()).unwrap();
    assert_eq!(results, ())
}
