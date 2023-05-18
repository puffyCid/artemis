use std::fmt;

#[derive(Debug)]
pub(crate) enum LaunchdError {
    UserPath,
    Files,
}

impl std::error::Error for LaunchdError {}

impl fmt::Display for LaunchdError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            LaunchdError::Files => write!(f, "Failed to get PLIST files"),
            LaunchdError::UserPath => write!(f, "Failed to get user paths"),
        }
    }
}
