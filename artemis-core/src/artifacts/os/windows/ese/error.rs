use std::fmt;

#[derive(Debug, PartialEq, Eq)]
pub(crate) enum EseError {
    ReadFile,
}

impl std::error::Error for EseError {}

impl fmt::Display for EseError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            EseError::ReadFile => write!(f, "Failed to read ESE db"),
        }
    }
}
