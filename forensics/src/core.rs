use crate::artifacts::collection::collect;
use crate::{
    error::TomlError,
    filesystem::files::{read_file, read_text_file},
    structs::toml::ArtemisToml,
};
use serde_json::Value;
use tracing::{error, info};

#[cfg(feature = "boa")]
use tracing::level_filters::LevelFilter;
#[cfg(feature = "boa")]
use tracing_subscriber::fmt::layer;
#[cfg(feature = "boa")]
use tracing_subscriber::layer::SubscriberExt;
#[cfg(feature = "boa")]
use tracing_subscriber::util::SubscriberInitExt;

#[cfg(feature = "boa")]
use crate::runtime::run::raw_script;
#[cfg(feature = "network")]
use url::Url;

/// Parse a TOML file at provided path
pub fn parse_toml_file(path: &str) -> Result<(), TomlError> {
    #[cfg(feature = "network")]
    if let Ok(url) = Url::parse(path)
        && path.starts_with("http")
    {
        let collection = match ArtemisToml::remote_artemis_toml(url.as_str()) {
            Ok(result) => result,
            Err(_err) => return Err(TomlError::RemoteToml),
        };
        return artemis_collection(collection);
    }

    let buffer_results = read_file(path);
    let buffer = match buffer_results {
        Ok(results) => results,
        Err(_) => {
            return Err(TomlError::NoFile);
        }
    };

    let toml_results = ArtemisToml::parse_artemis_toml(&buffer);
    let collection = match toml_results {
        Ok(results) => results,
        Err(_) => {
            return Err(TomlError::BadToml);
        }
    };

    artemis_collection(collection)
}

/// Parse an already read TOML file
pub fn parse_toml_data(data: &[u8]) -> Result<(), TomlError> {
    let toml_results = ArtemisToml::parse_artemis_toml(data);
    let collection = match toml_results {
        Ok(results) => results,
        Err(_) => {
            return Err(TomlError::BadToml);
        }
    };
    artemis_collection(collection)
}

#[cfg(feature = "boa")]
/// Execute a JavaScript file at provided path
pub fn parse_js_file(path: &str) -> Result<Value, TomlError> {
    let _ = tracing_subscriber::registry()
        .with(
            layer()
                .json()
                .with_file(true)
                .with_line_number(true)
                .flatten_event(true),
        )
        .with(LevelFilter::WARN)
        .try_init();
    let code_result = read_text_file(path);
    let script = match code_result {
        Ok(results) => results,
        Err(_) => {
            return Err(TomlError::NoFile);
        }
    };

    let script_result = raw_script(&script);
    if script_result.is_err() {
        error!("Failed to execute js file");
        return Err(TomlError::BadJs);
    }

    Ok(script_result.unwrap_or_default())
}

/// Based on target system collect data based on TOML config
pub fn artemis_collection(collection: ArtemisToml) -> Result<(), TomlError> {
    let result = collect(collection);
    match result {
        Ok(_) => info!("Parsed TOML data"),
        Err(err) => {
            error!("Failed to parse collection: {err:?}");
            return Err(TomlError::BadToml);
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::{parse_toml_data, parse_toml_file};
    use crate::structs::toml::{OutputConfig, OutputDestination, OutputFormat};
    use crate::{
        core::{ArtemisToml, artemis_collection},
        filesystem::files::read_file,
    };
    use std::path::PathBuf;

    #[test]
    #[cfg(target_os = "macos")]
    #[ignore = "Runs full macos.toml collection"]
    fn test_parse_macos_toml_file() {
        let mut test_location = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
        test_location.push("tests/test_data/macos.toml");
        parse_toml_file(&test_location.display().to_string()).unwrap();
    }

    #[test]
    #[cfg(target_os = "windows")]
    #[ignore = "Runs full windows.toml collection"]
    fn test_parse_windows_toml_file() {
        let mut test_location = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
        test_location.push("tests/test_data/windows.toml");
        parse_toml_file(&test_location.display().to_string()).unwrap();
    }

    #[test]
    #[cfg(target_os = "linux")]
    #[ignore = "Runs full linux.toml collection"]
    fn test_parse_linux_toml_file() {
        let mut test_location = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
        test_location.push("tests/test_data/linux.toml");
        parse_toml_file(&test_location.display().to_string()).unwrap();
    }

    #[test]
    #[cfg(target_os = "windows")]
    fn test_parse_windows_toml_data() {
        let mut test_location = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
        test_location.push("tests/test_data/windows/processes.toml");

        let buffer = read_file(&test_location.display().to_string()).unwrap();
        parse_toml_data(&buffer).unwrap();
    }

    #[test]
    #[cfg(target_os = "macos")]
    fn test_parse_macos_toml_data() {
        let mut test_location = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
        test_location.push("tests/test_data/macos/processes.toml");

        let buffer = read_file(&test_location.display().to_string()).unwrap();
        parse_toml_data(&buffer).unwrap();
    }

    #[test]
    #[cfg(target_os = "linux")]
    fn test_parse_linux_toml_data() {
        let mut test_location = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
        test_location.push("tests/test_data/linux/processes.toml");

        let buffer = read_file(&test_location.display().to_string()).unwrap();
        parse_toml_data(&buffer).unwrap();
    }

    #[test]
    #[cfg(feature = "boa")]
    fn test_parse_js_file() {
        use crate::core::parse_js_file;

        let mut test_location = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
        test_location.push("tests/test_data/deno_scripts/vanilla.js");
        parse_js_file(&test_location.display().to_string()).unwrap();
    }

    #[test]
    fn test_bad_parse_toml_file() {
        let collection = ArtemisToml {
            output: OutputConfig {
                name: String::from("core"),
                directory: PathBuf::from("./tmp"),
                format: OutputFormat::Json,
                endpoint_id: String::from("abcd"),
                destination: OutputDestination::Local,
                ..Default::default()
            },
            marker: None,
            artifacts: Vec::new(),
        };
        artemis_collection(collection).unwrap();
    }
}
