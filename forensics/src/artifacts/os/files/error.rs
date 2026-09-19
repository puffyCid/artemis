use std::fmt;

#[derive(Debug)]
pub(crate) enum FileError {
    Filelisting,
}

impl std::error::Error for FileError {}

impl fmt::Display for FileError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            FileError::Filelisting => write!(f, "Could not get filelisting"),
        }
    }
}
