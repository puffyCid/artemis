use std::fmt;

#[derive(Debug)]
pub enum DaemonError {
    ReadFile,
    MakeDirectory,
    WriteFile,
}

impl std::error::Error for DaemonError {}

impl fmt::Display for DaemonError {
    fn fmt<'a>(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            DaemonError::ReadFile => write!(f, "Failed to read file"),
            DaemonError::MakeDirectory => write!(f, "Failed to create directory"),
            DaemonError::WriteFile => write!(f, "Failed to create file"),
        }
    }
}
