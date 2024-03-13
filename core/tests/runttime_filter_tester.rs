#[test]
#[cfg(target_os = "macos")]
fn test_runtime_filter_apps_info() {
    use core::core::parse_toml_file;
    use std::path::PathBuf;

    let mut test_location = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    test_location.push("tests/test_data/deno_scripts/filter_app_info.toml");

    let results = parse_toml_file(&test_location.display().to_string()).unwrap();
    assert_eq!(results, ())
}
