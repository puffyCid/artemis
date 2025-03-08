/**
 * Windows `Registry` is a collection of binary files that store Windows configuration settings and OS information
 * There are multiple `Registry` files on a system such as:
 *   SYSTEM
 *   SOFTWARE
 *   SAM
 *   SECURITY
 *   NTUSER.DAT -- One per user
 *   UsrClass.dat -- One per user
 *
 * References for the Registry format:
 *  `https://github.com/msuhanov/regf/blob/master/Windows%20registry%20file%20format%20specification.md`
 *  `https://github.com/libyal/libregf/blob/main/documentation/Windows%20NT%20Registry%20File%20(REGF)%20format.asciidoc#file_types`
 *  `https://binaryforay.blogspot.com/2015/01/registry-hive-basics.html`
 *
 * Other Parsers:
 *  `https://github.com/Velocidex/velociraptor`
 *  `https://ericzimmerman.github.io/RegistryExplorer.zip`
 */
use super::{
    error::RegistryError,
    helper::{parse_raw_registry, read_registry},
};
use crate::{
    artifacts::os::windows::artifacts::output_data,
    filesystem::ntfs::{
        raw_files::{get_user_registry_files, raw_read_by_file_ref},
        setup::setup_ntfs_parser,
    },
    structs::{artifacts::os::windows::RegistryOptions, toml::Output},
    utils::{environment::get_systemdrive, regex_options::create_regex, time::time_now},
};
use common::windows::RegistryData;
use log::error;
use regex::Regex;
use std::collections::HashMap;

/// Parameters used for determining what `Registry` data to return
pub(crate) struct Params {
    pub(crate) start_path: String, // Start Path to use when walking the Registry
    pub(crate) path_regex: Regex,  // Any optional key path filtering
    pub(crate) registry_list: Vec<RegistryData>, // Store Registry entries
    pub(crate) key_tracker: Vec<String>, // Track Registry paths as we walk them
    pub(crate) offset_tracker: HashMap<u32, u32>, // Track Registry offsets to prevent infinite loops
    pub(crate) filter: bool,
    pub(crate) registry_path: String,
}

/// Parse Windows `Registry` files based on provided options
pub(crate) fn parse_registry(
    options: &RegistryOptions,
    output: &mut Output,
    filter: &bool,
) -> Result<(), RegistryError> {
    let path_regex = user_regex(options.path_regex.as_ref().unwrap_or(&String::new()))?;
    let mut params = Params {
        start_path: String::from(""),
        path_regex,
        registry_list: Vec::new(),
        key_tracker: Vec::new(),
        offset_tracker: HashMap::new(),
        filter: *filter,
        registry_path: String::new(),
    };

    if let Some(path) = &options.alt_file {
        params.registry_path = path.to_string();
        return parse_registry_file(output, &mut params);
    }

    let drive_result = get_systemdrive();
    let drive = match drive_result {
        Ok(result) => result,
        Err(_err) => {
            error!("[registry] Could not get systemdrive");
            return Err(RegistryError::SystemDrive);
        }
    };

    if options.user_hives {
        parse_user_hives(&drive, output, &mut params)?;
    }

    if options.system_hives {
        parse_default_system_hives(&drive, output, &mut params)?;
    }

    Ok(())
}

/// Create Regex based on provided input
fn user_regex(input: &str) -> Result<Regex, RegistryError> {
    let reg_result = create_regex(&input.to_lowercase());
    match reg_result {
        Ok(result) => Ok(result),
        Err(err) => {
            error!("[registry] Bad regex: {input}, error: {err:?}");
            Err(RegistryError::Regex)
        }
    }
}

