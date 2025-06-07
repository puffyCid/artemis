use std::fmt;

#[derive(Debug)]
pub enum ConnectionsError {
    ConnectionList,
    Serialize,
    OutputData,
}

impl std::error::Error for ConnectionsError {}

impl fmt::Display for ConnectionsError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            ConnectionsError::ConnectionList => {
                write!(f, "Failed to get connections listing")
            }
            ConnectionsError::Serialize => {
                write!(f, "Failed to serialize connections listing")
            }
            ConnectionsError::OutputData => {
                write!(f, "Failed to output connections listing")
            }
        }
    }
}
