#[test]
#[cfg(target_os = "windows")]
fn test_amcache_parser() {
    use std::path::PathBuf;

    use artemis_core::core::parse_toml_file;

    let mut test_location = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    test_location.push("tests/test_data/windows/amcache.toml");

    let results = parse_toml_file(&test_location.display().to_string()).unwrap();
    assert_eq!(results, ())
}
