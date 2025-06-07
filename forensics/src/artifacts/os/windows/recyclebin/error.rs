use std::fmt;

#[derive(Debug, PartialEq, Eq)]
pub(crate) enum RecycleBinError {
    ReadFile,
    Systemdrive,
    ParseFile,
}

impl std::error::Error for RecycleBinError {}

impl fmt::Display for RecycleBinError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            RecycleBinError::ReadFile => write!(f, "Failed to read Recycle Bin file"),
            RecycleBinError::Systemdrive => write!(f, "Failed to get systemdrive"),
            RecycleBinError::ParseFile => write!(f, "Failed to parse Recycle Bin file"),
        }
    }
}
