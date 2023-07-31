use std::fmt;

#[derive(Debug)]
pub(crate) enum FileSystemError {
    ReadDirectory,
    NotDirectory,
    UserPaths,
    NoUserParent,
    OpenFile,
    ReadFile,
    NotFile,
    BadGlob,
    #[cfg(target_os = "windows")]
    NtfsSectorReader,
    #[cfg(target_os = "windows")]
    NtfsNew,
    #[cfg(target_os = "windows")]
    RootDirectory,
    #[cfg(target_os = "windows")]
    NoFilenameAttr,
    #[cfg(target_os = "windows")]
    NoAttribute,
    #[cfg(target_os = "windows")]
    IndexDirectory,
    #[cfg(target_os = "windows")]
    FileData,
    #[cfg(target_os = "windows")]
    NoDataAttributeValue,
    LargeFile,
    #[cfg(target_family = "unix")]
    NoRootHome,
}

impl std::error::Error for FileSystemError {}

impl fmt::Display for FileSystemError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            FileSystemError::ReadDirectory => write!(f, "Could not read directory path"),
            FileSystemError::NotDirectory => write!(f, "Not a directory"),
            FileSystemError::UserPaths => write!(f, "Could not determine user home paths"),
            FileSystemError::NoUserParent => write!(f, "Could not find user parent path"),
            FileSystemError::BadGlob => write!(f, "Could not glob"),
            FileSystemError::OpenFile => write!(f, "Could not open file"),
            FileSystemError::ReadFile => write!(f, "Could not read file"),
            FileSystemError::NotFile => write!(f, "Not a file"),
            #[cfg(target_os = "windows")]
            FileSystemError::NtfsSectorReader => write!(f, "Failed to setup NTFS sector reader"),
            #[cfg(target_os = "windows")]
            FileSystemError::NtfsNew => write!(f, "Failed to start NTFS parser"),
            #[cfg(target_os = "windows")]
            FileSystemError::RootDirectory => write!(f, "Failed to get NTFS root directory"),
            #[cfg(target_os = "windows")]
            FileSystemError::NoFilenameAttr => write!(f, "Failed to get NTFS $FILENAME info"),
            #[cfg(target_os = "windows")]
            FileSystemError::IndexDirectory => write!(f, "Failed to get NTFS index directory info"),
            #[cfg(target_os = "windows")]
            FileSystemError::FileData => write!(f, "Failed to get NTFS file data"),
            FileSystemError::LargeFile => write!(f, "File larger than 2GB"),
            #[cfg(target_os = "windows")]
            FileSystemError::NoDataAttributeValue => {
                write!(f, "Failed to get NTFS $DATA attribute")
            }
            #[cfg(target_family = "unix")]
            FileSystemError::NoRootHome => write!(f, "Could not find root home directory"),
            #[cfg(target_os = "windows")]
            FileSystemError::NoAttribute => write!(f, "No attribute for entry"),
        }
    }
}
