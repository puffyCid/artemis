use std::fmt;

#[derive(Debug)]
pub(crate) enum JournalError {
    SeekError,
    ReadError,
    ObjectHeader,
    ObjectTypeArray,
}

impl std::error::Error for JournalError {}

impl fmt::Display for JournalError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            JournalError::SeekError => write!(f, "Failed to seek to journal offset"),
            JournalError::ReadError => write!(f, "Failed to read journal data"),
            JournalError::ObjectHeader => write!(f, "Failed to parse object header"),
            JournalError::ObjectTypeArray => write!(f, "Did not get object type array"),
        }
    }
}
