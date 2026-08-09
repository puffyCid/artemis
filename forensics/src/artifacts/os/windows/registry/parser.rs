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
    accessor::{
        access::Accessor,
        entry::handle::{EntryKind, GlobMatch},
    },
    artifacts::os::windows::registry::helper::read_registry_handle,
    output::{manager::OutputManager, record::serialize_records_to_stream},
    structs::artifacts::os::windows::RegistryOptions,
    utils::{environment::get_systemdrive, regex_options::create_regex},
};
use common::windows::RegistryData;
use regex::Regex;
use std::collections::HashMap;
use tracing::{error, info};

/// Parameters used for determining what `Registry` data to return
pub(crate) struct Params {
    pub(crate) start_path: String, // Start Path to use when walking the Registry
    pub(crate) path_regex: Regex,  // Any optional key path filtering
    pub(crate) registry_list: Vec<RegistryData>, // Store Registry entries
    pub(crate) key_tracker: Vec<String>, // Track Registry paths as we walk them
    pub(crate) offset_tracker: HashMap<u32, u32>, // Track Registry offsets to prevent infinite loops
    pub(crate) registry_path: String,
}

/// Parse Windows `Registry` files based on provided options
pub(crate) fn parse_registry(
    options: &RegistryOptions,
    manager: &mut OutputManager,
) -> Result<(), RegistryError> {
    let path_regex = user_regex(options.path_regex.as_ref().unwrap_or(&String::new()))?;
    let mut params = Params {
        start_path: String::from(""),
        path_regex,
        registry_list: Vec::new(),
        key_tracker: Vec::new(),
        offset_tracker: HashMap::new(),
        registry_path: String::new(),
    };

    if let Some(path) = &options.alt_file {
        params.registry_path = path.clone();
        return alt_registry(manager, &mut params, options);
    }

    let drive_result = get_systemdrive();
    let drive = match drive_result {
        Ok(result) => result,
        Err(_err) => {
            error!("Could not get systemdrive");
            return Err(RegistryError::SystemDrive);
        }
    };

    if options.user_hives {
        parse_user_hives(drive, manager, &mut params, options)?;
    }

    if options.system_hives {
        parse_default_system_hives(drive, manager, &mut params, options)?;
    }

    Ok(())
}

/// Create Regex based on provided input
fn user_regex(input: &str) -> Result<Regex, RegistryError> {
    let reg_result = create_regex(&input.to_lowercase());
    match reg_result {
        Ok(result) => Ok(result),
        Err(err) => {
            error!("Bad regex: {input}, error: {err:?}");
            Err(RegistryError::Regex)
        }
    }
}

/// Parse useful system hive files. Other hive files include: COMPONENTS, DEFAULT, DRIVERS, BBI, ELAM, userdiff, BCD-Template
fn parse_default_system_hives(
    drive: char,
    manager: &mut OutputManager,
    params: &mut Params,
    options: &RegistryOptions,
) -> Result<(), RegistryError> {
    // We are parsing system hives on live Windows system
    // We need to be explicit to use the NTFS accessor
    let paths = vec![
        format!("ntfs:{drive}:\\Windows\\System32\\config\\SOFTWARE"),
        format!("ntfs:{drive}:\\Windows\\System32\\config\\SYSTEM"),
        format!("ntfs:{drive}:\\Windows\\System32\\config\\SAM"),
        format!("ntfs:{drive}:\\Windows\\System32\\config\\SECURITY"),
    ];

    for path in paths {
        params.registry_path = path;
        let result = parse_registry_file(manager, params, options);
        match result {
            Ok(_) => {}
            Err(err) => {
                error!(
                    "Could not parse System Registry file: {}, error: {err:?}",
                    params.registry_path
                );
            }
        }
    }

    Ok(())
}

/// Read `Registry` files from provided alternative path
fn alt_registry(
    manager: &mut OutputManager,
    params: &mut Params,
    options: &RegistryOptions,
) -> Result<(), RegistryError> {
    let mut accessor = Accessor::with_defaults();
    let reg_paths = match accessor.globfs(&params.registry_path) {
        Ok(results) => results,
        Err(err) => {
            error!(
                "Could not glob registry files at {}: {err:?}",
                params.registry_path
            );
            return Err(RegistryError::ReadRegistry);
        }
    };
    for reg_path in reg_paths {
        if reg_path.meta.kind != EntryKind::File {
            continue;
        }

        let Some(handle) = reg_path.handle.as_file() else {
            continue;
        };

        info!("Reading registry file '{}'", handle.display_path());
        let bytes = read_registry_handle(handle)?;
        let _ = parse_registry_data(&bytes, manager, params, options);
    }

    Ok(())
}

