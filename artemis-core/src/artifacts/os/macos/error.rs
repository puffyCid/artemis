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
    ExecPolicy,
    Output,
    FilterOutput,
    Serialize,
    SudoLog,
    Format,
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
            MacArtifactError::ExecPolicy => write!(f, "Failed to query ExecPolicy"),
            MacArtifactError::Output => write!(f, "Failed to output data"),
            MacArtifactError::FilterOutput => write!(f, "Failed to filter macos data"),
            MacArtifactError::Serialize => write!(f, "Artemis failed serialize artifact data"),
            MacArtifactError::Format => write!(f, "Unknown formatter provided"),
            MacArtifactError::SudoLog => write!(f, "Failed to parse sudo logs"),
        }
    }
}
