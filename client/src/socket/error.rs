use std::fmt;

#[derive(Debug)]
pub enum SocketError {
    StartConnection,
    ParseJob,
    SaveJob,
}

impl fmt::Display for SocketError {
    fn fmt<'a>(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            SocketError::StartConnection => write!(f, "Failed to start websocket connection"),
            SocketError::SaveJob => write!(f, "Could not create job file"),
            SocketError::ParseJob => write!(f, "Could not parse job command"),
        }
    }
}