/// Parse a provided `Registry` file and output the results
fn parse_registry_file(
    manager: &mut OutputManager,
    params: &mut Params,
    options: &RegistryOptions,
) -> Result<(), RegistryError> {
    let bytes = read_registry(&params.registry_path)?;
    parse_registry_data(&bytes, manager, params, options)
}

/// Parse the user `Registry` hives (NTUSER.DAT and UsrClass.dat)
fn parse_user_hives(
    drive: char,
    manager: &mut OutputManager,
    params: &mut Params,
    options: &RegistryOptions,
) -> Result<(), RegistryError> {
    let user_hives = user_registry_files(drive)?;
    let mut accessor = Accessor::with_defaults();
    for reg_path in user_hives {
        if reg_path.meta.kind != EntryKind::File {
            continue;
        }

        let Some(handle) = reg_path.handle.as_file() else {
            continue;
        };

        info!("Reading user Registry file '{}'", handle.display_path());

        let bytes = match accessor.read_file_handle(handle) {
            Ok(results) => results,
            Err(err) => {
                error!(
                    "Failed to read Registry file {}: {err:?}",
                    handle.display_path()
                );
                continue;
            }
        };

        params.registry_path = handle.display_path();
        let _ = parse_registry_data(&bytes, manager, params, options);
    }

    Ok(())
}

/// Parse and output `Registry` data
fn parse_registry_data(
    bytes: &[u8],
    manager: &mut OutputManager,
    params: &mut Params,
    options: &RegistryOptions,
) -> Result<(), RegistryError> {
    let reg_results = parse_raw_registry(bytes, params, &mut Some(manager), Some(options));
    let entries = match reg_results {
        Ok((_, results)) => results,
        Err(_err) => {
            error!("Failed to parse Registry file: {}", params.registry_path);
            return Err(RegistryError::Parser);
        }
    };

    let artifact_name = "registry";
    let mut records = match serialize_records_to_stream(entries) {
        Ok(result) => result,
        Err(err) => {
            error!(
                "Failed to serialize Registry file {}: {err:?}",
                params.registry_path
            );
            return Err(RegistryError::Serialize);
        }
    };

    if let Err(err) = manager.write_artifact(artifact_name, options, &mut records) {
        error!(
            "Failed to output data for {}, error: {err:?}",
            params.registry_path
        );

        return Err(RegistryError::Output);
    }

    Ok(())
}

/// Glob for user Registry files
pub(crate) fn user_registry_files(drive: char) -> Result<Vec<GlobMatch>, RegistryError> {
    // Registry filenames are case insensitive
    // We are parsing user hives on live Windows system
    // We need to be explicit to use the NTFS accessor
    let ntuser_path = format!("ntfs:{drive}:\\Users\\*\\[nN][tT][uU][sS][eE][rR].[dD][aA][tT]");
    let usrclass_path = format!(
        "ntfs:{drive}:\\Users\\*\\AppData\\Local\\Microsoft\\Windows\\[uU][sS][rR][cC][lL][aA][sS][sS].[dD][aA][tT]"
    );
    let mut accessor = Accessor::with_defaults();

    let mut paths = Vec::new();

    let mut reg_paths = match accessor.globfs(&ntuser_path) {
        Ok(results) => results,
        Err(err) => {
            error!("Could not glob NTUSER.dat files {ntuser_path}: {err:?}");
            return Err(RegistryError::GetUserHives);
        }
    };

    paths.append(&mut reg_paths);
    let mut reg_paths = match accessor.globfs(&usrclass_path) {
        Ok(results) => results,
        Err(err) => {
            error!("Could not glob UsrClass.dat files {usrclass_path}: {err:?}");
            return Err(RegistryError::GetUserHives);
        }
    };
    paths.append(&mut reg_paths);

    Ok(paths)
}

#[cfg(test)]
#[cfg(target_os = "windows")]
mod tests {
    use crate::artifacts::os::windows::registry::error::RegistryError;
    use crate::artifacts::os::windows::registry::parser::{
        Params, parse_default_system_hives, parse_registry, parse_registry_data,
        parse_registry_file, parse_user_hives, user_registry_files,
    };
    use crate::structs::toml::{OutputConfig, OutputDestination, OutputFormat};
    use crate::{
        artifacts::os::windows::registry::parser::user_regex, output::manager::OutputManager,
        structs::artifacts::os::windows::RegistryOptions,
    };
    use regex::Regex;
    use std::{collections::HashMap, path::PathBuf};

    fn output_options(name: &str, directory: &str, compress: bool) -> OutputManager {
        let config = OutputConfig {
            name: name.to_string(),
            directory: PathBuf::from(directory),
            format: OutputFormat::Jsonl,
            compress,
            endpoint_id: String::from("abcd"),
            destination: OutputDestination::Local,
            ..Default::default()
        };
        OutputManager::new(config).unwrap()
    }

