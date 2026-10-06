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
