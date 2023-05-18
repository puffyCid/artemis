use std::fmt;

#[derive(Debug)]
pub enum ChromiumHistoryError {
    PathError,
    SQLITEParseError,
    BadSQL,
}

impl std::error::Error for ChromiumHistoryError {}

impl fmt::Display for ChromiumHistoryError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            ChromiumHistoryError::PathError => write!(f, "Failed to get user history file"),
            ChromiumHistoryError::BadSQL => write!(f, "Could not compose sqlite query"),
            ChromiumHistoryError::SQLITEParseError => {
                write!(f, "Failed to parse SQLITE History file")
            }
        }
    }
}
