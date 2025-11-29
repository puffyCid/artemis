use std::fmt;

#[derive(Debug)]
pub(crate) enum DiskError {
    Qcow,
}

impl std::error::Error for DiskError {}

impl fmt::Display for DiskError {
    fn fmt<'a>(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            DiskError::Qcow => write!(f, "Failed to setup QCOW reader"),
        }
    }
}
