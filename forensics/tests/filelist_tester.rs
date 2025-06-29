#[tokio::test]
#[cfg(target_os = "windows")]
async fn test_filelist_win_parser() {
    use forensics::core::parse_toml_file;
    use std::path::PathBuf;

    let mut test_location = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    test_location.push("tests/test_data/windows/files.toml");

    let results = parse_toml_file(&test_location.display().to_string())
        .await
        .unwrap();
    assert_eq!(results, ())
}

#[tokio::test]
#[cfg(target_os = "macos")]
async fn test_filelist_macos_parser() {
    use forensics::core::parse_toml_file;
    use std::path::PathBuf;

    let mut test_location = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    test_location.push("tests/test_data/macos/files.toml");

    let results = parse_toml_file(&test_location.display().to_string())
        .await
        .unwrap();
    assert_eq!(results, ())
}

#[tokio::test]
#[cfg(target_os = "linux")]
async fn test_filelist_linux_parser() {
    use forensics::core::parse_toml_file;
    use std::path::PathBuf;

    let mut test_location = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    test_location.push("tests/test_data/linux/files.toml");

    let results = parse_toml_file(&test_location.display().to_string())
        .await
        .unwrap();
    assert_eq!(results, ())
}
