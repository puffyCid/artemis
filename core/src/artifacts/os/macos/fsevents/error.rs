use std::fmt;

#[derive(Debug)]
pub(crate) enum FsEventsError {
    Decompress,
    Files,
}

impl std::error::Error for FsEventsError {}

impl fmt::Display for FsEventsError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            FsEventsError::Decompress => write!(f, "Could not decompress FsEvents file"),
            FsEventsError::Files => write!(f, "Could not get FsEvents files"),
        }
    }
}
