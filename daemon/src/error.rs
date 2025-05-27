use std::fmt;

#[derive(Debug)]
pub enum DaemonError {
    ReadFile,
}

impl std::error::Error for DaemonError {}

impl fmt::Display for DaemonError {
    fn fmt<'a>(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            DaemonError::ReadFile => write!(f, "Failed to read file"),
        }
    }
}
