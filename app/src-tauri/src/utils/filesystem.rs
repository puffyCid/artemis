use log::warn;
use std::{fs::symlink_metadata, path::Path};

/// Check if path is a file
pub(crate) fn is_file(path: &str) -> bool {
    let file = Path::new(path);
    if file.is_file() {
        return true;
    }
    false
}

/// Get file size
pub(crate) fn size(path: &str) -> u64 {
    match symlink_metadata(path) {
        Ok(result) => result.len(),
        Err(err) => {
            warn!("[app] could not get {path} size: {err:?}");
            0
        }
    }
}
