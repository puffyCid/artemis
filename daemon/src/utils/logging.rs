use super::config::ServerToml;
use crate::error::DaemonError;
use std::fs::{File, create_dir_all};
use tracing::level_filters::LevelFilter;

pub(crate) fn setup_logging(config: &ServerToml) -> Result<(File, LevelFilter), DaemonError> {
    if let Err(_err) = create_dir_all(&config.log_path) {
        return Err(DaemonError::LogFile);
    }

    let log_file = match File::create(format!("{}/daemon.jsonl", config.log_path.as_str())) {
        Ok(result) => result,
        Err(_err) => return Err(DaemonError::LogFile),
    };

    let log_level = match config.log_level.as_str() {
        "error" => LevelFilter::ERROR,
        "info" => LevelFilter::INFO,
        "debug" => LevelFilter::DEBUG,
        _ => LevelFilter::WARN,
    };

    Ok((log_file, log_level))
}
