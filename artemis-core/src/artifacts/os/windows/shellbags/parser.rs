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
    artifacts::os::windows::{
        registry::helper::get_registry_keys,
        shellitems::items::{ShellItem, ShellType},
    },
    filesystem::ntfs::raw_files::get_user_registry_files,
    structs::artifacts::os::windows::ShellbagsOptions,
    utils::{
        environment::{get_clsids, get_systemdrive},
        regex_options::create_regex,
    },
};
use log::error;
use serde::Serialize;
use std::collections::HashMap;

#[derive(Debug, Clone, Serialize)]
pub(crate) struct Shellbag {
    pub(crate) path: String,
    pub(crate) created: i64,
    pub(crate) modified: i64,
    pub(crate) accessed: i64,
    pub(crate) mft_entry: u64,
    pub(crate) mft_sequence: u16,
    pub(crate) shell_type: ShellType,
    pub(crate) resolve_path: String,
    pub(crate) reg_file: String,
    pub(crate) reg_path: String,
    pub(crate) reg_file_path: String,
}

/// Get Windows `Shellbags` for all users based on optional drive, otherwise default drive letter is used
pub(crate) fn grab_shellbags(options: &ShellbagsOptions) -> Result<Vec<Shellbag>, ShellbagError> {
    if let Some(alt_drive) = options.alt_drive {
        return alt_drive_shellbags(&alt_drive, options.resolve_guids);
    }
    default_shellbags(options.resolve_guids)
}

/// Get the default driver letter and parse the `Shellbags`
fn default_shellbags(resolve_guids: bool) -> Result<Vec<Shellbag>, ShellbagError> {
    let drive_result = get_systemdrive();
    let drive = match drive_result {
        Ok(result) => result,
        Err(err) => {
            error!("[shellbags] Could not get default systemdrive letter: {err:?}");
            return Err(ShellbagError::DefaultDrive);
        }
    };
    parse_shellbags(&drive, resolve_guids)
}

/// Parse `Shellbags` associated with provided alternative driver letter
fn alt_drive_shellbags(drive: &char, resolve_guids: bool) -> Result<Vec<Shellbag>, ShellbagError> {
    parse_shellbags(drive, resolve_guids)
}

#[derive(Debug)]
struct RegInfo {
    reg_path: String,
    bagkey: String,
    bagmru: String,
    reg_file: String,
    reg_file_path: String,
}

/**
 * `Shellbags` are stored in user Registry files
 * Get all user hives based on drive letter
 * Parse each user registry file for `ShellItem` data
 * Parse the `ShellItem` data and reconstruct browsed directories
 */
fn parse_shellbags(drive: &char, resolve_guids: bool) -> Result<Vec<Shellbag>, ShellbagError> {
    let user_hives_result = get_user_registry_files(drive);
    let user_hives = match user_hives_result {
        Ok(result) => result,
        Err(err) => {
            error!("[shellbags] Could not get User Registry data for Shellbags: {err:?}");
            return Err(ShellbagError::GetRegistryData);
        }
    };

    let clsids = if resolve_guids {
        let clsid_results = get_clsids();
        match clsid_results {
            Ok(results) => results,
            Err(_err) => HashMap::new(),
        }
    } else {
        HashMap::new()
    };

    let mut shellbags_vec: Vec<Shellbag> = Vec::new();
    for hive in user_hives {
        let shellbags = if hive.filename == "UsrClass.dat" {
            // UsrClass starts with <SID>_Classes
            let start_path = "";
            let path_regex =
                create_regex(r"local settings\\software\\microsoft\\windows\\shell\\bagmru")
                    .unwrap(); // Should always be valid

            let shellbags_result = get_registry_keys(start_path, &path_regex, &hive.full_path);
            match shellbags_result {
                Ok(result) => result,
                Err(err) => {
                    error!(
                        "[shellbags] Could not parse UsrClass.dat Registry file {}: {err:?}",
                        &hive.full_path
                    );
                    continue;
                }
            }
        } else {
            // Get NTUSER.DAT Shellbags
            let start_path = "";
            let path_regex = create_regex(r"software\\microsoft\\windows\\shell\\bagmru").unwrap(); // Should always be valid

            let shellbags_result = get_registry_keys(start_path, &path_regex, &hive.full_path);
            match shellbags_result {
                Ok(result) => result,
                Err(err) => {
                    error!(
                        "[shellbags] Could not parse NTUSER.DAT Registry file {}: {err:?}",
                        &hive.full_path
                    );
                    continue;
                }
            }
        };

        let mut shell_map: HashMap<String, Shellbag> = HashMap::new();

        for entry in shellbags {
            for value in entry.values {
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
                let bagkey = format!("{}\\{}", bagkey_vec[min_length - vec_adjust], value.value);
                let data_result = ShellItem::parse_encoded_shellitem(&value.data);
                let data = match data_result {
                    Ok(result) => result,
                    Err(err) => {
                        error!(
                            "[shellbags] Could not parse bag data at {} value name: {}: {err:?}",
                            entry.path, value.value
                        );
                        ShellItem {
                            value: String::from("[Failed to parse ShellItem]"),
                            shell_type: ShellType::Unknown,
                            created: 0,
                            modified: 0,
                            accessed: 0,
                            mft_entry: 0,
                            mft_sequence: 0,
                        }
                    }
                };

                let reg_info = RegInfo {
                    reg_path: entry.path.clone(),
                    bagkey,
                    bagmru: bagkey_vec[min_length - vec_adjust].to_string(),
                    reg_file: hive.filename.clone(),
                    reg_file_path: hive.full_path.clone(),
                };

                update_shellbags(&data, &mut shell_map, &clsids, &reg_info);
            }
        }

        save_shellbags(&mut shellbags_vec, &shell_map);
    }
    Ok(shellbags_vec)
}

