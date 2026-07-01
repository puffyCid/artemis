use crate::output::error::OutputError;
use std::fmt;

/// Errors produced by process listing
#[derive(Debug)]
pub enum ProcessError {
    /// Could not parse the process binary on disk
    ParseProcFile {
        /// Optional path associated with process executable
        path: String,
        /// Original IO error
        source: String,
    },
    /// Could not serialize process data
    Serialize(String),
    /// Could output process data
    Output(String),
}

impl std::error::Error for ProcessError {}

impl fmt::Display for ProcessError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            ProcessError::ParseProcFile { path, source } => {
                write!(f, "Failed to parse process binary at {path}: {source}",)
            }
            ProcessError::Serialize(message) => {
                write!(f, "Failed to serialize process listing: {message}")
            }
            ProcessError::Output(message) => {
                write!(f, "Failed to output process listing: {message}")
            }
        }
    }
}

impl ProcessError {
    /// Map failed serialization errors to `ProcessError`
    pub(crate) fn serialize_failed(error: OutputError) -> Self {
        Self::Serialize(error.to_string())
    }

    /// Map failed output errors to `ProcessError`
    pub(crate) fn output_failed(error: OutputError) -> Self {
        Self::Output(error.to_string())
    }
}
