use std::fmt;

#[derive(Debug)]
pub enum FileSystemError {
    BadToml,
    NotFile,
    ReadFile,
    CreateDirectory,
    CrateFile,
    WriteFile,
}

impl fmt::Display for FileSystemError {
    fn fmt<'a>(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            FileSystemError::BadToml => write!(f, "Failed to parse TOML data"),
            FileSystemError::NotFile => write!(f, "Not a file"),
            FileSystemError::ReadFile => write!(f, "Could not read file"),
            FileSystemError::CreateDirectory => write!(f, "Could not create directory"),
            FileSystemError::CrateFile => write!(f, "Could not create file"),
            FileSystemError::WriteFile => write!(f, "Could not write file"),
        }
    }
}
