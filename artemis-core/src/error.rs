use std::fmt;

#[derive(Debug)]
pub enum TomlError {
    NoFile,
    FailedToReadFile,
    BadToml,
    BadJs,
}

impl std::error::Error for TomlError {}

impl fmt::Display for TomlError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            TomlError::NoFile => write!(f, "Failed to read TOML file"),
            TomlError::FailedToReadFile => write!(f, "Failed to read TOML data"),
            TomlError::BadToml => write!(f, "Failed to parse TOML data"),
            TomlError::BadJs => write!(f, "Failed run JavaScript code"),
        }
    }
}
