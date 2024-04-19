use std::fmt;

#[derive(Debug)]
pub(crate) enum AcquireError {
    Reader,
    Compressor,
    CreateDirectory,
    Timestamps,
    Metadata,
    ZipOutput,
    Cleanup,
}

impl std::error::Error for AcquireError {}

impl fmt::Display for AcquireError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            AcquireError::Reader => write!(f, "Could not read data"),
            AcquireError::CreateDirectory => write!(f, "Could not create directory for output"),
            AcquireError::Compressor => write!(f, "Could not compress data"),
            AcquireError::Timestamps => write!(f, "Could not get timestamps for acquired file"),
            AcquireError::Metadata => write!(f, "Could not get metadata for acquired file"),
            AcquireError::ZipOutput => write!(f, "Could not zip acquired file"),
            AcquireError::Cleanup => write!(f, "Could not cleanup file acquisition"),
        }
    }
}
