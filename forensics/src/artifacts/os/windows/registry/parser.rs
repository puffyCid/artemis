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
use super::error::RegistryError;
use crate::{
    artifacts::os::windows::registry::helper::stream_registry,
    filesystem::ntfs::raw_files::get_user_registry_files,
    structs::{artifacts::os::windows::RegistryOptions, toml::Output},
    utils::{environment::get_systemdrive, regex_options::create_regex},
};
use log::error;
use regex::Regex;
use std::{collections::HashSet, io::BufReader};

/// Parameters used for determining what `Registry` data to return
pub(crate) struct ParamsReader<T: std::io::Seek + std::io::Read> {
    /**Start Path to use when walking the Registry */
    pub(crate) start_path: String,
    /**Regex to limit what keys to return */
    pub(crate) path_regex: Option<Regex>,
    /**Track Registry paths */
    pub(crate) key_tracker: Vec<String>,
    /**Track Registry offsets to prevent infinite loops */
    pub(crate) offset_tracker: HashSet<u32>,
    /**Apply filter to output */
    pub(crate) filter: bool,
    /**Path to the Registry file */
    pub(crate) registry_path: String,
    pub(crate) reader: BufReader<T>,
    pub(crate) offset: u32,
    /**Size of the HBIN data. Typically 4096 */
    pub(crate) size: u32,
    /**Registry minor version. Version 3 and higher have BigData lists */
    pub(crate) minor_version: u32,
}

/// Parse Windows `Registry` files based on provided options
pub(crate) fn parse_registry(
    options: &RegistryOptions,
    output: &mut Output,
    filter: bool,
) -> Result<(), RegistryError> {
    let path_regex = user_regex(options.path_regex.as_ref().unwrap_or(&String::new()))?;

    if let Some(path) = &options.alt_file {
        return stream_registry(path, "", Some(&path_regex), output, filter);
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
        parse_user_hives(drive, output, filter, &path_regex)?;
    }

    if options.system_hives {
        parse_default_system_hives(drive, output, filter, &path_regex)?;
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
    drive: char,
    output: &mut Output,
    filter: bool,
    regex: &Regex,
) -> Result<(), RegistryError> {
    let paths = vec![
        format!("{drive}:\\Windows\\System32\\config\\SOFTWARE"),
        format!("{drive}:\\Windows\\System32\\config\\SYSTEM"),
        format!("{drive}:\\Windows\\System32\\config\\SAM"),
        format!("{drive}:\\Windows\\System32\\config\\SECURITY"),
    ];

    for path in paths {
        let result = parse_registry_file(output, &path, filter, regex);
        match result {
            Ok(_) => {}
            Err(err) => {
                error!("[registry] Could not parse System Registry file: {path}, error: {err:?}",);
            }
        }
    }
    Ok(())
}

/// Parse a provided `Registry` file and output the results
fn parse_registry_file(
    output: &mut Output,
    path: &str,
    filter: bool,
    regex: &Regex,
) -> Result<(), RegistryError> {
    stream_registry(path, "", Some(regex), output, filter)
}

/// Parse the user `Registry` hives (NTUSER.DAT and UsrClass.dat)
fn parse_user_hives(
    drive: char,
    output: &mut Output,
    filter: bool,
    regex: &Regex,
) -> Result<(), RegistryError> {
    let user_hives_results = get_user_registry_files(drive);
    let user_hives = match user_hives_results {
        Ok(results) => results,
        Err(err) => {
            error!("[registry] Failed to get user registry files: {err:?}");
            return Err(RegistryError::GetUserHives);
        }
    };
    for path in user_hives {
        stream_registry(&path.full_path, "", Some(regex), output, filter)?;
    }
    Ok(())
}

#[cfg(test)]
#[cfg(target_os = "windows")]
mod tests {
    use super::{
        parse_default_system_hives, parse_registry, parse_registry_file, parse_user_hives,
    };
    use crate::{
        artifacts::os::windows::registry::parser::user_regex,
        structs::artifacts::os::windows::RegistryOptions, structs::toml::Output,
    };
    use regex::Regex;
    use std::path::PathBuf;

    fn output_options(name: &str, output: &str, directory: &str, compress: bool) -> Output {
        Output {
            name: name.to_string(),
            directory: directory.to_string(),
            format: String::from("jsonl"),
            compress,
            timeline: false,
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
        parse_user_hives('C', &mut output, false, &Regex::new("").unwrap()).unwrap();
    }

    #[test]
    fn test_parse_default_system_hives() {
        let mut output = output_options("reg_temp", "local", "./tmp", true);
        parse_default_system_hives('C', &mut output, false, &Regex::new("").unwrap()).unwrap();
    }

    #[test]
    fn test_parse_all_users_typed_paths() {
        let mut output = output_options("reg_temp", "local", "./tmp", false);
        parse_user_hives('C', &mut output, false, &Regex::new("").unwrap()).unwrap();
    }

    #[test]
    fn test_parse_system_run_key() {
        let mut output = output_options("reg_temp", "local", "./tmp", false);
        parse_default_system_hives('C', &mut output, false, &Regex::new("").unwrap()).unwrap();
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
        parse_registry(&reg_options, &mut output, false).unwrap();
    }

    #[test]
    fn test_parse_registry_file() {
        let mut output = output_options("reg_temp", "local", "./tmp", false);
        let mut test_location = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
        test_location.push("tests\\test_data\\windows\\registry\\win10\\NTUSER.DAT");
        parse_registry_file(
            &mut output,
            test_location.to_str().unwrap(),
            false,
            &Regex::new("").unwrap(),
        )
        .unwrap();
    }

    #[test]
    fn test_user_regex() {
        let reg = String::from(r".*");
        let regex = user_regex(&reg).unwrap();
        assert_eq!(regex.as_str(), ".*");
    }
}
