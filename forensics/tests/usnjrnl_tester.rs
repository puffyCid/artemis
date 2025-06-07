#[test]
#[cfg(target_os = "windows")]
#[ignore = "Parses the whole USNJrnl"]
fn test_usnjrnl_parser() {
    use forensics::core::parse_toml_file;
    use std::path::PathBuf;

    let mut test_location = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    test_location.push("tests/test_data/windows/usnjrnl.toml");

    let results = parse_toml_file(&test_location.display().to_string()).unwrap();
    assert_eq!(results, ())
}
