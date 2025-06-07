use std::fmt;

#[derive(Debug)]
pub(crate) enum LoginItemError {
    Path,
    Plist,
}

impl std::error::Error for LoginItemError {}

impl fmt::Display for LoginItemError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            LoginItemError::Path => write!(f, "Failed to get provided path"),
            LoginItemError::Plist => write!(f, "No bookmark data"),
        }
    }
}
