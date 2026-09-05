/**
 * Shellbags track what directories the user has browsed via Explorer.
 * They are stored in the `ShellItem` format
 *
 * Other parsers:
 *   `https://f001.backblazeb2.com/file/EricZimmermanTools/ShellBagsExplorer.zip`
 *   `https://github.com/Velocidex/velociraptor`
 */
use super::error::ShellbagError;
use crate::{
    accessor::{
        access::Accessor,
        entry::handle::{EntryKind, GlobMatch},
    },
    artifacts::os::windows::{
        registry::{helper::get_registry_keys_handle, parser::user_registry_files},
        shellitems::items::parse_encoded_shellitem,
    },
    structs::artifacts::os::windows::ShellbagsOptions,
    utils::{
        environment::{get_clsids, get_systemdrive},
        regex_options::create_regex,
    },
};
use common::windows::{RegistryData, ShellItem, ShellType};
use serde::Serialize;
use serde_json::Value;
use std::collections::HashMap;
use tracing::{error, info};

#[derive(Debug, Clone, Serialize, Default)]
pub(crate) struct Shellbag {
    pub(crate) path: String,
    pub(crate) created: String,
    pub(crate) modified: String,
    pub(crate) accessed: String,
    pub(crate) mft_entry: u64,
    pub(crate) mft_sequence: u16,
    pub(crate) shell_type: ShellType,
    pub(crate) resolve_path: String,
    pub(crate) reg_modified: String,
    pub(crate) reg_file: String,
    pub(crate) reg_path: String,
    pub(crate) evidence: String,
    pub(crate) stores: Vec<HashMap<String, Value>>,
}

/// Get Windows `Shellbags` for all users based on optional drive, otherwise default drive letter is used
pub(crate) fn grab_shellbags(options: &ShellbagsOptions) -> Result<Vec<Shellbag>, ShellbagError> {
    if let Some(path) = &options.alt_file {
        return alt_shellbags(path, options.resolve_guids);
    }

    let drive_result = get_systemdrive();
    let drive = match drive_result {
        Ok(result) => result,
        Err(err) => {
            error!("Could not get default system drive letter: {err:?}");
            return Err(ShellbagError::DefaultDrive);
        }
    };

    parse_shellbags(drive, options.resolve_guids)
}

/// Parse `Shellbags` associated with provided alternative path
fn alt_shellbags(pattern: &str, resolve_guids: bool) -> Result<Vec<Shellbag>, ShellbagError> {
    let mut accessor = Accessor::with_defaults();
    let paths = match accessor.globfs(pattern) {
        Ok(results) => results,
        Err(err) => {
            error!("Could not glob {pattern} for Shellbags files: {err:?}");
            return Err(ShellbagError::GetRegistryData);
        }
    };

    extract_registry_shellbags(paths, resolve_guids)
}

#[derive(Debug)]
struct RegInfo {
    reg_path: String,
    bagkey: String,
    bagmru: String,
    reg_file: String,
    reg_file_path: String,
    last_modified: String,
}

/**
 * `Shellbags` are stored in user Registry files
 * Get all user hives based on drive letter
 * Parse each user registry file for `ShellItem` data
 * Parse the `ShellItem` data and reconstruct browsed directories
 */
fn parse_shellbags(drive: char, resolve_guids: bool) -> Result<Vec<Shellbag>, ShellbagError> {
    let paths = match user_registry_files(drive) {
        Ok(result) => result,
        Err(err) => {
            error!("Could not get user hives: {err:?}");
            return Err(ShellbagError::GetRegistryData);
        }
    };

    extract_registry_shellbags(paths, resolve_guids)
}

/// Extract `Shellbag` `Registry` keys
fn extract_registry_shellbags(
    paths: Vec<GlobMatch>,
    resolve_guids: bool,
) -> Result<Vec<Shellbag>, ShellbagError> {
    let clsids = if resolve_guids {
        get_clsids().unwrap_or_default()
    } else {
        HashMap::new()
    };

    let mut shellbags = Vec::new();
    for hive in paths {
        if hive.meta.kind != EntryKind::File {
            continue;
        }

        let Some(handle) = hive.handle.as_file() else {
            continue;
        };

        info!(
            "Reading ShellBags file '{}'. Resolve GUIDS: '{resolve_guids}'",
            handle.display_path()
        );

        let reg_regex = if handle
            .display_path()
            .to_lowercase()
            .ends_with("usrclass.dat")
        {
            r"local settings\\software\\microsoft\\windows\\shell\\bagmru"
        } else {
            r"software\\microsoft\\windows\\shell\\bagmru"
        };
        let regex = create_regex(reg_regex).unwrap(); // Should always be valid
        let start_path = String::new();

        let shellbag_reg_data = match get_registry_keys_handle(start_path, regex, handle) {
            Ok(result) => result,
            Err(err) => {
                error!("Could not parse {}: {err:?}", handle.display_path());
                continue;
            }
        };

        let mut shell_map = HashMap::new();
        extract_shellbags(
            &shellbag_reg_data,
            &handle.filename(),
            &handle.display_path(),
            &clsids,
            &mut shell_map,
        );

        info!(
            "Got '{}' raw ShellItems for '{}'. Now will build directory paths",
            shell_map.len(),
            handle.display_path()
        );
        save_shellbags(&mut shellbags, &shell_map);
    }

    Ok(shellbags)
}

