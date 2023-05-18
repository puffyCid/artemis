use std::fmt;

#[derive(Debug)]
pub enum ApplicationError {
    #[cfg(target_os = "macos")]
    SafariHistory,
    #[cfg(target_os = "macos")]
    SafariDownloads,
    FirefoxHistory,
    FirefoxDownloads,
    ChromiumHistory,
    ChromiumDownloads,
    Output,
    Serialize,
    Format,
    FilterOutput,
}

impl std::error::Error for ApplicationError {}

impl fmt::Display for ApplicationError {
    fn fmt<'a>(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            #[cfg(target_os = "macos")]
            ApplicationError::SafariHistory => write!(f, "Failed to parse Safari History"),
            #[cfg(target_os = "macos")]
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
            ApplicationError::FilterOutput => write!(f, "Failed to filter data"),
            ApplicationError::Serialize => {
                write!(f, "Artemis failed serialize artifact data")
            }
            ApplicationError::Format => write!(f, "Unknown formatter provided"),
        }
    }
}
