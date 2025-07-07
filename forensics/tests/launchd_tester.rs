#[tokio::test]
#[cfg(target_os = "macos")]
async fn test_launchd_parser() {
    use forensics::core::parse_toml_file;
    use std::path::PathBuf;

    let mut test_location = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    test_location.push("tests/test_data/macos/launchd.toml");

    let results = parse_toml_file(&test_location.display().to_string())
        .await
        .unwrap();
    assert_eq!(results, ())
}
