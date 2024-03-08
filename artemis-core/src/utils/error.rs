use std::fmt;

#[derive(Debug)]
pub enum ArtemisError {
    BadToml,
    Regex,
    #[cfg(target_os = "windows")]
    Env,
    CreateDirectory,
    LogFile,
    Local,
    Remote,
    Cleanup,
    ReadXml,
    UtfType,
}

impl std::error::Error for ArtemisError {}

impl fmt::Display for ArtemisError {
    fn fmt<'a>(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            ArtemisError::BadToml => write!(f, "Failed to parse TOML data"),
            ArtemisError::Regex => write!(f, "Invalid regex provided"),
            #[cfg(target_os = "windows")]
            ArtemisError::Env => write!(f, "Could not get environment variable"),
            ArtemisError::CreateDirectory => write!(f, "Could not create directory(ies)"),
            ArtemisError::LogFile => write!(f, "Could not create log file"),
            ArtemisError::Local => write!(f, "Failed output data to local directory"),
            ArtemisError::Remote => write!(f, "Failed output data to remote URL"),
            ArtemisError::Cleanup => write!(f, "Failed to delete artemis output files"),
            ArtemisError::ReadXml => write!(f, "Failed to read XML"),
            ArtemisError::UtfType => write!(f, "Failed to determine UTF XML type"),
        }
    }
}
