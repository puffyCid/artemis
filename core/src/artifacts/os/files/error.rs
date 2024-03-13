use std::fmt;

#[derive(Debug)]
pub(crate) enum FileError {
    Regex,
    ParseFile,
    Filelisting,
}

impl std::error::Error for FileError {}

impl fmt::Display for FileError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            FileError::Regex => write!(f, "Failed to compile file regex"),
            FileError::ParseFile => write!(f, "Failed to get parse executable file"),
            FileError::Filelisting => write!(f, "Could not get filelisting"),
        }
    }
}
