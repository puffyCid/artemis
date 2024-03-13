#[test]
#[cfg(target_os = "windows")]
fn test_services_parser() {
    use std::path::PathBuf;

    use core::core::parse_toml_file;
    let mut test_location = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    test_location.push("tests/test_data/windows/services.toml");

    let results = parse_toml_file(&test_location.display().to_string()).unwrap();
    assert_eq!(results, ())
}
