use std::fmt;

#[derive(Debug)]
pub enum ArtemisError {
    BadToml,
    Regex,
    #[cfg(target_os = "windows")]
    Env,
    #[cfg(target_family = "unix")]
    GzipReadFile,
    #[cfg(target_family = "unix")]
    GzipDecompress,
    #[cfg(target_family = "unix")]
    ZstdDecompresss,
    #[cfg(target_family = "unix")]
    Lz4Decompresss,
    #[cfg(target_family = "unix")]
    XzDecompress,
    CompressCreate,
    GzipFinish,
    CreateDirectory,
    LogFile,
    #[cfg(target_os = "windows")]
    HuffmanCompression,
    Local,
    Remote,
    Cleanup,
    ReadXml,
    UtfType,
}

impl std::error::Error for ArtemisError {}

impl fmt::Display for ArtemisError {
    fn fmt<'a>(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            ArtemisError::BadToml => write!(f, "Failed to parse TOML data"),
            ArtemisError::Regex => write!(f, "Invalid regex provided"),
            #[cfg(target_os = "windows")]
            ArtemisError::Env => write!(f, "Could not get environment variable"),
            #[cfg(target_family = "unix")]
            ArtemisError::GzipReadFile => write!(f, "Could not read file"),
            #[cfg(target_family = "unix")]
            ArtemisError::GzipDecompress => write!(f, "Could not decompress gzip data"),
            #[cfg(target_family = "unix")]
            ArtemisError::ZstdDecompresss => write!(f, "Could not decompress zstd data"),
            #[cfg(target_family = "unix")]
            ArtemisError::Lz4Decompresss => write!(f, "Could not decompress lz4 data"),
            #[cfg(target_family = "unix")]
            ArtemisError::XzDecompress => write!(f, "Could not decompress xz data"),
            ArtemisError::CompressCreate => write!(f, "Could not create file for compression"),
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
            ArtemisError::ReadXml => write!(f, "Failed to read XML"),
            ArtemisError::UtfType => write!(f, "Failed to determine UTF XML type"),
        }
    }
}