/**
* The goal of parsing `Shellbags` is to reconstruct the directories that a user has browsed to.
* Each `ShellItem` is a single directory. Get the parent directory (if any) from our hashmap and append our current `ShellItem` to it
  and insert into our hashmap as new entry
*/
fn update_shellbags(
    shell: &ShellItem,
    shell_map: &mut HashMap<String, Shellbag>,
    clsids: &HashMap<String, String>,
    reg_info: &RegInfo,
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
                })
                .clone();
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
            shell_type: shell.shell_type.clone(),
            resolve_path,
            reg_file: reg_info.reg_file.clone(),
            reg_file_path: reg_info.reg_file_path.clone(),
            reg_path: reg_info.reg_path.clone(),
        };

        shell_map.insert(reg_info.bagkey.clone(), bag);
        return;
    }

    let resolve_path = if shell.shell_type == ShellType::RootFolder
        || shell.shell_type == ShellType::Delegate
        || shell.shell_type == ShellType::Variable
        || shell.shell_type == ShellType::Mtp
    {
        // GUID may either be upper or lowercase
        clsids
            .get(&format!("{{{}}}", shell.value))
            .unwrap_or_else(|| {
                clsids
                    .get(&format!("{{{}}}", shell.value.to_uppercase()))
                    .unwrap_or(&shell.value)
            })
            .clone()
    } else {
        shell.value.clone()
    };

    let bag = Shellbag {
        path: shell.value.clone(),
        created: shell.created,
        modified: shell.modified,
        accessed: shell.accessed,
        mft_entry: shell.mft_entry,
        mft_sequence: shell.mft_sequence,
        shell_type: shell.shell_type.clone(),
        resolve_path,
        reg_file: reg_info.reg_file.clone(),
        reg_file_path: reg_info.reg_file_path.clone(),
        reg_path: reg_info.reg_path.clone(),
    };

    shell_map.insert(reg_info.bagkey.clone(), bag);
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
        artifacts::os::windows::{
            shellbags::parser::{
                alt_drive_shellbags, default_shellbags, grab_shellbags, parse_shellbags,
                save_shellbags, update_shellbags, RegInfo, Shellbag,
            },
            shellitems::items::{ShellItem, ShellType},
        },
        structs::artifacts::os::windows::ShellbagsOptions,
    };
    use std::collections::HashMap;

    #[test]
    fn test_default_shellbags() {
        let _result = default_shellbags(true).unwrap();
    }

    #[test]
    fn test_grab_shellbags() {
        let options = ShellbagsOptions {
            resolve_guids: true,
            alt_drive: None,
        };

        let _results = grab_shellbags(&options).unwrap();
    }

    #[test]
    fn test_alt_drive_shellbags() {
        let drive = 'C';
        let _results = alt_drive_shellbags(&drive, false).unwrap();
    }

    #[test]
    fn test_parse_shellbags() {
        let drive = 'C';
        let _results = parse_shellbags(&drive, false).unwrap();
    }

    #[test]
    fn test_update_shellbags() {
        let item = ShellItem {
            value: String::from("rust is nice"),
            shell_type: ShellType::Directory,
            created: 0,
            modified: 0,
            accessed: 0,
            mft_entry: 0,
            mft_sequence: 0,
        };
        let mut shell_map = HashMap::new();
        let empty_clsids = HashMap::new();
        let reg_info = RegInfo {
            reg_path: String::from("shellbags are complex"),
            bagkey: String::from("shellbags are complex"),
            bagmru: String::from("shellbags are complex"),
            reg_file: String::from("shellbags are complex"),
            reg_file_path: String::from("shellbags are complex"),
        };
        update_shellbags(&item, &mut shell_map, &empty_clsids, &reg_info);
        assert_eq!(shell_map.len(), 1);
    }

    #[test]
    fn test_save_shellbags() {
        let bag = Shellbag {
            path: String::from("rust is nice"),
            shell_type: ShellType::Directory,
            created: 0,
            modified: 0,
            accessed: 0,
            mft_entry: 0,
            mft_sequence: 0,
            resolve_path: String::from("shellbags are complex"),
            reg_file: String::from("shellbags are complex"),
            reg_path: String::from("shellbags are complex"),
            reg_file_path: String::from("shellbags are complex"),
        };
        let mut shell_map = HashMap::new();
        let mut shellbag_vec = Vec::new();

        shell_map.insert(String::from("test"), bag);
        save_shellbags(&mut shellbag_vec, &shell_map);
        assert_eq!(shellbag_vec.len(), 1);
    }
}
