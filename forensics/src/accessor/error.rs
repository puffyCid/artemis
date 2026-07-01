use std::{fmt, io, path::PathBuf};

pub(crate) type AccessorResult<T> = Result<T, AccessorError>;

#[derive(Debug)]
pub(crate) enum AccessorError {
    /// A location or source string could not be parsed
    Location {
        /// Original input string
        input: String,
        /// Why parsing failed
        reason: String,
    },
    /// Provided scheme not supported
    UnsupportedScheme {
        /// Scheme name. Examples: ewf, raw, qcow2
        scheme: String,
    },
    /// Raw volume access not available
    RawAccessNotSupported {
        /// Human explanation of error
        reason: String,
    },
    /// Configured `AccessMode` rejected access
    AccessMode {
        /// Human explanation of error
        reason: String,
    },
    /// File or directory found at provided path
    NotFound {
        /// Path provided
        path: String,
    },
    /// The path exists but refers to a directory, not a file
    NotAFile {
        /// Path provided
        path: String,
    },
    /// The path exists but refers to a file, not a directory
    NotADirectory {
        /// Path provided
        path: String,
    },
    /// File read would exceed the configured size limit
    FileTooLarge {
        /// File size in bytes
        size: u64,
        /// Max allowed file read size in bytes
        limit: u64,
    },
    /// A glob pattern could not be applied
    BadGlob {
        /// Glob provided by the caller
        pattern: String,
        /// Why the pattern failed
        reason: String,
    },
    /// An `EntryHandle` or `DirHandle` is invalid for the requested operation
    InvalidHandle {
        /// Human explanation of error
        reason: String,
    },
    /// A `SourceHandle` was not found in the cache
    SourceNotOpen {
        /// Source identifier
        source_id: String,
    },
    /// Underlying filesystem or OS IO failure
    Io {
        /// Optional path associated with the failure
        path: Option<PathBuf>,
        /// Original IO error
        source: io::Error,
    },
    /// NTFS parsing or raw read failure
    Ntfs {
        /// Path if known
        path: Option<String>,
        /// Human explanation of error
        reason: String,
    },
    /// Zip archive read failure
    Zip {
        /// Archive path if known
        archive: Option<PathBuf>,
        /// Human explanation of error
        reason: String,
    },
    /// Disk image failure. Example: EWF, QCOW2, raw
    Volume {
        /// Human explanation of error
        reason: String,
    },
    /// Filesystem implementation failure for non-specific variants
    Filesystem {
        /// Human explanation of error
        reason: String,
    },
}

impl std::error::Error for AccessorError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            Self::Io { source, .. } => Some(source),
            _ => None,
        }
    }
}

impl fmt::Display for AccessorError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            AccessorError::Location { input, reason } => {
                write!(f, "could not parse location '{input}': {reason}")
            }
            AccessorError::UnsupportedScheme { scheme } => {
                write!(f, "unsupported accessor scheme '{scheme}'")
            }
            AccessorError::RawAccessNotSupported { reason } => {
                write!(f, "raw disk access int supported: {reason}")
            }
            AccessorError::AccessMode { reason } => {
                write!(f, "access mode rejected operation: {reason}")
            }
            AccessorError::NotFound { path } => {
                write!(f, "path not found: {path}")
            }
            AccessorError::NotAFile { path } => {
                write!(f, "not a file: {path}")
            }
            AccessorError::NotADirectory { path } => {
                write!(f, "not a directory: {path}")
            }
            AccessorError::FileTooLarge { size, limit } => {
                write!(
                    f,
                    "file size {size} bytes exceeds accessor limit of {limit} bytes"
                )
            }
            AccessorError::BadGlob { pattern, reason } => {
                write!(f, "bad glob pattern '{pattern}': {reason}")
            }
            AccessorError::InvalidHandle { reason } => {
                write!(f, "invalid handle: {reason}")
            }
            AccessorError::SourceNotOpen { source_id } => {
                write!(f, "source is not open: {source_id}")
            }
            AccessorError::Io { path, source } => {
                if let Some(io_path) = path {
                    write!(f, "IO error at {}: {source}", io_path.display())
                } else {
                    write!(f, "IO error: {source}")
                }
            }
            AccessorError::Ntfs { path, reason } => {
                if let Some(ntfs_path) = path {
                    write!(f, "NTFS error at {ntfs_path}: {reason}")
                } else {
                    write!(f, "NTFS error: {reason}")
                }
            }
            AccessorError::Zip { archive, reason } => {
                if let Some(archive_path) = archive {
                    write!(f, "zip error for {}: {reason}", archive_path.display())
                } else {
                    write!(f, "zip error: {reason}")
                }
            }
            AccessorError::Volume { reason } => {
                write!(f, "volume error: {reason}")
            }
            AccessorError::Filesystem { reason } => {
                write!(f, "filesystem error: {reason}")
            }
        }
    }
}

impl From<io::Error> for AccessorError {
    fn from(source: io::Error) -> Self {
        Self::Io { path: None, source }
    }
}

impl AccessorError {
    pub(crate) fn io_path(path: impl Into<PathBuf>, source: io::Error) -> Self {
        Self::Io {
            path: Some(path.into()),
            source,
        }
    }
    pub(crate) fn location(input: impl Into<String>, reason: impl Into<String>) -> Self {
        Self::Location {
            input: input.into(),
            reason: reason.into(),
        }
    }
    pub(crate) fn unsupported_scheme(scheme: impl Into<String>) -> Self {
        Self::UnsupportedScheme {
            scheme: scheme.into(),
        }
    }
    pub(crate) fn not_found(path: impl Into<String>) -> Self {
        Self::NotFound { path: path.into() }
    }
    pub(crate) fn not_a_file(path: impl Into<String>) -> Self {
        Self::NotAFile { path: path.into() }
    }
    pub(crate) fn not_a_directory(path: impl Into<String>) -> Self {
        Self::NotADirectory { path: path.into() }
    }
    pub(crate) fn file_too_large(size: u64, limit: u64) -> Self {
        Self::FileTooLarge { size, limit }
    }
    pub(crate) fn bad_glob(pattern: impl Into<String>, reason: impl Into<String>) -> Self {
        Self::BadGlob {
            pattern: pattern.into(),
            reason: reason.into(),
        }
    }
    pub(crate) fn is_recoverable_via_raw(&self) -> bool {
        matches!(self, Self::Io { source, .. } if matches!(source.kind(), io::ErrorKind::PermissionDenied | io::ErrorKind::NotFound))
    }
}
