use std::fmt;

#[derive(Debug)]
pub(crate) enum FileSystemError {
    ReadDirectory,
    NotDirectory,
    OpenFile,
    ReadFile,
    NotFile,
    BadGlob,
    LargeFile,
}

impl std::error::Error for FileSystemError {}

impl fmt::Display for FileSystemError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            FileSystemError::ReadDirectory => write!(f, "Could not read directory path"),
            FileSystemError::NotDirectory => write!(f, "Not a directory"),
            FileSystemError::BadGlob => write!(f, "Could not glob"),
            FileSystemError::OpenFile => write!(f, "Could not open file"),
            FileSystemError::ReadFile => write!(f, "Could not read file"),
            FileSystemError::NotFile => write!(f, "Not a file"),
            FileSystemError::LargeFile => write!(f, "File larger than 2GB"),
        }
    }
}