/// Parse useful system hive files. Other hive files include: COMPONENTS, DEFAULT, DRIVERS, BBI, ELAM, userdiff, BCD-Template
fn parse_default_system_hives(
    drive: &char,
    output: &mut Output,
    params: &mut Params,
) -> Result<(), RegistryError> {
    let paths = vec![
        format!("{drive}:\\Windows\\System32\\config\\SOFTWARE"),
        format!("{drive}:\\Windows\\System32\\config\\SYSTEM"),
        format!("{drive}:\\Windows\\System32\\config\\SAM"),
        format!("{drive}:\\Windows\\System32\\config\\SECURITY"),
    ];

    for path in paths {
        params.registry_path = path;
        let result = parse_registry_file(output, params);
        match result {
            Ok(_) => {}
            Err(err) => {
                error!(
                    "[registry] Could not parse System Registry file: {}, error: {err:?}",
                    params.registry_path
                );
            }
        }
    }
    Ok(())
}

/// Parse a provided `Registry` file and output the results
fn parse_registry_file(output: &mut Output, params: &mut Params) -> Result<(), RegistryError> {
    let start_time = time_now();

    let buffer = read_registry(&params.registry_path)?;
    let reg_results = parse_raw_registry(&buffer, params);
    let reg_data = match reg_results {
        Ok((_, results)) => results,
        Err(_err) => {
            error!(
                "[registry] Failed to parse Registry file: {}",
                params.registry_path
            );
            return Err(RegistryError::Parser);
        }
    };

    let serde_data_result = serde_json::to_value(&reg_data);
    let mut serde_data = match serde_data_result {
        Ok(results) => results,
        Err(err) => {
            error!(
                "[registry] Failed to serialize Registry file {}: {err:?}",
                params.registry_path
            );
            return Err(RegistryError::Serialize);
        }
    };

    let result = output_data(
        &mut serde_data,
        "registry",
        output,
        &start_time,
        &params.filter,
    );
    match result {
        Ok(_) => Ok(()),
        Err(err) => {
            error!(
                "[registry] Failed to output data for {}, error: {err:?}",
                params.registry_path
            );
            Err(RegistryError::Output)
        }
    }
}

/// Parse the user `Registry` hives (NTUSER.DAT and UsrClass.dat)
fn parse_user_hives(
    drive: &char,
    output: &mut Output,
    params: &mut Params,
) -> Result<(), RegistryError> {
    let user_hives_results = get_user_registry_files(drive);
    let user_hives = match user_hives_results {
        Ok(results) => results,
        Err(err) => {
            error!("[registry] Failed to get user registry files: {err:?}");
            return Err(RegistryError::GetUserHives);
        }
    };
    let ntfs_parser_result = setup_ntfs_parser(drive);
    let mut ntfs_parser = match ntfs_parser_result {
        Ok(result) => result,
        Err(err) => {
            error!("[registry] Could not setup NTFS parser: {err:?}");
            return Err(RegistryError::NtfsSetup);
        }
    };

    let start_time = time_now();

    for path in user_hives {
        let buffer_result =
            raw_read_by_file_ref(&path.reg_reference, &ntfs_parser.ntfs, &mut ntfs_parser.fs);
        let buffer = match buffer_result {
            Ok(result) => result,
            Err(err) => {
                error!(
                    "[registry] Failed to read Registry file: {}, error: {err:?}",
                    path.full_path
                );
                continue;
            }
        };

        params.registry_path = path.full_path;

        let reg_results = parse_raw_registry(&buffer, params);
        let reg_data = match reg_results {
            Ok((_, results)) => results,
            Err(_err) => {
                error!(
                    "[registry] Failed to parse Registry file: {}",
                    params.registry_path
                );
                continue;
            }
        };

        let serde_data_result = serde_json::to_value(&reg_data);
        let mut serde_data = match serde_data_result {
            Ok(results) => results,
            Err(err) => {
                error!(
                    "[registry] Failed to serialize User Registry file {}: {err:?}",
                    params.registry_path
                );
                continue;
            }
        };

        let result = output_data(
            &mut serde_data,
            "registry",
            output,
            &start_time,
            &params.filter,
        );
        match result {
            Ok(_) => {}
            Err(err) => {
                error!(
                    "[registry] Failed to output data for {}, error: {err:?}",
                    params.registry_path
                );
            }
        }
    }
    Ok(())
}

