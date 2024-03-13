use core::core::parse_toml_file;
use std::path::PathBuf;

#[test]
#[cfg(target_os = "macos")]
fn test_chromium_parser_macos() {
    let mut test_location = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    test_location.push("tests/test_data/browser/chromium.toml");

    parse_toml_file(&test_location.display().to_string()).unwrap();
}

#[test]
#[cfg(target_os = "windows")]
fn test_chromium_parser_windows() {
    let mut test_location = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    test_location.push("tests/test_data/browser/chromiumwin.toml");

    parse_toml_file(&test_location.display().to_string()).unwrap();
}

#[test]
#[cfg(target_os = "linux")]
fn test_chromium_parser_windows() {
    let mut test_location = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    test_location.push("tests/test_data/browser/chromiumlinux.toml");

    parse_toml_file(&test_location.display().to_string()).unwrap();
}
