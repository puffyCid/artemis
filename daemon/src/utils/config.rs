use super::error::ConfigError;
use crate::error::DaemonError;
use log::error;
use serde::Deserialize;
use std::str::from_utf8;
use tokio::fs::read;

#[derive(Deserialize, Debug)]
pub(crate) struct ServerConfig {
    pub(crate) server: Server,
}

#[derive(Deserialize, Debug)]
pub(crate) struct Server {
    pub(crate) url: String,
    pub(crate) port: u16,
    pub(crate) ignore_ssl: bool,
    pub(crate) enrollment: String,
    pub(crate) collections: String,
    pub(crate) ping: String,
    pub(crate) version: u8,
    pub(crate) key: String,
}

/// Parse the provided `Server` TOML config file
pub(crate) async fn server(path: &str) -> Result<ServerConfig, ConfigError> {
    let bytes = match read_file(path).await {
        Ok(result) => result,
        Err(_err) => return Err(ConfigError::BadToml),
    };

    let config: ServerConfig = match toml::from_str(from_utf8(&bytes).unwrap_or_default()) {
        Ok(result) => result,
        Err(err) => {
            println!("[daemon] Failed to parse server config {path}: {err:?}");
            return Err(ConfigError::BadToml);
        }
    };

    Ok(config)
}

/// Read the provided file
async fn read_file(path: &str) -> Result<Vec<u8>, DaemonError> {
    let bytes = match read(path).await {
        Ok(result) => result,
        Err(err) => {
            error!("[daemon] Failed to read file {path}: {err:?}");
            return Err(DaemonError::ReadFile);
        }
    };

    Ok(bytes)
}

#[cfg(test)]
mod tests {
    use crate::utils::config::{read_file, server};
    use std::path::PathBuf;

    #[tokio::test]
    async fn test_read_file() {
        let mut test_location = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
        test_location.push("tests/configs/server.toml");

        let result = read_file(test_location.to_str().unwrap()).await.unwrap();

        assert_eq!(result.len(), 183);
    }

    #[tokio::test]
    #[should_panic(expected = "ReadFile")]
    async fn test_read_no_file() {
        let mut test_location = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
        test_location.push("tests/configs/server123.toml");

        let _ = read_file(test_location.to_str().unwrap()).await.unwrap();
    }

    #[tokio::test]
    async fn test_server_config() {
        let mut test_location = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
        test_location.push("tests/configs/server.toml");

        let result = server(test_location.to_str().unwrap()).await.unwrap();

        assert_eq!(result.server.collections, "endpoint/collections");
        assert_eq!(result.server.version, 1);
        assert_eq!(result.server.url, "http://127.0.0.1");
        assert_eq!(result.server.ignore_ssl, false);
        assert_eq!(result.server.ping, "endpoint/ping");
        assert_eq!(result.server.enrollment, "endpoint/enroll");
        assert_eq!(result.server.port, 8000);
        assert_eq!(result.server.key, "my key");
    }
}
