use crate::{
    error::TomlError,
    filesystem::files::read_file,
    utils::{artemis_toml::ArtemisToml, logging::create_log_file},
};
use log::{error, info};
use simplelog::{Config, WriteLogger};
use std::str::from_utf8;

#[cfg(target_os = "macos")]
use crate::artifacts::macos_collection::macos_collection;
#[cfg(target_os = "windows")]
use crate::artifacts::windows_collection::windows_collection;

/// Parse a TOML file at provided path
pub fn parse_toml_file(path: &str) -> Result<(), TomlError> {
    let buffer_results = read_file(path);
    let buffer = match buffer_results {
        Ok(results) => results,
        Err(_) => {
            return Err(TomlError::NoFile);
        }
    };

    let toml_results = toml::from_str(from_utf8(&buffer).unwrap_or_default());
    let os_target: ArtemisToml = match toml_results {
        Ok(results) => results,
        Err(_) => {
            return Err(TomlError::BadToml);
        }
    };

    toml_data(&os_target, &buffer)?;
    Ok(())
}

/// Parse an already read TOML file
pub fn parse_toml_data(data: &[u8]) -> Result<(), TomlError> {
    let toml_results = toml::from_str(from_utf8(data).unwrap_or_default());
    let os_target: ArtemisToml = match toml_results {
        Ok(results) => results,
        Err(_) => {
            return Err(TomlError::BadToml);
        }
    };
    toml_data(&os_target, data)?;
    Ok(())
}

/// Based on target system collect data based on TOML config
fn toml_data(os_target: &ArtemisToml, toml_data: &[u8]) -> Result<(), TomlError> {
    if let Ok(log_file) = create_log_file(&os_target.output) {
        let _ = WriteLogger::init(log::LevelFilter::Warn, Config::default(), log_file);
    }

    if os_target.system == "macos" {
        #[cfg(target_os = "macos")]
        {
            let result = macos_collection(toml_data);
            match result {
                Ok(_) => info!("[artemis-core] Core parsed macos TOML data"),
                Err(err) => {
                    error!("[artemis-core] Core failed to parse macos TOML data: {err:?}");
                    return Err(TomlError::BadToml);
                }
            }
        }
    } else if os_target.system == "windows" {
        #[cfg(target_os = "windows")]
        {
            let result = windows_collection(toml_data);
            match result {
                Ok(_) => info!("[artemis-core] Core parsed Windows TOML data"),
                Err(err) => {
                    error!("[artemis-core] Core failed to parse Windows TOML data: {err:?}");
                    return Err(TomlError::BadToml);
                }
            }
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::{parse_toml_data, parse_toml_file};
    use crate::{
        core::{toml_data, ArtemisToml},
        filesystem::files::read_file,
        utils::artemis_toml::Output,
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
    #[should_panic(expected = "BadToml")]
    fn test_bad_parse_toml_file() {
        let mut test_location = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
        test_location.push("tests/test_data/malformed_tests/bad.toml");

        let buffer = read_file(&test_location.display().to_string()).unwrap();
        let os_target = ArtemisToml {
            #[cfg(target_os = "macos")]
            system: String::from("macos"),
            #[cfg(target_os = "windows")]
            system: String::from("windows"),
            output: Output {
                name: String::from("core"),
                directory: String::from("tmp"),
                format: String::from("json"),
                compress: false,
                // url: Some(String::new()),
                // port: Some(0),
                // api_key: Some(String::new()),
                // username: Some(String::new()),
                // password: Some(String::new()),
                // generic_keys: Some(Vec::new()),
                endpoint_id: String::from("abcd"),
                collection_id: 0,
                output: String::from("local"),
                filter_name: Some(String::new()),
                filter_script: Some(String::new()),
            },
            artifacts: Vec::new(),
        };
        toml_data(&os_target, &buffer).unwrap();
    }
}
