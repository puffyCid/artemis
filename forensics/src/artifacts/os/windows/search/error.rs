use std::fmt;

#[derive(Debug, PartialEq, Eq)]
pub(crate) enum SearchError {
    Systemdrive,
    ParseEse,
    MissingIndexes,
    Serialize,
    SqliteParse,
    BadSQL,
    NotSearchFile,
}

impl std::error::Error for SearchError {}

impl fmt::Display for SearchError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            SearchError::Systemdrive => write!(f, "Failed to get systemdrive"),
            SearchError::ParseEse => write!(f, "Failed to parse ESE db"),
            SearchError::MissingIndexes => write!(f, "Failed to find Indexes from Search"),
            SearchError::Serialize => write!(f, "Failed to serialize Search data"),
            SearchError::SqliteParse => write!(f, "Failed to read and parse SQLITE file"),
            SearchError::BadSQL => write!(f, "Failed to query SQLITE file"),
            SearchError::NotSearchFile => write!(f, "Not a recognizable search file"),
        }
    }
}
