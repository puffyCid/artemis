use std::fmt;

#[derive(Debug)]
pub enum PrefetchError {
    Header,
    Decompress,
    Version,
    FileMetrics,
    Filenames,
    VolumeInfo,
    ReadFile,
    ReadDirectory,
    DriveLetter,
}

impl std::error::Error for PrefetchError {}

impl fmt::Display for PrefetchError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            PrefetchError::Header => write!(f, "Failed to read prefetch header"),
            PrefetchError::Decompress => write!(f, "Failed to decompress prefetch data"),
            PrefetchError::Version => write!(f, "Failed to parse version data"),
            PrefetchError::FileMetrics => write!(f, "Failed to parse file metrics data"),
            PrefetchError::Filenames => write!(f, "Failed to get filenames from prefetch"),
            PrefetchError::VolumeInfo => write!(f, "Failed to get volume data from prefetch"),
            PrefetchError::ReadFile => write!(f, "Failed to read file"),
            PrefetchError::ReadDirectory => write!(f, "Failed to read directory"),
            PrefetchError::DriveLetter => write!(f, "Failed to get drive eltter"),
        }
    }
}
