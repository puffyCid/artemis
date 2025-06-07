use std::fmt;

#[derive(Debug)]
pub(crate) enum LocalError {
    CreateDirectory,
    CreateFile,
    WriteJson,
}

impl std::error::Error for LocalError {}

impl fmt::Display for LocalError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            LocalError::CreateDirectory => write!(f, "Failed to create output directory"),
            LocalError::CreateFile => write!(f, "Failed to create output file"),
            LocalError::WriteJson => write!(f, "Failed write json data"),
        }
    }
}
