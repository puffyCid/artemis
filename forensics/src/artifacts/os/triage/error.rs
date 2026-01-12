use std::fmt;

#[derive(Debug)]
pub enum TriageError {
    ReadFile,
    CreateDirectories,
    CopyFile,
}

impl std::error::Error for TriageError {}

impl fmt::Display for TriageError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            TriageError::ReadFile => write!(f, "Failed to read target file"),
            TriageError::CreateDirectories => write!(f, "Failed to recreate directories"),
            TriageError::CopyFile => write!(f, "Failed to copy target"),
        }
    }
}
