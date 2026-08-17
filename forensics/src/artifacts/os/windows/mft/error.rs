use std::fmt;

#[derive(Debug, PartialEq, Eq)]
pub(crate) enum MftError {
    ReadFile,
    Serialize,
    OutputData,
    EntrySize,
}

impl std::error::Error for MftError {}

impl fmt::Display for MftError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            MftError::ReadFile => write!(f, "Failed to read file"),
            MftError::Serialize => write!(f, "Failed to serialize mft entries"),
            MftError::OutputData => write!(f, "Failed to output mft entries"),
            MftError::EntrySize => write!(f, "Failed to determine file entry size"),
        }
    }
}
