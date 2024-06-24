use super::{
    directory::create_dirs,
    error::FileSystemError,
    files::{read_file, write_file},
};
use common::server::config::ArtemisConfig;
use log::error;
use std::str::from_utf8;

/// Read the client TOML config file
pub(crate) async fn read_config(path: &str) -> Result<ArtemisConfig, FileSystemError> {
    let buffer = read_file(path).await?;

    let config_result = toml::from_str(from_utf8(&buffer).unwrap_or_default());
    let config = match config_result {
        Ok(result) => result,
        Err(err) => {
            error!("[client] Failed to parse client config at {path}: {err:?}");
            return Err(FileSystemError::BadToml);
        }
    };

    Ok(config)
}

/// Generate the client storage layout
pub(crate) async fn create_layout(config: &ArtemisConfig) -> Result<(), FileSystemError> {
    let storage = format!("{}/artemis/storage", config.endpoint_server.storage);
    let logs = format!("{}/artemis/logs", config.endpoint_server.storage);

    create_dirs(&storage).await?;
    create_dirs(&logs).await?;

    let config_file = format!("{}/artemis/config.toml", config.endpoint_server.storage);
    let toml_data_result = toml::to_string(&config);
    let toml_data = match toml_data_result {
        Ok(result) => result,
        Err(err) => {
            error!("[client] Failed to convert client config to TOML file: {err:?}");
            return Err(FileSystemError::BadToml);
        }
    };
    write_file(toml_data.as_bytes(), &config_file).await?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use crate::filesystem::config::create_layout;
    use crate::filesystem::config::read_config;
    use std::path::PathBuf;

    #[tokio::test]
    async fn test_read_config() {
        let mut test_location = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
        test_location.push("tests/test_data/client.toml");

        let result = read_config(&test_location.display().to_string())
            .await
            .unwrap();
        assert_eq!(result.enroll_key, "arandomkey");
        assert_eq!(result.endpoint_server.address, "127.0.0.1")
    }

    #[tokio::test]
    async fn test_create_layout() {
        let mut test_location = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
        test_location.push("tests/test_data/client.toml");

        let mut result = read_config(&test_location.display().to_string())
            .await
            .unwrap();

        result.endpoint_id = String::from("anything");

        create_layout(&result).await.unwrap();
    }
}
