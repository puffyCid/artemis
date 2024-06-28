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
    MaxAttempts,
    GcpStatus,
    GcpSetup,
    GcpToken,
    GcpSession,
    AwsSetup,
    AwsUpload,
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
            AcquireError::MaxAttempts => write!(f, "Reached max upload attempts"),
            AcquireError::GcpStatus => write!(f, "No upload status response"),
            AcquireError::GcpSetup => write!(f, "Could not setup GCP upload"),
            AcquireError::GcpToken => write!(f, "Could not create GCP token"),
            AcquireError::GcpSession => write!(f, "Could not create GCP session"),
            AcquireError::AwsSetup => write!(f, "Could not setup AWS upload"),
            AcquireError::AwsUpload => write!(f, "Could not upload AWS data"),
        }
    }
}