/// Extract `Shellbag` data from Registry data
fn extract_shellbags(
    shellbags: &[RegistryData],
    reg_filename: &str,
    reg_path: &str,
    clsids: &HashMap<String, String>,
    shell_map: &mut HashMap<String, Shellbag>,
) {
    for entry in shellbags {
        for value in &entry.values {
            // Shellbag Registry value names should always be a number
            // Skip non-number values
            if value.value.parse::<u32>().is_err() {
                continue;
            }
            // Based on hive file, split the Registry key path and get BagMRU key
            let (bagkey_vec, min_length) = if entry.name == "UsrClass.dat" {
                (entry.path.splitn(6, '\\').collect::<Vec<&str>>(), 6)
            } else {
                (entry.path.splitn(5, '\\').collect::<Vec<&str>>(), 5)
            };
            if bagkey_vec.len() < min_length {
                continue;
            }

            // Vec start at 0
            let vec_adjust = 1;
            // Index lookup is safe because we check for minimum size above
            let bagkey = format!("{}\\{}", bagkey_vec[min_length - vec_adjust], value.value);
            let data_result = parse_encoded_shellitem(&value.data);
            let data = match data_result {
                Ok(result) => result,
                Err(err) => {
                    error!(
                        "Could not parse bag data at {} value name: {}: {err:?}",
                        entry.path, value.value
                    );
                    ShellItem {
                        value: String::from("[Failed to parse ShellItem]"),
                        shell_type: ShellType::Unknown,
                        created: String::new(),
                        modified: String::new(),
                        accessed: String::new(),
                        mft_entry: 0,
                        mft_sequence: 0,
                        stores: Vec::new(),
                    }
                }
            };

            let reg_info = RegInfo {
                reg_path: entry.path.clone(),
                bagkey,
                bagmru: bagkey_vec[min_length - vec_adjust].to_string(),
                reg_file: reg_filename.to_string(),
                reg_file_path: reg_path.to_string(),
                last_modified: entry.last_modified.clone(),
            };

            update_shellbags(data, shell_map, clsids, reg_info);
        }
    }
}

/**
* The goal of parsing `Shellbags` is to reconstruct the directories that a user has browsed to.
* Each `ShellItem` is a single directory. Get the parent directory (if any) from our hashmap and append our current `ShellItem` to it
  and insert into our hashmap as new entry
*/
fn update_shellbags(
    shell: ShellItem,
    shell_map: &mut HashMap<String, Shellbag>,
    clsids: &HashMap<String, String>,
    reg_info: RegInfo,
) {
    if let Some(entry) = shell_map.get(&reg_info.bagmru) {
        let path = format!("{}\\{}", entry.path, shell.value);
        let resolve_path = if shell.shell_type == ShellType::RootFolder
            || shell.shell_type == ShellType::Delegate
            || shell.shell_type == ShellType::Variable
            || shell.shell_type == ShellType::Mtp
        {
            // GUID may either be upper or lowercase
            let path = clsids
                .get(&format!("{{{}}}", shell.value))
                .unwrap_or_else(|| {
                    clsids
                        .get(&format!("{{{}}}", shell.value.to_uppercase()))
                        .unwrap_or(&shell.value)
                });
            format!("{}\\{}", entry.resolve_path, path)
        } else {
            format!("{}\\{}", entry.resolve_path, shell.value)
        };

        let bag = Shellbag {
            path,
            created: shell.created,
            modified: shell.modified,
            accessed: shell.accessed,
            mft_entry: shell.mft_entry,
            mft_sequence: shell.mft_sequence,
            shell_type: shell.shell_type,
            resolve_path,
            reg_modified: reg_info.last_modified,
            reg_file: reg_info.reg_file,
            evidence: reg_info.reg_file_path,
            reg_path: reg_info.reg_path,
            stores: shell.stores,
        };

        shell_map.insert(reg_info.bagkey, bag);
        return;
    }

    let mut bag = Shellbag {
        path: shell.value,
        created: shell.created,
        modified: shell.modified,
        accessed: shell.accessed,
        mft_entry: shell.mft_entry,
        mft_sequence: shell.mft_sequence,
        shell_type: shell.shell_type,
        reg_modified: reg_info.last_modified,
        reg_file: reg_info.reg_file,
        evidence: reg_info.reg_file_path,
        reg_path: reg_info.reg_path,
        stores: shell.stores,
        ..Default::default()
    };

    if bag.shell_type == ShellType::RootFolder
        || bag.shell_type == ShellType::Delegate
        || bag.shell_type == ShellType::Variable
        || bag.shell_type == ShellType::Mtp
    {
        // GUID may either be upper or lowercase
        bag.resolve_path = clsids
            .get(&format!("{{{}}}", bag.path))
            .unwrap_or_else(|| {
                clsids
                    .get(&format!("{{{}}}", bag.path.to_uppercase()))
                    .unwrap_or(&bag.path)
            })
            .clone();
    } else {
        bag.resolve_path = bag.path.clone();
    }

    shell_map.insert(reg_info.bagkey, bag);
}

