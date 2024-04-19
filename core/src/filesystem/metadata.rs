use crate::filesystem::error::FileSystemError;
use log::error;
use serde::Serialize;
use std::fs::symlink_metadata;
use std::{fs::Metadata, io::Error};

// Timestamps containing number of seconds since UNIXEPOCH
pub(crate) struct StandardTimestamps {
    pub(crate) created: i64,
    pub(crate) modified: i64,
    pub(crate) accessed: i64,
    pub(crate) changed: i64,
}

/// Get standard timestamps (created, modified, accessed, changed (if supported))
pub(crate) fn get_timestamps(path: &str) -> Result<StandardTimestamps, Error> {
    let meta = get_metadata(path)?;
    let mut timestamps = StandardTimestamps {
        created: 0,
        modified: 0,
        accessed: 0,
        changed: 0,
    };

    #[cfg(target_os = "windows")]
    {
        use crate::utils::time::filetime_to_unixepoch;
        use std::os::windows::fs::MetadataExt;
        // Rust for Windows does not support getting Changed times :(
        timestamps.accessed = filetime_to_unixepoch(&meta.last_access_time());
        timestamps.modified = filetime_to_unixepoch(&meta.last_write_time());
        timestamps.created = filetime_to_unixepoch(&meta.creation_time());
    }

    #[cfg(target_family = "unix")]
    {
        #[cfg(target_os = "linux")]
        use std::os::linux::fs::MetadataExt;
        #[cfg(target_os = "macos")]
        use std::os::macos::fs::MetadataExt;

        timestamps.accessed = meta.st_atime();
        timestamps.modified = meta.st_mtime();
        timestamps.changed = meta.st_ctime();
        #[cfg(target_os = "macos")]
        {
            timestamps.created = meta.st_birthtime();
        }
    }

    Ok(timestamps)
}

/// Get the metadata associated with provided path
pub(crate) fn get_metadata(path: &str) -> Result<Metadata, Error> {
    symlink_metadata(path)
}

#[derive(Debug, Serialize)]
pub(crate) struct GlobInfo {
    pub(crate) full_path: String,
    pub(crate) filename: String,
    pub(crate) is_file: bool,
    pub(crate) is_directory: bool,
    pub(crate) is_symlink: bool,
}

/// Execute a provided Glob pattern (Ex: /files/*) and return results
pub(crate) fn glob_paths(glob_pattern: &str) -> Result<Vec<GlobInfo>, FileSystemError> {
    let mut info = Vec::new();
    let glob_results = glob::glob(glob_pattern);
    let paths = match glob_results {
        Ok(result) => result,
        Err(err) => {
            error!("[artemis-core] Could not glob {glob_pattern}: {err:?}");
            return Err(FileSystemError::BadGlob);
        }
    };

    for entry in paths.flatten() {
        let glob_info = GlobInfo {
            full_path: entry.to_str().unwrap_or_default().to_string(),
            filename: entry
                .file_name()
                .unwrap_or_default()
                .to_str()
                .unwrap_or_default()
                .to_string(),
            is_directory: entry.is_dir(),
            is_file: entry.is_file(),
            is_symlink: entry.is_symlink(),
        };
        info.push(glob_info);
    }

    Ok(info)
}

#[cfg(test)]
mod tests {
    use super::get_metadata;
    use crate::filesystem::metadata::{get_timestamps, glob_paths};
    use std::path::PathBuf;

    #[test]
    fn test_get_metadata() {
        let mut test_location = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
        test_location.push("tests");

        let result = get_metadata(&test_location.display().to_string()).unwrap();
        assert!(result.is_dir());
    }

    #[test]
    fn test_glob_paths() {
        let mut test_location = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
        test_location.push("tests");

        let result = glob_paths(&format!("{}/*", test_location.to_str().unwrap())).unwrap();
        assert!(result.len() > 10);
    }

    #[test]
    fn test_get_timestamps() {
        let mut test_location = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
        test_location.push("tests");

        let result = get_timestamps(&test_location.display().to_string()).unwrap();
        #[cfg(target_os = "windows")]
        assert!(result.created > 0);
        #[cfg(target_os = "macos")]
        assert!(result.created > 0);

        assert!(result.modified > 0);
        assert!(result.accessed > 0);
        #[cfg(target_os = "windows")]
        assert_eq!(result.changed, 0);
        #[cfg(target_family = "unix")]
        assert!(result.changed > 0);
    }
}
