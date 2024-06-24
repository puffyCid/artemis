use std::fmt;

#[derive(Debug)]
pub enum SocketError {
    StartConnection,
    SaveCollection,
    QuickCollection,
}

impl fmt::Display for SocketError {
    fn fmt<'a>(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            SocketError::StartConnection => write!(f, "Failed to start websocket connection"),
            SocketError::SaveCollection => write!(f, "Could not create collection file"),
            SocketError::QuickCollection => write!(f, "Could not do quick collection"),
        }
    }
}
