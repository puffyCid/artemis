use super::{
    error::ConfigError,
    info::{PlatformType, get_platform_enum},
};
use crate::{error::DaemonError, utils::env::get_env_value};
use log::error;
use serde::{Deserialize, Serialize};
use std::str::from_utf8;
use tokio::fs::{create_dir_all, read, write};

#[derive(Deserialize, Debug)]
pub(crate) struct ServerToml {
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
    pub(crate) config: String,
    pub(crate) version: u8,
    pub(crate) key: String,
}

/// Parse the provided `Server` TOML config file
pub(crate) async fn server(path: &str, alt_base: Option<&str>) -> Result<ServerToml, ConfigError> {
    let bytes = match read_file(path).await {
        Ok(result) => result,
        Err(_err) => return Err(ConfigError::BadToml),
    };

    let mut default_path = String::from("/var/artemis");

    if get_platform_enum() == PlatformType::Windows {
        default_path = format!("{}\\artemis", get_env_value("ProgramData"));
        if default_path == "\\artemis" && alt_base.is_none() {
            error!("[daemon] Failed to find ProgramData env value and alt_base is none");
            return Err(ConfigError::NoPath);
        }
    }

    if let Some(alt_path) = alt_base {
        default_path = alt_path.to_string();
    }

    let _ = create_directory(&default_path).await;

    let config: ServerToml = match toml::from_str(from_utf8(&bytes).unwrap_or_default()) {
        Ok(result) => result,
        Err(err) => {
            error!("[daemon] Failed to parse server config {path}: {err:?}");
            return Err(ConfigError::BadToml);
        }
    };

    Ok(config)
}

#[derive(Deserialize, Serialize, Debug)]
pub(crate) struct DaemonToml {
    pub(crate) daemon: Daemon,
}

#[derive(Deserialize, Serialize, Debug)]
pub(crate) struct Daemon {
    pub(crate) node_key: String,
    pub(crate) collection_path: String,
    pub(crate) log_level: String,
}

/// Create a daemon config file. If the `DaemonToml` structure is empty. `Ex: collection_path = ""`.  
/// Then default paths will be used
pub(crate) async fn daemon(
    config: &mut DaemonToml,
    alt_artemis_path: Option<&str>,
) -> Result<(), ConfigError> {
    let mut collect_path = String::from("/var/artemis/collections");
    let mut artemis_path = String::from("/var/artemis");

    if get_platform_enum() == PlatformType::Windows {
        let programdata = get_env_value("ProgramData");
        if programdata.is_empty() && alt_artemis_path.is_none() {
            error!("[daemon] Failed to find ProgramData env value and alt path is none");
            return Err(ConfigError::NoPath);
        }

        collect_path = format!("{programdata}\\artemis\\collections");
        artemis_path = format!("{programdata}\\artemis");
    }

    // Check if we are using alternative collections path
    if config.daemon.collection_path.is_empty() {
        config.daemon.collection_path = collect_path;
    }

    // Check if we are using alternative logging level
    if config.daemon.log_level.is_empty() {
        config.daemon.log_level = String::from("warn");
    }

    // Check if we are using alternative base path
    if let Some(alt_path) = alt_artemis_path {
        artemis_path = alt_path.to_string();
    }

    let daemon_config = match toml::to_string(&config) {
        Ok(result) => result,
        Err(err) => {
            error!("[daemon] Failed to parse daemon config: {err:?}");
            return Err(ConfigError::BadToml);
        }
    };

    let _ = create_directory(&config.daemon.collection_path).await;
    if let Err(status) = write_file(
        daemon_config.as_bytes(),
        &format!("{artemis_path}/daemon.toml"),
    )
    .await
    {
        error!("[daemon] Could not write daemon TOML file at {artemis_path}: {status:?}");
        return Err(ConfigError::DaemonTomlWrite);
    }

    Ok(())
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

/// Create directory and any parents
async fn create_directory(path: &str) -> Result<(), DaemonError> {
    match create_dir_all(path).await {
        Ok(result) => Ok(result),
        Err(err) => {
            error!("[daemon] Failed to make directory {path}: {err:?}");
            Err(DaemonError::MakeDirectory)
        }
    }
}

/// Write data to the provided path
async fn write_file(bytes: &[u8], path: &str) -> Result<(), DaemonError> {
    match write(path, bytes).await {
        Ok(result) => Ok(result),
        Err(err) => {
            error!("[daemon] Failed to write file {path}: {err:?}");
            Err(DaemonError::WriteFile)
        }
    }
}

#[cfg(test)]
mod tests {
    use crate::utils::config::{
        Daemon, DaemonToml, create_directory, daemon, read_file, server, write_file,
    };
    use std::path::PathBuf;

    #[tokio::test]
    async fn test_read_file() {
        let mut test_location = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
        test_location.push("tests/configs/server.toml");

        let result = read_file(test_location.to_str().unwrap()).await.unwrap();
        assert!(result.len() > 200);
    }

    #[tokio::test]
    #[should_panic(expected = "ReadFile")]
    async fn test_read_no_file() {
        let mut test_location = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
        test_location.push("tests/configs/server123.toml");

        let _ = read_file(test_location.to_str().unwrap()).await.unwrap();
    }

    #[tokio::test]
    async fn test_create_directory() {
        let mut test_location = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
        test_location.push("tmp");

        let _ = create_directory(test_location.to_str().unwrap())
            .await
            .unwrap();
    }

    #[tokio::test]
    async fn test_write_file() {
        let mut test_location = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
        test_location.push("tmp");

        let _ = create_directory(test_location.to_str().unwrap())
            .await
            .unwrap();

        let _ = write_file("test".as_bytes(), "./tmp/test").await.unwrap();
    }

    #[tokio::test]
    #[should_panic(expected = "WriteFile")]
    async fn test_write_file_bad() {
        let mut test_location = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
        test_location.push("tmp");

        let _ = create_directory(test_location.to_str().unwrap())
            .await
            .unwrap();

        let _ = write_file("test".as_bytes(), "tmp").await.unwrap();
    }

    #[tokio::test]
    async fn test_server_config() {
        let mut test_location = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
        test_location.push("tests/configs/server.toml");

        let result = server(test_location.to_str().unwrap(), Some("./tmp/artemis"))
            .await
            .unwrap();

        assert_eq!(result.server.collections, "endpoint/collections");
        assert_eq!(result.server.version, 1);
        assert_eq!(result.server.url, "http://127.0.0.1");
        assert_eq!(result.server.ignore_ssl, false);
        assert_eq!(result.server.ping, "endpoint/ping");
        assert_eq!(result.server.config, "endpoint/config");
        assert_eq!(result.server.enrollment, "endpoint/enroll");
        assert_eq!(result.server.port, 8000);
        assert_eq!(result.server.key, "my key");
    }

    #[tokio::test]
    async fn test_daemon() {
        let mut test_location = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
        test_location.push("./tmp/artemis");

        let alt_path = test_location.to_str().unwrap();
        let mut daemon_toml = DaemonToml {
            daemon: Daemon {
                node_key: String::from("test"),
                collection_path: String::from("./tmp/artemis/collections"),
                log_level: String::from("warn"),
            },
        };

        daemon(&mut daemon_toml, Some(alt_path)).await.unwrap();
        assert_eq!(daemon_toml.daemon.node_key, "test");
        assert_eq!(daemon_toml.daemon.log_level, "warn");
        assert!(
            daemon_toml
                .daemon
                .collection_path
                .contains("./tmp/artemis/collections")
        );
    }
}
