use std::fmt;

#[derive(Debug, PartialEq, Eq)]
pub(crate) enum SrumError {
    Systemdrive,
    ParseEse,
    Serialize,
    NoTable,
}

impl std::error::Error for SrumError {}

impl fmt::Display for SrumError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            SrumError::Systemdrive => write!(f, "Failed to get systemdrive"),
            SrumError::ParseEse => write!(f, "Failed to parse ESE db"),
            SrumError::NoTable => write!(f, "Provided table(s) not found"),
            SrumError::Serialize => write!(f, "Failed to serialize SRUM data"),
        }
    }
}
