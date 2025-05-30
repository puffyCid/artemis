use std::fmt;

#[derive(Debug)]
pub enum ConfigError {
    BadToml,
    DaemonTomlWrite,
    ServerTomlWrite,
    NoPath,
}

impl std::error::Error for ConfigError {}

impl fmt::Display for ConfigError {
    fn fmt<'a>(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            ConfigError::BadToml => write!(f, "Failed to parse TOML data"),
            ConfigError::DaemonTomlWrite => write!(f, "Could not write daemon TOML file"),
            ConfigError::ServerTomlWrite => write!(f, "Could not write server TOML file"),
            ConfigError::NoPath => write!(f, "Could not find suitable path for configs"),
        }
    }
}
