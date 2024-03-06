use std::fmt;

#[derive(Debug)]
pub(crate) enum LinuxArtifactError {
    Output,
    FilterOutput,
    Serialize,
    Format,
    Journal,
    SudoLog,
}

impl std::error::Error for LinuxArtifactError {}

impl fmt::Display for LinuxArtifactError {
    fn fmt<'a>(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            LinuxArtifactError::Output => write!(f, "Failed to output data"),
            LinuxArtifactError::FilterOutput => write!(f, "Failed to filter linux data"),
            LinuxArtifactError::Serialize => write!(f, "Artemis failed serialize artifact data"),
            LinuxArtifactError::Format => write!(f, "Unknown formatter provided"),
            LinuxArtifactError::Journal => write!(f, "Failed to parse Journals"),
            LinuxArtifactError::SudoLog => write!(f, "Failed to parse sudo logs"),
        }
    }
}
