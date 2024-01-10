use super::{error::UtilServerError, filesystem::read_file, uuid::generate_uuid};
use log::error;
use serde::Deserialize;
use std::str::from_utf8;

#[derive(Debug, Deserialize, Clone)]
pub(crate) struct ArtemisConfig {
    pub(crate) metadata: ArtemisInfo,
    pub(crate) enroll_key: String,
    pub(crate) endpoint_server: EndpointServer,
}

#[derive(Debug, Deserialize, Clone)]
pub(crate) struct ArtemisInfo {
    pub(crate) version: String,
    pub(crate) name: String,
    pub(crate) target: String,
}

#[derive(Debug, Deserialize, Clone)]
pub(crate) struct EndpointServer {
    pub(crate) address: String,
    pub(crate) port: u16,
    pub(crate) cert: String,
    pub(crate) storage: String,
}

/// Generate a server TOML config file
pub(crate) fn generate_config() -> ArtemisConfig {
    let metadata = ArtemisInfo {
        version: env!("CARGO_PKG_VERSION").to_string(),
        name: env!("CARGO_PKG_NAME").to_string(),
        target: std::env::var("CARGO_BUILD_TARGET").unwrap_or_default(),
    };

    let endpoint_server = EndpointServer {
        address: String::from("127.0.0.1"),
        port: 8443,
        cert: String::new(),
        storage: String::new(),
    };

    ArtemisConfig {
        metadata,
        enroll_key: generate_uuid(),
        endpoint_server,
    }
}

/// Compare and verify enrollment key against server TOML config
pub(crate) async fn verify_enroll_key(
    key: &str,
    config_path: &str,
) -> Result<bool, UtilServerError> {
    let config = read_config(config_path).await?;

    if key != config.enroll_key {
        return Ok(false);
    }

    Ok(true)
}

/// Return only the storage path from the server config
pub(crate) async fn storage_path(config_path: &str) -> Result<String, UtilServerError> {
    let config = read_config(config_path).await?;
    Ok(config.endpoint_server.storage)
}

/// Read the server TOML config file
pub(crate) async fn read_config(path: &str) -> Result<ArtemisConfig, UtilServerError> {
    let buffer = read_file(path).await?;

    let config_result = toml::from_str(from_utf8(&buffer).unwrap_or_default());
    let config = match config_result {
        Ok(result) => result,
        Err(err) => {
            error!("[server] Failed to parse server config at {path}: {err:?}");
            return Err(UtilServerError::BadToml);
        }
    };

    Ok(config)
}

#[cfg(test)]
mod tests {
    use super::generate_config;
    use crate::utils::config::{read_config, storage_path, verify_enroll_key};
    use std::path::PathBuf;

    #[test]
    fn test_generate_config() {
        let result = generate_config();
        assert!(!result.metadata.name.is_empty());
    }

    #[tokio::test]
    async fn test_read_config() {
        let mut test_location = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
        test_location.push("tests/test_data/server.toml");

        let result = read_config(&test_location.display().to_string())
            .await
            .unwrap();
        assert_eq!(result.enroll_key, "arandomkey");
        assert_eq!(result.endpoint_server.address, "127.0.0.1")
    }

    #[tokio::test]
    async fn test_verify_enroll_key() {
        let mut test_location = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
        test_location.push("tests/test_data/server.toml");

        let result = verify_enroll_key("arandomkey", &test_location.display().to_string())
            .await
            .unwrap();
        assert!(result);
    }

    #[tokio::test]
    async fn test_storage_path() {
        let mut test_location = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
        test_location.push("tests/test_data/server.toml");

        let result = storage_path(&test_location.display().to_string())
            .await
            .unwrap();
        assert_eq!(result, "./tmp");
    }
}
