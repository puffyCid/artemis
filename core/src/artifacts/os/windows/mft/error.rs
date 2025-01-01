use std::fmt;

#[derive(Debug, PartialEq, Eq)]
pub(crate) enum MftError {
    ReadFile,
    Systemdrive,
}

impl std::error::Error for MftError {}

impl fmt::Display for MftError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            MftError::ReadFile => write!(f, "Failed to read file"),
            MftError::Systemdrive => write!(f, "Failed to determine systemdrive"),
        }
    }
}
