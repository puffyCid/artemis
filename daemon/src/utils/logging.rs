use super::config::ServerToml;
use crate::error::DaemonError;
use log::LevelFilter;
use std::fs::{File, create_dir_all};

pub(crate) fn setup_logging(config: &ServerToml) -> Result<(File, LevelFilter), DaemonError> {
    if let Err(_err) = create_dir_all(&config.log_path) {
        return Err(DaemonError::LogFile);
    }

    let log_file = match File::create(format!("{}/daemon.log", config.log_path.as_str())) {
        Ok(result) => result,
        Err(_err) => return Err(DaemonError::LogFile),
    };

    let log_level = match config.log_level.as_str() {
        "error" => LevelFilter::Error,
        "info" => LevelFilter::Info,
        "debug" => LevelFilter::Debug,
        _ => LevelFilter::Warn,
    };

    Ok((log_file, log_level))
}
