use std::fmt;

#[derive(Debug)]
pub(crate) enum LinuxArtifactError {
    Output,
    FilterOutput,
    Serialize,
    Format,
    File,
    Process,
    Journal,
}

impl std::error::Error for LinuxArtifactError {}

impl fmt::Display for LinuxArtifactError {
    fn fmt<'a>(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            LinuxArtifactError::Output => write!(f, "Failed to output data"),
            LinuxArtifactError::FilterOutput => write!(f, "Failed to filter linux data"),
            LinuxArtifactError::Serialize => write!(f, "Artemis failed serialize artifact data"),
            LinuxArtifactError::Format => write!(f, "Unknown formatter provided"),
            LinuxArtifactError::Process => write!(f, "Failed to parse Processes"),
            LinuxArtifactError::File => write!(f, "Failed to parse Files"),
            LinuxArtifactError::Journal => write!(f, "Failed to parse Journals"),
        }
    }
}
