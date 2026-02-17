use std::fmt;

#[derive(Debug)]
pub(crate) enum TriageError {
    Regex,
    Decode,
    Toml,
    ReadFile,
    StartZip,
    NoReader,
    Output,
}

impl std::error::Error for TriageError {}

impl fmt::Display for TriageError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            TriageError::Regex => write!(f, "Failed to compile file regex"),
            TriageError::Decode => write!(f, "Failed to base64 decode triage"),
            TriageError::Toml => write!(f, "Failed to parse TOML triage"),
            TriageError::ReadFile => write!(f, "Could not read file"),
            TriageError::StartZip => write!(f, "Could not start writing to zip"),
            TriageError::NoReader => write!(f, "No reader provided"),
            TriageError::Output => write!(f, "Could not write output"),
        }
    }
}
