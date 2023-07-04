#[test]
#[cfg(target_os = "linux")]
fn test_journal_parser() {
    use artemis_core::core::parse_toml_file;
    use std::path::PathBuf;

    let mut test_location = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    test_location.push("tests/test_data/linux/journal.toml");

    let results = parse_toml_file(&test_location.display().to_string()).unwrap();
    assert_eq!(results, ())
}