/// Loop through hashmap and store in `Shellbag` structure and append to vec
fn save_shellbags(shellbag_vec: &mut Vec<Shellbag>, shell_map: &HashMap<String, Shellbag>) {
    for entry in shell_map.values() {
        shellbag_vec.push(entry.clone());
    }
}

#[cfg(test)]

mod tests {
    use crate::{
        accessor::{access::Accessor, entry::handle::FileHandle},
        artifacts::os::windows::{
            registry::helper::get_registry_keys_handle,
            shellbags::parser::{
                RegInfo, Shellbag, alt_shellbags, extract_registry_shellbags, extract_shellbags,
                save_shellbags, update_shellbags,
            },
        },
        utils::regex_options::create_regex,
    };
    use common::windows::{ShellItem, ShellType};
    use std::{collections::HashMap, path::PathBuf};

    #[test]
    #[cfg(target_os = "windows")]
    fn test_grab_shellbags() {
        use crate::{
            artifacts::os::windows::shellbags::parser::grab_shellbags,
            structs::artifacts::os::windows::ShellbagsOptions,
        };

        let options = ShellbagsOptions {
            resolve_guids: true,
            alt_file: None,
        };

        let _results = grab_shellbags(&options).unwrap();
    }

    #[test]
    fn test_alt_shellbags() {
        let mut test_location = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
        test_location.push("tests/test_data/windows/registry/win10/NTUSER.DAT");
        let result = alt_shellbags(test_location.to_str().unwrap(), false).unwrap();
        assert_eq!(result.len(), 0);
    }

    #[test]
    fn test_extract_shellbags() {
        let mut test_location = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
        test_location.push("tests/test_data/windows/registry/win10/NTUSER.DAT");
        let regex = create_regex(r"software\\microsoft\\windows\\shell\\bagmru").unwrap();
        let clsids = HashMap::new();
        let handle = FileHandle::host(test_location);

        let start_path = String::new();
        let bags = get_registry_keys_handle(start_path, regex, &handle).unwrap();
        let mut shell_map = HashMap::new();

        extract_shellbags(
            &bags,
            &handle.filename(),
            &handle.display_path(),
            &clsids,
            &mut shell_map,
        );
        assert_eq!(shell_map.len(), 0);
    }

    #[test]
    fn test_extract_registry_shellbags() {
        let mut test_location = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
        test_location.push("tests/test_data/windows/registry/win10/NTUSER.DAT");

        let mut accessor = Accessor::with_defaults();
        let paths = accessor.globfs(test_location.to_str().unwrap()).unwrap();
        let results = extract_registry_shellbags(paths, false).unwrap();
        assert!(results.is_empty());
    }

    #[test]
    #[cfg(target_os = "windows")]
    fn test_parse_shellbags() {
        let drive = 'C';
        let _results = parse_shellbags(drive, false).unwrap();
    }

    #[test]
    fn test_update_shellbags() {
        let item = ShellItem {
            value: String::from("rust is nice"),
            shell_type: ShellType::Directory,
            created: String::new(),
            modified: String::new(),
            accessed: String::new(),
            mft_entry: 0,
            mft_sequence: 0,
            stores: Vec::new(),
        };
        let mut shell_map = HashMap::new();
        let empty_clsids = HashMap::new();
        let reg_info = RegInfo {
            reg_path: String::from("shellbags are complex"),
            bagkey: String::from("shellbags are complex"),
            bagmru: String::from("shellbags are complex"),
            reg_file: String::from("shellbags are complex"),
            reg_file_path: String::from("shellbags are complex"),
            last_modified: String::new(),
        };
        update_shellbags(item, &mut shell_map, &empty_clsids, reg_info);
        assert_eq!(shell_map.len(), 1);
    }

    #[test]
    fn test_save_shellbags() {
        let bag = Shellbag {
            path: String::from("rust is nice"),
            shell_type: ShellType::Directory,
            created: String::new(),
            modified: String::new(),
            accessed: String::new(),
            mft_entry: 0,
            mft_sequence: 0,
            resolve_path: String::from("shellbags are complex"),
            reg_modified: String::new(),
            reg_file: String::from("shellbags are complex"),
            reg_path: String::from("shellbags are complex"),
            evidence: String::from("shellbags are complex"),
            stores: Vec::new(),
        };
        let mut shell_map = HashMap::new();
        let mut shellbag_vec = Vec::new();

        shell_map.insert(String::from("test"), bag);
        save_shellbags(&mut shellbag_vec, &shell_map);
        assert_eq!(shellbag_vec.len(), 1);
    }
}
