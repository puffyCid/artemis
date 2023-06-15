use std::fmt;

#[derive(Debug)]
pub enum ArtemisError {
    BadToml,
    Regex,
    #[cfg(target_os = "windows")]
    Env,
    #[cfg(target_os = "macos")]
    GzipReadFile,
    #[cfg(target_os = "macos")]
    GzipDecompress,
    GzipOpen,
    CompressCreate,
    GzipCopy,
    GzipFinish,
    CreateDirectory,
    LogFile,
    #[cfg(target_os = "windows")]
    HuffmanCompression,
    Local,
    Remote,
    Cleanup,
}

impl std::error::Error for ArtemisError {}

impl fmt::Display for ArtemisError {
    fn fmt<'a>(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            ArtemisError::BadToml => write!(f, "Failed to parse TOML data"),
            ArtemisError::Regex => write!(f, "Invalid regex provided"),
            #[cfg(target_os = "windows")]
            ArtemisError::Env => write!(f, "Could not get environment variable"),
            #[cfg(target_os = "macos")]
            ArtemisError::GzipReadFile => write!(f, "Could not read file"),
            #[cfg(target_os = "macos")]
            ArtemisError::GzipDecompress => write!(f, "Could not decompress gzip data"),
            ArtemisError::GzipOpen => write!(f, "Could not open file for compression"),
            ArtemisError::CompressCreate => write!(f, "Could not create file for compression"),
            ArtemisError::GzipCopy => write!(f, "Could not copy data for compression"),
            ArtemisError::GzipFinish => write!(f, "Could not complete gzip compression"),
            ArtemisError::CreateDirectory => write!(f, "Could not create directory(ies)"),
            ArtemisError::LogFile => write!(f, "Could not create log file"),

            #[cfg(target_os = "windows")]
            ArtemisError::HuffmanCompression => {
                write!(f, "Failed to decompress huffman compressed data")
            }
            ArtemisError::Local => write!(f, "Failed output data to local directory"),
            ArtemisError::Remote => write!(f, "Failed output data to remote URL"),
            ArtemisError::Cleanup => write!(f, "Failed to delete artemis output files"),
        }
    }
}
