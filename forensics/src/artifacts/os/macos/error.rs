use std::fmt;

#[derive(Debug)]
pub(crate) enum MacArtifactError {
    FsEventsd,
    UnifiedLogs,
    Output,
    Serialize,
    SudoLog,
    Spotlight,
}

impl std::error::Error for MacArtifactError {}

impl fmt::Display for MacArtifactError {
    fn fmt<'a>(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            MacArtifactError::FsEventsd => write!(f, "Failed to parse FsEvents"),
            MacArtifactError::UnifiedLogs => write!(f, "Failed to parse Unified Logs"),
            MacArtifactError::Output => write!(f, "Failed to output data"),
            MacArtifactError::Serialize => write!(f, "Artemis failed serialize artifact data"),
            MacArtifactError::SudoLog => write!(f, "Failed to parse sudo logs"),
            MacArtifactError::Spotlight => write!(f, "Failed to parse spotlight"),
        }
    }
}
