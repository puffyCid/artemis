use std::fmt;

#[derive(Debug)]
pub enum ShellbagError {
    GetRegistryData,
    DefaultDrive,
}

impl std::error::Error for ShellbagError {}

impl fmt::Display for ShellbagError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            ShellbagError::GetRegistryData => write!(f, "Failed to get Registry Shellbag data"),
            ShellbagError::DefaultDrive => write!(f, "Failed to get default driver letter"),
        }
    }
}
