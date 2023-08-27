use std::fmt;

#[derive(Debug)]
pub(crate) enum FreeBSDArtifactError {
    Output,
    FilterOutput,
    BadToml,
    Serialize,
    Format,
    File,
    Process,
}

impl std::error::Error for FreeBSDArtifactError {}

impl fmt::Display for FreeBSDArtifactError {
    fn fmt<'a>(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            FreeBSDArtifactError::Output => write!(f, "Failed to output data"),
            FreeBSDArtifactError::FilterOutput => write!(f, "Failed to filter linux data"),
            FreeBSDArtifactError::BadToml => write!(f, "Artemis failed to parse TOML data"),
            FreeBSDArtifactError::Serialize => write!(f, "Artemis failed serialize artifact data"),
            FreeBSDArtifactError::Format => write!(f, "Unknown formatter provided"),
            FreeBSDArtifactError::Process => write!(f, "Failed to parse Processes"),
            FreeBSDArtifactError::File => write!(f, "Failed to parse Files"),
        }
    }
}
