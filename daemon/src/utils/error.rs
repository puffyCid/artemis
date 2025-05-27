use std::fmt;

#[derive(Debug)]
pub enum ConfigError {
    BadToml,
}

impl std::error::Error for ConfigError {}

impl fmt::Display for ConfigError {
    fn fmt<'a>(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            ConfigError::BadToml => write!(f, "Failed to parse TOML data"),
        }
    }
}
