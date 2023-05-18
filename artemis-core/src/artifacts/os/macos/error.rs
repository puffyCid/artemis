use std::fmt;

#[derive(Debug)]
pub(crate) enum MacArtifactError {
    LoginItem,
    Emond,
    FsEventsd,
    Launchd,
    Process,
    File,
    UnifiedLogs,
    Output,
    FilterOutput,
    BadToml,
    Serialize,
    Format,
    Cleanup,
}

impl std::error::Error for MacArtifactError {}

impl fmt::Display for MacArtifactError {
    fn fmt<'a>(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            MacArtifactError::LoginItem => write!(f, "Failed to parse Login Items"),
            MacArtifactError::Emond => write!(f, "Failed to parse Emond"),
            MacArtifactError::FsEventsd => write!(f, "Failed to parse FsEvents"),
            MacArtifactError::Launchd => write!(f, "Failed to parse Launchd"),
            MacArtifactError::Process => write!(f, "Failed to parse Processes"),
            MacArtifactError::File => write!(f, "Failed to parse Files"),
            MacArtifactError::UnifiedLogs => write!(f, "Failed to parse Unified Logs"),
            MacArtifactError::Output => write!(f, "Failed to output data"),
            MacArtifactError::FilterOutput => write!(f, "Failed to filter macos data"),
            MacArtifactError::BadToml => write!(f, "Artemis failed to parse TOML data"),
            MacArtifactError::Serialize => write!(f, "Artemis failed serialize artifact data"),
            MacArtifactError::Format => write!(f, "Unknown formatter provided"),
            MacArtifactError::Cleanup => write!(f, "Could not delete output data safely"),
        }
    }
}
