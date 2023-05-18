use std::fmt;

#[derive(Debug, PartialEq, Eq)]
pub enum NTFSError {
    Parser,
    RootDir,
    IndexDir,
    FilenameInfo,
    FileData,
    AttributeValue,
    Dos,
    BadStart,
    Regex,
    NoAttribute,
}

impl std::error::Error for NTFSError {}

impl fmt::Display for NTFSError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            NTFSError::Parser => write!(f, "Failed to setup NTFS parser"),
            NTFSError::RootDir => write!(f, "Failed to get NTFS root directory"),
            NTFSError::IndexDir => write!(f, "Failed to get directory index"),
            NTFSError::FilenameInfo => write!(f, "Failed to get filename info"),
            NTFSError::FileData => write!(f, "Failed to get filedata"),
            NTFSError::AttributeValue => write!(f, "Failed to get attribute value data"),
            NTFSError::Dos => write!(f, "Not parsing DoS entries"),
            NTFSError::BadStart => write!(
                f,
                "Improper start path, need full start path. Ex: C:\\ or C:\\Users\\"
            ),
            NTFSError::Regex => write!(f, "Invalid regex provided"),
            NTFSError::NoAttribute => write!(f, "No attribute for entry"),
        }
    }
}
