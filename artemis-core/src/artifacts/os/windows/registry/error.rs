use std::fmt;

#[derive(Debug, PartialEq, Eq)]
pub(crate) enum RegistryError {
    Parser,
    Regex,
    ReadRegistry,
    GetUserHives,
    NtfsSetup,
    Serialize,
    Output,
    SystemDrive,
}

impl std::error::Error for RegistryError {}

impl fmt::Display for RegistryError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            RegistryError::Parser => write!(f, "Failed to parse Registry data"),
            RegistryError::Regex => write!(f, "Invalid regex provided"),
            RegistryError::ReadRegistry => write!(f, "Could not read Registry file"),
            RegistryError::GetUserHives => {
                write!(f, "Could not get user Registry hives via NTFS parser")
            }
            RegistryError::NtfsSetup => write!(f, "Could not setup NTFS parser"),
            RegistryError::Serialize => write!(f, "Could not serialize Registry data"),
            RegistryError::Output => write!(f, "Could not output Registry data"),
            RegistryError::SystemDrive => write!(f, "Could not get systemdrive"),
        }
    }
}
