use std::fmt;

#[derive(Debug)]
pub(crate) enum ApplicationError {
    SafariHistory,
    SafariDownloads,
    FirefoxHistory,
    FirefoxDownloads,
    ChromiumHistory,
    ChromiumDownloads,
    Output,
    Serialize,
}

impl std::error::Error for ApplicationError {}

impl fmt::Display for ApplicationError {
    fn fmt<'a>(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            ApplicationError::SafariHistory => write!(f, "Failed to parse Safari History"),
            ApplicationError::SafariDownloads => {
                write!(f, "Failed to parse Safari Downloads")
            }
            ApplicationError::FirefoxHistory => {
                write!(f, "Failed to parse Firefox History")
            }
            ApplicationError::FirefoxDownloads => {
                write!(f, "Failed to parse Firefox Downloads")
            }
            ApplicationError::ChromiumHistory => {
                write!(f, "Failed to parse Chromium History")
            }
            ApplicationError::ChromiumDownloads => {
                write!(f, "Failed to parse Chromium Downloads")
            }
            ApplicationError::Output => write!(f, "Failed to output data"),
            ApplicationError::Serialize => {
                write!(f, "Artemis failed serialize artifact data")
            }
        }
    }
}