#[cfg(test)]
#[cfg(target_os = "windows")]
mod tests {
    use super::{
        Params, parse_default_system_hives, parse_registry, parse_registry_file, parse_user_hives,
    };
    use crate::{
        artifacts::os::windows::registry::parser::user_regex,
        structs::artifacts::os::windows::RegistryOptions, structs::toml::Output,
    };
    use regex::Regex;
    use std::{collections::HashMap, path::PathBuf};

    fn output_options(name: &str, output: &str, directory: &str, compress: bool) -> Output {
        Output {
            name: name.to_string(),
            directory: directory.to_string(),
            format: String::from("jsonl"),
            compress,
            url: Some(String::new()),
            api_key: Some(String::new()),
            endpoint_id: String::from("abcd"),
            collection_id: 0,
            output: output.to_string(),
            filter_name: None,
            filter_script: None,
            logging: None,
        }
    }
    #[test]
    fn test_parse_user_hives() {
        let mut output = output_options("reg_temp", "local", "./tmp", true);
        let mut params = Params {
            start_path: String::from("ROOT"),
            path_regex: Regex::new("").unwrap(),
            registry_list: Vec::new(),
            key_tracker: Vec::new(),
            offset_tracker: HashMap::new(),
            filter: false,
            registry_path: String::new(),
        };
        parse_user_hives(&'C', &mut output, &mut params).unwrap();
    }

    #[test]
    fn test_parse_default_system_hives() {
        let mut output = output_options("reg_temp", "local", "./tmp", true);
        let mut params = Params {
            start_path: String::from("ROOT"),
            path_regex: Regex::new("").unwrap(),
            registry_list: Vec::new(),
            key_tracker: Vec::new(),
            offset_tracker: HashMap::new(),
            filter: false,
            registry_path: String::new(),
        };
        parse_default_system_hives(&'C', &mut output, &mut params).unwrap();
    }

    #[test]
    fn test_parse_all_users_typed_paths() {
        let mut output = output_options("reg_temp", "local", "./tmp", false);
        let mut params = Params {
            start_path: String::from("ROOT\\SOFTWARE\\Microsoft\\"),
            path_regex: Regex::new(r".*\\TypedPaths").unwrap(),
            registry_list: Vec::new(),
            key_tracker: Vec::new(),
            offset_tracker: HashMap::new(),
            filter: false,
            registry_path: String::new(),
        };
        parse_user_hives(&'C', &mut output, &mut params).unwrap();
    }

    #[test]
    fn test_parse_system_run_key() {
        let mut output = output_options("reg_temp", "local", "./tmp", false);
        let mut params = Params {
            start_path: String::from("ROOT\\Microsoft\\Windows\\CurrentVersion\\Run"),
            path_regex: Regex::new("").unwrap(),
            registry_list: Vec::new(),
            key_tracker: Vec::new(),
            offset_tracker: HashMap::new(),
            filter: false,
            registry_path: String::new(),
        };
        parse_default_system_hives(&'C', &mut output, &mut params).unwrap();
    }

    #[test]
    fn test_parse_registry() {
        let mut output = output_options("reg_temp", "local", "./tmp", true);

        let reg_options = RegistryOptions {
            user_hives: true,
            system_hives: false,
            alt_file: None,
            path_regex: None,
        };
        parse_registry(&reg_options, &mut output, &false).unwrap();
    }

    #[test]
    fn test_parse_registry_file() {
        let mut output = output_options("reg_temp", "local", "./tmp", false);
        let mut test_location = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
        test_location.push("tests\\test_data\\windows\\registry\\win10\\NTUSER.DAT");
        let mut params = Params {
            start_path: String::from(""),
            path_regex: Regex::new("").unwrap(),
            registry_list: Vec::new(),
            key_tracker: Vec::new(),
            offset_tracker: HashMap::new(),
            filter: false,
            registry_path: test_location.to_str().unwrap().to_string(),
        };
        parse_registry_file(&mut output, &mut params).unwrap();
    }

    #[test]
    fn test_user_regex() {
        let reg = String::from(r".*");
        let regex = user_regex(&reg).unwrap();
        assert_eq!(regex.as_str(), ".*");
    }
}
