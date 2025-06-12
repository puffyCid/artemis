use std::fmt;

#[derive(Debug)]
pub(crate) enum JournalError {
    SeekError,
    ReadError,
    ObjectHeader,
    ReaderError,
    JournalHeader,
    NotJournal,
}

impl std::error::Error for JournalError {}

impl fmt::Display for JournalError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            JournalError::SeekError => write!(f, "Failed to seek to journal offset"),
            JournalError::ReadError => write!(f, "Failed to read journal data"),
            JournalError::ObjectHeader => write!(f, "Failed to parse object header"),
            JournalError::JournalHeader => write!(f, "Failed to parse journal header"),
            JournalError::ReaderError => write!(f, "Could not create reader"),
            JournalError::NotJournal => write!(f, "Not a journal file"),
        }
    }
}
