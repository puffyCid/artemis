use std::fmt;

#[derive(Debug, PartialEq, Eq)]
pub(crate) enum MftError {
    ReadFile,
    Systemdrive,
    Serialize,
    OutputData,
    RawSize,
    EntrySize,
}

impl std::error::Error for MftError {}

impl fmt::Display for MftError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            MftError::ReadFile => write!(f, "Failed to read file"),
            MftError::Systemdrive => write!(f, "Failed to determine systemdrive"),
            MftError::Serialize => write!(f, "Failed to serialize mft entries"),
            MftError::OutputData => write!(f, "Failed to output mft entries"),
            MftError::RawSize => write!(f, "Failed to determine size of mft file"),
            MftError::EntrySize => write!(f, "Failed to determine file entry size"),
        }
    }
}
