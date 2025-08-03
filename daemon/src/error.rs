use std::fmt;

#[derive(Debug)]
pub enum DaemonError {
    ReadFile,
    MakeDirectory,
    WriteFile,
    LogFile,
    BadToml,
    DaemonTomlWrite,
    ServerTomlWrite,
    NoPath,
}

impl std::error::Error for DaemonError {}

impl fmt::Display for DaemonError {
    fn fmt<'a>(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            DaemonError::ReadFile => write!(f, "Failed to read file"),
            DaemonError::MakeDirectory => write!(f, "Failed to create directory"),
            DaemonError::WriteFile => write!(f, "Failed to create file"),
            DaemonError::LogFile => write!(f, "Failed to create log file"),
            DaemonError::BadToml => write!(f, "Failed to parse TOML data"),
            DaemonError::DaemonTomlWrite => write!(f, "Could not write daemon TOML file"),
            DaemonError::ServerTomlWrite => write!(f, "Could not write server TOML file"),
            DaemonError::NoPath => write!(f, "Could not find suitable path for configs"),
        }
    }
}
