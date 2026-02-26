use std::fmt;

#[derive(Debug, PartialEq, Eq)]
pub(crate) enum UsnJrnlError {
    Attribute,
    SystemDrive,
    Parser,
    ReadFile,
    Serialize,
    OutputData,
}

impl std::error::Error for UsnJrnlError {}

impl fmt::Display for UsnJrnlError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            UsnJrnlError::Attribute => write!(f, "Failed to get attribute data"),
            UsnJrnlError::SystemDrive => write!(f, "Failed to systemdrive env variable value"),
            UsnJrnlError::Parser => write!(f, "Failed to parse usnrjnl"),
            UsnJrnlError::ReadFile => write!(f, "Failed to read usnrjnl"),
            UsnJrnlError::Serialize => write!(f, "Failed to serialize usnrjnl data"),
            UsnJrnlError::OutputData => write!(f, "Failed to output usnrjnl data"),
        }
    }
}
