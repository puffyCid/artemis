use crate::artifacts::collection::collect;
use crate::runtime::run::raw_script;
use crate::{
    error::TomlError,
    filesystem::files::{read_file, read_text_file},
    structs::toml::ArtemisToml,
    utils::logging::create_log_file,
};
use log::{LevelFilter, error, info};
use serde_json::Value;
use simplelog::{Config, SimpleLogger, WriteLogger};

/// Parse a TOML file at provided path
pub async fn parse_toml_file(path: &str) -> Result<(), TomlError> {
    let buffer_results = read_file(path);
    let buffer = match buffer_results {
        Ok(results) => results,
        Err(_) => {
            return Err(TomlError::NoFile);
        }
    };

    let toml_results = ArtemisToml::parse_artemis_toml(&buffer);
    let mut collection = match toml_results {
        Ok(results) => results,
        Err(_) => {
            return Err(TomlError::BadToml);
        }
    };

    artemis_collection(&mut collection).await?;
    Ok(())
}

/// Parse an already read TOML file
pub async fn parse_toml_data(data: &[u8]) -> Result<(), TomlError> {
    let toml_results = ArtemisToml::parse_artemis_toml(data);
    let mut collection = match toml_results {
        Ok(results) => results,
        Err(_) => {
            return Err(TomlError::BadToml);
        }
    };
    artemis_collection(&mut collection).await?;
    Ok(())
}

/// Execute a JavaScript file at provided path
pub fn parse_js_file(path: &str) -> Result<Value, TomlError> {
    let _ = SimpleLogger::init(LevelFilter::Warn, Config::default());
    let code_result = read_text_file(path);
    let script = match code_result {
        Ok(results) => results,
        Err(_) => {
            return Err(TomlError::NoFile);
        }
    };

    let script_result = raw_script(&script);
    if script_result.is_err() {
        error!("[runtime] Failed to execute js file");
        return Err(TomlError::BadJs);
    }

    Ok(script_result.unwrap_or_default())
}

/// Based on target system collect data based on TOML config
pub async fn artemis_collection(collection: &mut ArtemisToml) -> Result<(), TomlError> {
    if let Ok((log_file, level)) = create_log_file(&collection.output) {
        let _ = WriteLogger::init(level, Config::default(), log_file);
    }

    let result = collect(collection).await;
    match result {
        Ok(_) => info!("[forensics] Core parsed TOML data"),
        Err(err) => {
            error!("[forensics] Core failed to parse collection: {err:?}");
            return Err(TomlError::BadToml);
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::{parse_js_file, parse_toml_data, parse_toml_file};
    use crate::{
        core::{ArtemisToml, artemis_collection},
        filesystem::files::read_file,
        structs::toml::Output,
    };
    use std::path::PathBuf;

    #[tokio::test]
    #[cfg(target_os = "macos")]
    #[ignore = "Runs full macos.toml collection"]
    async fn test_parse_macos_toml_file() {
        let mut test_location = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
        test_location.push("tests/test_data/macos.toml");
        parse_toml_file(&test_location.display().to_string())
            .await
            .unwrap();
    }

    #[tokio::test]
    #[cfg(target_os = "windows")]
    #[ignore = "Runs full windows.toml collection"]
    async fn test_parse_windows_toml_file() {
        let mut test_location = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
        test_location.push("tests/test_data/windows.toml");
        parse_toml_file(&test_location.display().to_string())
            .await
            .unwrap();
    }

    #[tokio::test]
    #[cfg(target_os = "linux")]
    #[ignore = "Runs full linux.toml collection"]
    async fn test_parse_linux_toml_file() {
        let mut test_location = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
        test_location.push("tests/test_data/linux.toml");
        parse_toml_file(&test_location.display().to_string())
            .await
            .unwrap();
    }

    #[tokio::test]
    #[cfg(target_os = "windows")]
    async fn test_parse_windows_toml_data() {
        let mut test_location = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
        test_location.push("tests/test_data/windows/processes.toml");

        let buffer = read_file(&test_location.display().to_string()).unwrap();
        parse_toml_data(&buffer).await.unwrap();
    }

    #[tokio::test]
    #[cfg(target_os = "macos")]
    async fn test_parse_macos_toml_data() {
        let mut test_location = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
        test_location.push("tests/test_data/macos/processes.toml");

        let buffer = read_file(&test_location.display().to_string()).unwrap();
        parse_toml_data(&buffer).await.unwrap();
    }

    #[tokio::test]
    #[cfg(target_os = "linux")]
    async fn test_parse_linux_toml_data() {
        let mut test_location = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
        test_location.push("tests/test_data/linux/processes.toml");

        let buffer = read_file(&test_location.display().to_string()).unwrap();
        parse_toml_data(&buffer).await.unwrap();
    }

    #[test]
    fn test_parse_js_file() {
        let mut test_location = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
        test_location.push("tests/test_data/deno_scripts/vanilla.js");
        parse_js_file(&test_location.display().to_string()).unwrap();
    }

    #[tokio::test]
    async fn test_bad_parse_toml_file() {
        let mut collection = ArtemisToml {
            output: Output {
                name: String::from("core"),
                directory: String::from("tmp"),
                format: String::from("json"),
                compress: false,
                timeline: false,
                url: Some(String::new()),
                api_key: Some(String::new()),
                endpoint_id: String::from("abcd"),
                collection_id: 0,
                output: String::from("local"),
                filter_name: Some(String::new()),
                filter_script: Some(String::new()),
                logging: Some(String::new()),
            },
            artifacts: Vec::new(),
        };
        artemis_collection(&mut collection).await.unwrap();
    }
}
