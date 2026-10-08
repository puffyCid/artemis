use crate::{accessor::error::AccessorError, output::error::OutputError};
use std::fmt;

/// Result type for disk image queries
pub(crate) type DiskResult<T> = Result<T, DiskError>;

/// Possible failures when reading disk images
/// or saving results
#[derive(Debug)]
pub(crate) enum DiskError {
    /// We failed to open, read, or parse the disk image
    Source(AccessorError),
    /// We failed to write output
    Output(OutputError),
}

impl From<AccessorError> for DiskError {
    fn from(value: AccessorError) -> Self {
        Self::Source(value)
    }
}

impl From<OutputError> for DiskError {
    fn from(source: OutputError) -> Self {
        Self::Output(source)
    }
}

impl std::error::Error for DiskError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            Self::Source(source) => Some(source),
            Self::Output(source) => Some(source),
        }
    }
}

impl fmt::Display for DiskError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Source(source) => write!(f, "disk source error: {source}"),
            Self::Output(source) => write!(f, "disk output error: {source}"),
        }
    }
}

/// Error to return the cli caller
#[derive(Debug)]
pub struct DiskCommandError {
    /// Error message
    message: String,
}

impl fmt::Display for DiskCommandError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.message)
    }
}

impl std::error::Error for DiskCommandError {}
impl From<DiskError> for DiskCommandError {
    fn from(err: DiskError) -> Self {
        Self {
            message: err.to_string(),
        }
    }
}
impl From<OutputError> for DiskCommandError {
    fn from(err: OutputError) -> Self {
        Self {
            message: err.to_string(),
        }
    }
}
