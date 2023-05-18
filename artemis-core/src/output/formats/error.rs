use std::fmt;

#[derive(Debug)]
pub enum FormatError {
    Serialize,
    Output,
    RemoveOldFile,
}

impl std::error::Error for FormatError {}

impl fmt::Display for FormatError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            FormatError::Serialize => write!(f, "Could not serialize data"),
            FormatError::Output => write!(f, "Could not output data"),
            FormatError::RemoveOldFile => write!(f, "Could not remove old file"),
        }
    }
}
