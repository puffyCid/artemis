use std::fmt;

#[derive(Debug)]
pub(crate) enum SystemInfoError {
    Serialize,
}

impl std::error::Error for SystemInfoError {}

impl fmt::Display for SystemInfoError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            SystemInfoError::Serialize => {
                write!(f, "Failed to serialize process listing")
            }
        }
    }
}
