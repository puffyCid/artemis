use std::fmt;

#[derive(Debug)]
pub enum EventLogsError {
    DefaultDrive,
    Parser,
    Serialize,
}

impl std::error::Error for EventLogsError {}

impl fmt::Display for EventLogsError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            EventLogsError::DefaultDrive => write!(f, "Failed to get default driver letter"),
            EventLogsError::Parser => write!(f, "Failed to parse event logs"),
            EventLogsError::Serialize => write!(f, "Failed to serialize event logs"),
        }
    }
}
