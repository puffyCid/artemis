#[test]
#[cfg(target_os = "windows")]
fn test_prefetch_parser() {
    use std::path::PathBuf;

    use forensics::core::parse_toml_file;
    let mut test_location = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    test_location.push("tests/test_data/windows/prefetch.toml");

    let results = parse_toml_file(&test_location.display().to_string()).unwrap();
    assert_eq!(results, ())
}
