use std::fmt;

#[derive(Debug)]
pub enum AmcacheError {
    GetRegistryData,
    DefaultDrive,
}

impl std::error::Error for AmcacheError {}

impl fmt::Display for AmcacheError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            AmcacheError::GetRegistryData => write!(f, "Failed to get Registry Amcache data"),
            AmcacheError::DefaultDrive => write!(f, "Failed to get default driver letter"),
        }
    }
}
