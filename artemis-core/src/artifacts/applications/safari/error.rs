use std::fmt;

#[derive(Debug)]
pub enum SafariError {
    SqliteParse,
    BadSQL,
    Plist,
    Bookmark,
    PathError,
}

impl std::error::Error for SafariError {}

impl fmt::Display for SafariError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            SafariError::PathError => write!(f, "Failed to get user history file"),
            SafariError::BadSQL => write!(f, "Could not compose sqlite query"),
            SafariError::Plist => write!(f, "Could not parse PLIST file"),
            SafariError::Bookmark => write!(f, "Could not parse PLIST bookmark data"),
            SafariError::SqliteParse => write!(f, "Failed to parse sqlite History file"),
        }
    }
}
