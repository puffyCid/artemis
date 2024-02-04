use std::fmt;

#[derive(Debug, PartialEq, Eq)]
pub(crate) enum WmiError {
    ReadObjects,
    ReadMaps,
    GlobMaps,
    ParseMap,
    ReadIndex,
    ParseIndex,
    DriveLetter,
}

impl std::error::Error for WmiError {}

impl fmt::Display for WmiError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            WmiError::ReadObjects => write!(f, "Failed to read objects data"),
            WmiError::ReadMaps => write!(f, "Failed to read mapping data"),
            WmiError::ParseMap => write!(f, "Failed to parse mapping data"),
            WmiError::GlobMaps => write!(f, "Failed to glob mapping data"),
            WmiError::ReadIndex => write!(f, "Failed to read index data"),
            WmiError::ParseIndex => write!(f, "Failed to parse index data"),
            WmiError::DriveLetter => write!(f, "Could not get drive letter"),
        }
    }
}
