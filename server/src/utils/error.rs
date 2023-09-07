use std::fmt;

#[derive(Debug)]
pub enum UtilServerError {
    NoConfig,
    BadToml,
    NotFile,
    ReadFile,
    CreateDirectory,
}

impl fmt::Display for UtilServerError {
    fn fmt<'a>(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            UtilServerError::NoConfig => write!(f, "Could not read config file"),
            UtilServerError::BadToml => write!(f, "Failed to parse TOML data"),
            UtilServerError::NotFile => write!(f, "Not a file"),
            UtilServerError::ReadFile => write!(f, "Could not read file"),
            UtilServerError::CreateDirectory => write!(f, "Could create directory"),
        }
    }
}
