use std::fmt;

#[derive(Debug)]
pub(crate) enum FirefoxHistoryError {
    PathError,
    SqliteParse,
    BadSQL,
}

impl std::error::Error for FirefoxHistoryError {}

impl fmt::Display for FirefoxHistoryError {
    fn fmt<'a>(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            FirefoxHistoryError::PathError => write!(f, "Failed to get user history file"),
            FirefoxHistoryError::BadSQL => write!(f, "Could not compose sqlite query"),
            FirefoxHistoryError::SqliteParse => {
                write!(f, "Failed to parse SQLITE History file")
            }
        }
    }
}
