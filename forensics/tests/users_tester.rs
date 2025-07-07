#[tokio::test]
#[cfg(target_os = "windows")]
async fn test_users_parser() {
    use forensics::core::parse_toml_file;
    use std::path::PathBuf;

    let mut test_location = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    test_location.push("tests/test_data/windows/users.toml");

    let results = parse_toml_file(&test_location.display().to_string())
        .await
        .unwrap();
    assert_eq!(results, ())
}

#[tokio::test]
#[cfg(target_os = "macos")]
async fn test_users_parser() {
    use forensics::core::parse_toml_file;
    use std::path::PathBuf;

    let mut test_location = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    test_location.push("tests/test_data/macos/users.toml");

    let results = parse_toml_file(&test_location.display().to_string())
        .await
        .unwrap();
    assert_eq!(results, ())
}

#[tokio::test]
#[cfg(target_os = "macos")]
async fn test_groups_parser() {
    use forensics::core::parse_toml_file;
    use std::path::PathBuf;

    let mut test_location = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    test_location.push("tests/test_data/macos/groups.toml");

    let results = parse_toml_file(&test_location.display().to_string())
        .await
        .unwrap();
    assert_eq!(results, ())
}