    #[test]
    fn test_parse_user_hives() {
        let mut output = output_options("reg_temp", "./tmp", true);
        let options = RegistryOptions {
            user_hives: true,
            system_hives: false,
            alt_file: None,
            path_regex: None,
        };
        let mut params = Params {
            start_path: String::from("ROOT"),
            path_regex: Regex::new("").unwrap(),
            registry_list: Vec::new(),
            key_tracker: Vec::new(),
            offset_tracker: HashMap::new(),
            registry_path: String::new(),
        };
        parse_user_hives('C', &mut output, &mut params, &options).unwrap();
    }

    #[test]
    fn test_parse_default_system_hives() {
        let mut output = output_options("reg_temp", "./tmp", true);
        let options = RegistryOptions {
            user_hives: false,
            system_hives: true,
            alt_file: None,
            path_regex: None,
        };
        let mut params = Params {
            start_path: String::from("ROOT"),
            path_regex: Regex::new("").unwrap(),
            registry_list: Vec::new(),
            key_tracker: Vec::new(),
            offset_tracker: HashMap::new(),
            registry_path: String::new(),
        };
        parse_default_system_hives('C', &mut output, &mut params, &options).unwrap();
    }

    #[test]
    fn test_parse_all_users_typed_paths() {
        let mut output = output_options("reg_temp", "./tmp", false);
        let options = RegistryOptions {
            user_hives: true,
            system_hives: false,
            alt_file: None,
            path_regex: None,
        };
        let mut params = Params {
            start_path: String::from("ROOT\\SOFTWARE\\Microsoft\\"),
            path_regex: Regex::new(r".*\\TypedPaths").unwrap(),
            registry_list: Vec::new(),
            key_tracker: Vec::new(),
            offset_tracker: HashMap::new(),
            registry_path: String::new(),
        };
        parse_user_hives('C', &mut output, &mut params, &options).unwrap();
    }

    #[test]
    fn test_parse_system_run_key() {
        let mut output = output_options("reg_temp", "./tmp", false);
        let options = RegistryOptions {
            user_hives: false,
            system_hives: true,
            alt_file: None,
            path_regex: None,
        };
        let mut params = Params {
            start_path: String::from("ROOT\\Microsoft\\Windows\\CurrentVersion\\Run"),
            path_regex: Regex::new("").unwrap(),
            registry_list: Vec::new(),
            key_tracker: Vec::new(),
            offset_tracker: HashMap::new(),
            registry_path: String::new(),
        };
        parse_default_system_hives('C', &mut output, &mut params, &options).unwrap();
    }

    #[test]
    fn test_parse_registry() {
        let mut output = output_options("reg_temp", "./tmp", true);

        let reg_options = RegistryOptions {
            user_hives: true,
            system_hives: false,
            alt_file: None,
            path_regex: None,
        };
        parse_registry(&reg_options, &mut output).unwrap();
    }

    #[test]
    fn test_parse_registry_file() {
        let mut output = output_options("reg_temp", "./tmp", false);
        let mut test_location = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
        test_location.push("tests\\test_data\\windows\\registry\\win10\\NTUSER.DAT");
        let options = RegistryOptions {
            user_hives: false,
            system_hives: false,
            alt_file: None,
            path_regex: None,
        };
        let mut params = Params {
            start_path: String::from(""),
            path_regex: Regex::new("").unwrap(),
            registry_list: Vec::new(),
            key_tracker: Vec::new(),
            offset_tracker: HashMap::new(),
            registry_path: test_location.to_str().unwrap().to_string(),
        };
        parse_registry_file(&mut output, &mut params, &options).unwrap();
    }

    #[test]
    fn test_user_regex() {
        let reg = String::from(r".*");
        let regex = user_regex(&reg).unwrap();
        assert_eq!(regex.as_str(), ".*");
    }

    #[test]
    fn test_user_registry_files() {
        let result = user_registry_files('C').unwrap();

        // Should at least have three (3). User (NTUSER and UsrClass), Default (NTUSER)
        assert!(result.len() >= 3);
        let mut default = false;
        for entry in result {
            if entry.meta.display_path.contains("Default") {
                default = true;
            }
        }
        assert_eq!(default, true)
    }

    #[test]
    fn test_parse_registry_data() {
        let mut output = output_options("reg_temp", "./tmp", true);

        let reg_options = RegistryOptions {
            user_hives: true,
            system_hives: false,
            alt_file: None,
            path_regex: None,
        };

        let mut params = Params {
            start_path: String::from(""),
            path_regex: Regex::new("").unwrap(),
            registry_list: Vec::new(),
            key_tracker: Vec::new(),
            offset_tracker: HashMap::new(),
            registry_path: String::new(),
        };
        let err = parse_registry_data(&[], &mut output, &mut params, &reg_options).unwrap_err();
        assert_eq!(err, RegistryError::Parser);
    }
}
