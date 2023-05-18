use std::fmt;

#[derive(Debug)]
pub enum ShimdbError {
    ParseSdb,
    ReadFile,
    ReadDirectory,
    DriveLetter,
}

impl std::error::Error for ShimdbError {}

impl fmt::Display for ShimdbError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            ShimdbError::ParseSdb => write!(f, "Failed to parse sdb file"),
            ShimdbError::ReadFile => write!(f, "Failed to read file"),
            ShimdbError::ReadDirectory => write!(f, "Failed to read directory"),
            ShimdbError::DriveLetter => write!(f, "Failed to get systemdrive"),
        }
    }
}
