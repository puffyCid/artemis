use crate::output::error::OutputError;
use std::fmt;

#[derive(Debug)]
pub(crate) enum SystemInfoError {
    /// Could not serialize system info data
    Serialize(String),
    /// Could output system info data
    Output(String),
}

impl std::error::Error for SystemInfoError {}

impl fmt::Display for SystemInfoError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            SystemInfoError::Serialize(message) => {
                write!(f, "Failed to serialize system info: {message}")
            }
            SystemInfoError::Output(message) => {
                write!(f, "Failed to output system info: {message}")
            }
        }
    }
}

impl SystemInfoError {
    /// Map failed serialization errors to `SystemInfoError`
    pub(crate) fn serialize_failed(error: OutputError) -> Self {
        Self::Serialize(error.to_string())
    }

    /// Map failed output errors to `SystemInfoError`
    pub(crate) fn output_failed(error: OutputError) -> Self {
        Self::Output(error.to_string())
    }
}
