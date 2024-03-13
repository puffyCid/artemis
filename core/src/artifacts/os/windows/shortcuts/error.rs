use std::fmt;

#[derive(Debug)]
pub enum LnkError {
    Parse,
    BadHeader,
    ReadFile,
    NotLnkData,
    ReadDirectory,
}

impl std::error::Error for LnkError {}

impl fmt::Display for LnkError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            LnkError::Parse => write!(f, "Failed to parse shortcut data"),
            LnkError::BadHeader => write!(f, "Bad LNK header"),
            LnkError::ReadFile => write!(f, "Could not read lnk file"),
            LnkError::NotLnkData => write!(f, "Not shortcut data"),
            LnkError::ReadDirectory => write!(f, "Could not read directory"),
        }
    }
}
