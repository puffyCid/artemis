#[test]
fn test_mft_parser() {
    use core::core::parse_toml_file;
    use std::path::PathBuf;

    let mut test_location = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    test_location.push("tests/test_data/windows/mft.toml");

    let results = parse_toml_file(&test_location.display().to_string()).unwrap();
    assert_eq!(results, ())
}
