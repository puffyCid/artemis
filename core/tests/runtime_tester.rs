#[test]
#[cfg(target_os = "macos")]
fn test_runtime_plist_files() {
    use core::core::parse_toml_file;
    use std::path::PathBuf;

    let mut test_location = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    test_location.push("tests/test_data/deno_scripts/plist_files.toml");

    let results = parse_toml_file(&test_location.display().to_string()).unwrap();
    assert_eq!(results, ())
}

#[test]
#[cfg(target_os = "windows")]
fn test_runtime_enhanced_shimcache_files() {
    use core::core::parse_toml_file;
    use std::path::PathBuf;

    let mut test_location = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    test_location.push("tests/test_data/deno_scripts/enhanced_shimcache.toml");

    let results = parse_toml_file(&test_location.display().to_string()).unwrap();
    assert_eq!(results, ())
}
