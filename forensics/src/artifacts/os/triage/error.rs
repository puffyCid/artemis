use std::fmt;

#[derive(Debug)]
pub(crate) enum TriageError {
    Regex,
    ReadFile,
    StartZip,
    NoReader,
    Output,
    WriteReport,
}

impl std::error::Error for TriageError {}

impl fmt::Display for TriageError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            TriageError::Regex => write!(f, "Failed to compile file regex"),
            TriageError::ReadFile => write!(f, "Could not read file"),
            TriageError::StartZip => write!(f, "Could not start writing to zip"),
            TriageError::NoReader => write!(f, "No reader provided"),
            TriageError::Output => write!(f, "Could not write output"),
            TriageError::WriteReport => write!(f, "Could not write report to zip"),
        }
    }
}
