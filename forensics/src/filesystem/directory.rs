use std::path::Path;
use tracing::warn;

/// Check if path is a directory
pub(crate) fn is_directory(path: &str) -> bool {
    Path::new(path).is_dir()
}

/// Get the parent directory of a provided path. From: "C:\\Users\\bob\\1.txt" will return "C:\\Users\\bob"
pub(crate) fn get_parent_directory(path: &str) -> String {
    if path.is_empty() {
        return String::new();
    }
    let entry_opt = if path.contains('/') {
        path.rsplit_once('/')
    } else {
        path.rsplit_once('\\')
    };

    if entry_opt.is_none() {
        warn!("Failed to get parent directory for path: {path}");
        return path.to_string();
    }

    let (directory, _) = entry_opt.unwrap_or_default();
    directory.to_string()
}

#[cfg(test)]
mod tests {
    use crate::filesystem::directory::{get_parent_directory, is_directory};
    use std::path::PathBuf;

    #[test]
    fn test_is_directory() {
        let mut test_location = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
        test_location.push("tests");
        let result = is_directory(&test_location.display().to_string());
        assert!(result);
    }

    #[test]
    #[cfg(target_family = "unix")]
    fn test_get_parent_directory() {
        let mut test_location = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
        test_location.push("tests/fsevents_tester.rs");
        let result = get_parent_directory(&test_location.display().to_string());
        assert!(result.ends_with("tests"));
    }

    #[test]
    #[cfg(target_os = "windows")]
    fn test_get_parent_directory() {
        let mut test_location = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
        test_location.push("tests\\fsevents_tester.rs");
        let result = get_parent_directory(&test_location.display().to_string());
        assert!(result.ends_with("tests"));
    }
}
