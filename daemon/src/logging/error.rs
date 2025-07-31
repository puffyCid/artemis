use std::fmt;

#[derive(Debug)]
pub(crate) enum LoggingError {
    FailedUpload,
    OpenFile,
    UploadNotOk,
    UploadBadResponse,
    ClearLog,
}

impl std::error::Error for LoggingError {}

impl fmt::Display for LoggingError {
    fn fmt<'a>(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            LoggingError::FailedUpload => write!(f, "Failed to upload logs"),
            LoggingError::OpenFile => write!(f, "Failed to open log file"),
            LoggingError::UploadNotOk => write!(f, "Server did not like log file upload"),
            LoggingError::UploadBadResponse => write!(f, "Server provided bad response"),
            LoggingError::ClearLog => write!(f, "Could not clear the log file"),
        }
    }
}
