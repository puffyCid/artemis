use std::fmt;

#[derive(Debug, PartialEq, Eq)]
pub(crate) enum BitsError {
    ReadFile,
    Systemdrive,
    ParseEse,
    ParseLegacyBits,
    MissingJobs,
    MissingFiles,
    CarveBits,
}

impl std::error::Error for BitsError {}

impl fmt::Display for BitsError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            BitsError::ReadFile => write!(f, "Failed to read ESE db"),
            BitsError::Systemdrive => write!(f, "Failed to get systemdrive"),
            BitsError::ParseEse => write!(f, "Failed to parse ESE db"),
            BitsError::ParseLegacyBits => write!(f, "Failed to parse legacy BITS format"),
            BitsError::MissingJobs => write!(f, "No Jobs table in ESE db"),
            BitsError::MissingFiles => write!(f, "No Files table in ESE db"),
            BitsError::CarveBits => write!(f, "Could not carve BITS"),
        }
    }
}
