use std::fmt;

#[derive(Debug)]
pub enum UnixArtifactError {
    Zsh,
    Bash,
    Python,
    Cron,
    Serialize,
    Output,
    FilterOutput,
    Format,
    SudoLog,
}

impl std::error::Error for UnixArtifactError {}

impl fmt::Display for UnixArtifactError {
    fn fmt<'a>(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            UnixArtifactError::Zsh => write!(f, "Failed to parse zsh history"),
            UnixArtifactError::Bash => write!(f, "Failed to parse bash history"),
            UnixArtifactError::Python => write!(f, "Failed to parse python history"),
            UnixArtifactError::Cron => write!(f, "Failed to parse cron data"),
            UnixArtifactError::Serialize => write!(f, "Failed to serialize unix data"),
            UnixArtifactError::Output => write!(f, "Failed to output unix data"),
            UnixArtifactError::FilterOutput => write!(f, "Failed to filter unix data"),
            UnixArtifactError::Format => write!(f, "Unknown format provided"),
            UnixArtifactError::SudoLog => write!(f, "Failed to parse sudo logs"),
        }
    }
}
