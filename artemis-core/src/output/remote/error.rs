use std::fmt;

#[derive(Debug)]
pub(crate) enum RemoteError {
    RemoteUrl,
    RemoteUpload,
    BadResponse,
    RemoteApiKey,
    MaxAttempts,
}

impl std::error::Error for RemoteError {}

impl fmt::Display for RemoteError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            RemoteError::RemoteUrl => write!(f, "Missing url from TOML"),
            RemoteError::RemoteUpload => write!(f, "Failed to upload data"),
            RemoteError::RemoteApiKey => write!(f, "Missing API key from TOML"),
            RemoteError::BadResponse => write!(f, "Received non-200 response from server"),
            RemoteError::MaxAttempts => write!(f, "Max attempts (15) reached for trying uploads"),
        }
    }
}
