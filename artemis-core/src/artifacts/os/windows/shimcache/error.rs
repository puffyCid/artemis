use std::fmt;

#[derive(Debug, PartialEq, Eq)]
pub(crate) enum ShimcacheError {
    RegistryFile,
    Base64,
    UnknownOS,
    Parser,
    Drive,
}

impl std::error::Error for ShimcacheError {}

impl fmt::Display for ShimcacheError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            ShimcacheError::RegistryFile => write!(f, "Could not parse SYSTEM registry file"),
            ShimcacheError::Base64 => write!(f, "Could not base64 decode data"),
            ShimcacheError::UnknownOS => write!(f, "Unknown Windows OS"),
            ShimcacheError::Parser => write!(f, "Could not parse shimcache data"),
            ShimcacheError::Drive => write!(f, "Could not determine systemdrive letter"),
        }
    }
}
