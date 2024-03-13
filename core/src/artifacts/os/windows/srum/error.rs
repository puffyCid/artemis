use std::fmt;

#[derive(Debug, PartialEq, Eq)]
pub(crate) enum SrumError {
    Systemdrive,
    ParseEse,
    MissingIndexes,
    Serialize,
    NoTable,
}

impl std::error::Error for SrumError {}

impl fmt::Display for SrumError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            SrumError::Systemdrive => write!(f, "Failed to get systemdrive"),
            SrumError::ParseEse => write!(f, "Failed to parse ESE db"),
            SrumError::MissingIndexes => write!(f, "Failed to find Indexes from SRUM"),
            SrumError::NoTable => write!(f, "Provided table(s) not found"),
            SrumError::Serialize => write!(f, "Failed to serialize SRUM data"),
        }
    }
}
