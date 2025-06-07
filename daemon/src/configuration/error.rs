use std::fmt;

#[derive(Debug)]
pub enum ConfigError {
    FailedConfig,
    BadConfig,
    ConfigNotOk,
}

impl std::error::Error for ConfigError {}

impl fmt::Display for ConfigError {
    fn fmt<'a>(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            ConfigError::FailedConfig => write!(f, "Failed to get config for endpoint"),
            ConfigError::BadConfig => write!(f, "Config data was bad"),
            ConfigError::ConfigNotOk => write!(f, "Server returned non-Ok response"),
        }
    }
}
