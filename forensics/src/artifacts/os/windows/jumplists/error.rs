use std::fmt;

#[derive(Debug, PartialEq, Eq)]
pub(crate) enum JumplistError {
    ReadFile,
    Systemdrive,
    ParseJumplist,
    NotJumplist,
}

impl std::error::Error for JumplistError {}

impl fmt::Display for JumplistError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            JumplistError::ReadFile => write!(f, "Failed to read Jumplist file"),
            JumplistError::Systemdrive => write!(f, "Failed to get systemdrive"),
            JumplistError::ParseJumplist => write!(f, "Failed to parse Jumplist file"),
            JumplistError::NotJumplist => write!(f, "Not a jumplist file"),
        }
    }
}
