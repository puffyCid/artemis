#[test]
#[cfg(target_os = "macos")]
fn test_quick_artifacts() {
    use core::core::parse_toml_file;
    use std::path::PathBuf;

    let mut test_location = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    test_location.push("tests/test_data/macos/quick.toml");
    let result = parse_toml_file(&test_location.display().to_string()).unwrap();
    assert_eq!(result, ())
}
