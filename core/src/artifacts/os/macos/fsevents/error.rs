use std::fmt;

#[derive(Debug)]
pub(crate) enum FsEventsError {
    Decompress,
    Files,
    Serialize,
    OutputData,
}

impl std::error::Error for FsEventsError {}

impl fmt::Display for FsEventsError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            FsEventsError::Decompress => write!(f, "Could not decompress fsevents file"),
            FsEventsError::Files => write!(f, "Could not get FsfseventsEvents files"),
            FsEventsError::Serialize => write!(f, "Could not serialize fsevents"),
            FsEventsError::OutputData => write!(f, "Could not output fsevents"),
        }
    }
}
