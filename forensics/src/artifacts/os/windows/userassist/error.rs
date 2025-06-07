use std::fmt;

#[derive(Debug, PartialEq, Eq)]
pub(crate) enum UserAssistError {
    RegistryFiles,
    UserAssistData,
    DriveLetter,
}

impl std::error::Error for UserAssistError {}

impl fmt::Display for UserAssistError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            UserAssistError::RegistryFiles => write!(f, "Could not get user Registry files"),
            UserAssistError::UserAssistData => write!(f, "Could not get UserAssist data"),
            UserAssistError::DriveLetter => write!(f, "Failed to get systemdrive letter"),
        }
    }
}
