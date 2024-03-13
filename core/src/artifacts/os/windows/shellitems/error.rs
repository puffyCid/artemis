use std::fmt;

#[derive(Debug)]
pub enum ShellItemError {
    ParseItem,
    Decode,
}

impl std::error::Error for ShellItemError {}

impl fmt::Display for ShellItemError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            ShellItemError::ParseItem => write!(f, "Failed to parse ShellItem"),
            ShellItemError::Decode => write!(f, "Failed to base64 decode data"),
        }
    }
}
