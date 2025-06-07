use std::fmt;

#[derive(Debug, PartialEq, Eq)]
pub(crate) enum ServicesError {
    RegistryFiles,
    ServicesData,
    DriveLetter,
    Base64Decode,
}

impl std::error::Error for ServicesError {}

impl fmt::Display for ServicesError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            ServicesError::RegistryFiles => write!(f, "Could not get Registry file"),
            ServicesError::ServicesData => write!(f, "Could not get Services data"),
            ServicesError::DriveLetter => write!(f, "Failed to get systemdrive letter"),
            ServicesError::Base64Decode => write!(f, "Failed to base64 service data"),
        }
    }
}
