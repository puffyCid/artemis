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
    NtfsSectorReader,
    NtfsNew,
    RootDirectory,
    NoFilenameAttr,
    NoAttribute,
    IndexDirectory,
    FileData,
    NoDataAttributeValue,
    LargeFile,
    NoRootHome,
    CompressFile,
    CompressedBytes,
    AcquireFile,
    UploadSetup,
    FinalUpload,
    DecodeYara,
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
            FileSystemError::NtfsSectorReader => write!(f, "Failed to setup NTFS sector reader"),
            FileSystemError::NtfsNew => write!(f, "Failed to start NTFS parser"),
            FileSystemError::RootDirectory => write!(f, "Failed to get NTFS root directory"),
            FileSystemError::NoFilenameAttr => write!(f, "Failed to get NTFS $FILENAME info"),
            FileSystemError::IndexDirectory => write!(f, "Failed to get NTFS index directory info"),
            FileSystemError::FileData => write!(f, "Failed to get NTFS file data"),
            FileSystemError::LargeFile => write!(f, "File larger than 2GB"),
            FileSystemError::NoDataAttributeValue => {
                write!(f, "Failed to get NTFS $DATA attribute")
            }
            FileSystemError::NoRootHome => write!(f, "Could not find root home directory"),
            FileSystemError::NoAttribute => write!(f, "No attribute for entry"),
            FileSystemError::CompressFile => write!(f, "Cannot compress acquire file"),
            FileSystemError::CompressedBytes => write!(f, "Cannot compress all bytes"),
            FileSystemError::AcquireFile => write!(f, "Could not finish file acquisition"),
            FileSystemError::UploadSetup => write!(f, "Could not setup file upload"),
            FileSystemError::FinalUpload => write!(f, "Could not finish file upload"),
            FileSystemError::DecodeYara => write!(f, "Could not decode yara rule"),
        }
    }
}
