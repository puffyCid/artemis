use crate::utils::error::UtilServerError;
use log::error;
use std::{
    fs::{create_dir_all, read},
    path::Path,
};

/// Check if path is a file
pub(crate) fn is_file(path: &str) -> bool {
    let file = Path::new(path);
    if file.is_file() {
        return true;
    }
    false
}

/// Read a file into memory
pub(crate) fn read_file(path: &str) -> Result<Vec<u8>, UtilServerError> {
    // Verify provided path is a file
    if !is_file(path) {
        return Err(UtilServerError::NotFile);
    }

    let read_result = read(path);
    match read_result {
        Ok(result) => Ok(result),
        Err(err) => {
            error!("[server] Failed to read file {path}: {err:?}");
            Err(UtilServerError::ReadFile)
        }
    }
}

/// Create a directory and all its parents
pub(crate) fn create_dirs(path: &str) -> Result<(), UtilServerError> {
    let result = create_dir_all(path);
    if result.is_err() {
        error!(
            "[server] Failed to directory {path}: {:?}",
            result.unwrap_err()
        );
        return Err(UtilServerError::CreateDirectory);
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::read_file;
    use crate::utils::filesystem::{create_dirs, is_file};
    use std::path::PathBuf;

    #[test]
    fn test_read_file() {
        let mut test_location = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
        test_location.push("tests/test_data/server.toml");
        let config_path = test_location.display().to_string();
        let results = read_file(&config_path).unwrap();

        assert!(!results.is_empty());
    }

    #[test]
    fn test_is_file() {
        let mut test_location = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
        test_location.push("tests/test_data/server.toml");
        let config_path = test_location.display().to_string();
        let results = is_file(&config_path);

        assert!(results);
    }

    #[test]
    fn test_create_dirs() {
        create_dirs(&"./tmp/atest").unwrap();
    }
}
