use std::{fmt, io, path::PathBuf};

/// Result type used by output workflow
pub(crate) type OutputResult<T> = Result<T, OutputError>;

/// Errors produced by output workflow
#[derive(Debug)]
pub(crate) enum OutputError {
    /// Got unsupported output format
    UnsupportedFormat(String),
    /// Got unsupported destination value
    UnsupportedDestination(String),
    /// Got bad output config
    Config(String),
    /// Could not create an output context value
    Context(String),
    /// Could not write an artifact record
    Record(String),
    /// Could not encode artifact record into output format
    Encode(String),
    /// Could not write encoded artifact record to destination
    Sink(String),
    /// Issue writing Artemis reports
    Report(String),
    /// Issue finalizing the output
    Finalize(String),
    /// Could not initialize or write to log file
    Logger(String),
    /// Filesystem output errors
    Io {
        /// Optional path associated with IO
        path: Option<PathBuf>,
        /// Original IO error
        source: io::Error,
    },
    /// JSON serialization error
    Json(serde_json::Error),
    /// CSV serialization error
    Csv(csv::Error),
}

impl From<io::Error> for OutputError {
    fn from(source: io::Error) -> Self {
        Self::Io { path: None, source }
    }
}

impl From<serde_json::Error> for OutputError {
    fn from(source: serde_json::Error) -> Self {
        Self::Json(source)
    }
}

impl From<csv::Error> for OutputError {
    fn from(source: csv::Error) -> Self {
        Self::Csv(source)
    }
}

impl std::error::Error for OutputError {}

impl fmt::Display for OutputError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            OutputError::UnsupportedFormat(value) => write!(f, "Unsupported format: {value}"),
            OutputError::UnsupportedDestination(value) => {
                write!(f, "Unsupported destination: {value}")
            }
            Self::Config(value) => write!(f, "Output config error: {value}"),
            Self::Context(value) => write!(f, "Output context error: {value}"),
            Self::Record(value) => write!(f, "Record stream error: {value}"),
            Self::Encode(value) => write!(f, "Encode error: {value}"),
            Self::Sink(value) => write!(f, "Sink error: {value}"),
            Self::Report(value) => write!(f, "Report error: {value}"),
            Self::Finalize(value) => write!(f, "Finalize error: {value}"),
            Self::Logger(value) => write!(f, "Logger error: {value}"),
            Self::Io { path, source } => {
                if let Some(io_path) = path {
                    write!(f, "IO error at {}: {source}", io_path.display())
                } else {
                    write!(f, "IO error: {source}")
                }
            }
            Self::Json(value) => write!(f, "json error: {value}"),
            Self::Csv(value) => write!(f, "csv error: {value}"),
        }
    }
}

impl OutputError {
    pub(crate) fn io_path(path: impl Into<PathBuf>, source: io::Error) -> Self {
        Self::Io {
            path: Some(path.into()),
            source,
        }
    }
}
