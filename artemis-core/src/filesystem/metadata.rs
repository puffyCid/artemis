use std::{
    fs::{metadata, Metadata},
    io::Error,
};

// Timestamps containing number of seconds since UNIX-EPOCH
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
        use std::os::macos::fs::MetadataExt;

        timestamps.accessed = meta.st_atime();
        timestamps.modified = meta.st_mtime();
        timestamps.changed = meta.st_ctime();
        timestamps.created = meta.st_birthtime();
    }

    Ok(timestamps)
}

/// Get the metadata associated with provided path
pub(crate) fn get_metadata(path: &str) -> Result<Metadata, Error> {
    metadata(path)
}

#[cfg(test)]
mod tests {
    use super::get_metadata;
    use crate::filesystem::metadata::get_timestamps;
    use std::path::PathBuf;

    #[test]
    fn test_get_metadata() {
        let mut test_location = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
        test_location.push("tests");

        let result = get_metadata(&test_location.display().to_string()).unwrap();
        assert_eq!(result.is_dir(), true);
    }

    #[test]
    fn test_get_timestamps() {
        let mut test_location = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
        test_location.push("tests");

        let result = get_timestamps(&test_location.display().to_string()).unwrap();
        assert!(result.created > 0);
        assert!(result.modified > 0);
        assert!(result.accessed > 0);
        #[cfg(target_os = "windows")]
        assert_eq!(result.changed, 0);
        #[cfg(target_family = "unix")]
        assert!(result.changed > 0);
    }
}
