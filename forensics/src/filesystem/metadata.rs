use crate::filesystem::error::FileSystemError;
use crate::utils::time::unixepoch_to_iso;
use log::error;
use serde::Serialize;
use std::fs::symlink_metadata;
use std::{fs::Metadata, io::Error};

// Timestamps containing number of seconds since UNIXEPOCH
pub(crate) struct StandardTimestamps {
    pub(crate) created: String,
    pub(crate) modified: String,
    pub(crate) accessed: String,
    pub(crate) changed: String,
}

/// Get standard timestamps (created, modified, accessed, changed (if supported))
pub(crate) fn get_timestamps(path: &str) -> Result<StandardTimestamps, Error> {
    let meta = get_metadata(path)?;
    let mut timestamps = StandardTimestamps {
        created: String::from("1970-01-01T00:00:00Z"),
        modified: String::from("1970-01-01T00:00:00Z"),
        accessed: String::from("1970-01-01T00:00:00Z"),
        changed: String::from("1970-01-01T00:00:00Z"),
    };

    #[cfg(target_os = "windows")]
    {
        use crate::utils::time::filetime_to_unixepoch;
        use std::os::windows::fs::MetadataExt;
        // Rust for Windows does not support getting Changed times :(
        timestamps.accessed = unixepoch_to_iso(&filetime_to_unixepoch(meta.last_access_time()));
        timestamps.modified = unixepoch_to_iso(&filetime_to_unixepoch(meta.last_write_time()));
        timestamps.created = unixepoch_to_iso(&filetime_to_unixepoch(meta.creation_time()));
    }

    #[cfg(any(target_os = "linux", target_os = "macos", target_os = "freebsd"))]
    {
        #[cfg(target_os = "linux")]
        use std::os::linux::fs::MetadataExt;
        #[cfg(target_os = "macos")]
        use std::os::macos::fs::MetadataExt;
        #[cfg(any(target_os = "freebsd", target_os = "netbsd"))]
        use std::os::unix::fs::MetadataExt;

        #[cfg(any(target_os = "linux", target_os = "macos"))]
        {
            timestamps.accessed = unixepoch_to_iso(&meta.st_atime());
            timestamps.modified = unixepoch_to_iso(&meta.st_mtime());
            timestamps.changed = unixepoch_to_iso(&meta.st_ctime());
        }

        #[cfg(any(target_os = "freebsd", target_os = "netbsd"))]
        {
            timestamps.accessed = unixepoch_to_iso(&meta.atime());
            timestamps.modified = unixepoch_to_iso(&meta.mtime());
            timestamps.changed = unixepoch_to_iso(&meta.ctime());
        }

        #[cfg(target_os = "linux")]
        {
            use std::time::SystemTime;

            let created = meta
                .created()
                .unwrap_or(SystemTime::UNIX_EPOCH)
                .duration_since(SystemTime::UNIX_EPOCH)
                .unwrap_or_default()
                .as_secs();
            timestamps.created = unixepoch_to_iso(&(created as i64));
        }
        #[cfg(target_os = "macos")]
        {
            timestamps.created = unixepoch_to_iso(&meta.st_birthtime());
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
            error!("[core] Could not glob {glob_pattern}: {err:?}");
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
        assert!(result.created != "");
        assert!(result.created != "");

        assert!(result.modified != "");
        assert!(result.accessed != "");
        #[cfg(target_os = "windows")]
        assert_eq!(result.changed, "1970-01-01T00:00:00Z");
        #[cfg(target_family = "unix")]
        assert!(result.changed != "");
    }
}
