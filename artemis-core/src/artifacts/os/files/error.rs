use std::fmt;

#[derive(Debug)]
pub(crate) enum FileError {
    Regex,
    #[cfg(target_os = "macos")]
    ReadFile,
    ParseFile,
}

impl std::error::Error for FileError {}

impl fmt::Display for FileError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            FileError::Regex => write!(f, "Failed to compile file regex"),
            FileError::ParseFile => write!(f, "Failed to get parse executable file"),
            #[cfg(target_os = "macos")]
            FileError::ReadFile => write!(f, "Failed to read file"),
        }
    }
}
