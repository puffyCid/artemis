use std::path::Path;

/// Check if path is a file
pub(crate) fn is_file(path: &str) -> bool {
    let file = Path::new(path);
    if file.is_file() {
        return true;
    }
    false
}
