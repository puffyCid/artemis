use super::error::UserAssistError;
use crate::{
    artifacts::os::windows::registry::helper::{get_registry_keys, get_registry_keys_by_ref},
    filesystem::ntfs::{raw_files::get_user_registry_files, setup::setup_ntfs_parser},
    utils::regex_options::create_regex,
};
use common::windows::RegistryData;
use log::error;

pub(crate) struct UserAssistReg {
    pub(crate) regs: Vec<RegistryData>,
    pub(crate) reg_file: String,
}

/// Grab the `UserAssist` data from the Registry based on provided drive letter
pub(crate) fn get_userassist_drive(drive: char) -> Result<Vec<UserAssistReg>, UserAssistError> {
    let user_reg_results = get_user_registry_files(drive);
    let user_hives = match user_reg_results {
        Ok(result) => result,
        Err(err) => {
            error!("[userassist] Could not get user hives: {err:?}");
            return Err(UserAssistError::RegistryFiles);
        }
    };

    let parser_result = setup_ntfs_parser(drive);
    let mut ntfs_parser = match parser_result {
        Ok(result) => result,
        Err(err) => {
            error!("[userassist] Could no create ntfs parser: {err:?}");
            return Err(UserAssistError::UserAssistData);
        }
    };

    let assist_regex =
        create_regex(r".*\\software\\microsoft\\windows\\currentversion\\explorer\\userassist")
            .unwrap(); // always valid
    let start_path = "";

    let mut userassist_data: Vec<UserAssistReg> = Vec::new();
    for hive in user_hives {
        // UserAssist only exists in NTUSER.DAT hives
        if hive.filename != "NTUSER.DAT" {
            continue;
        }
        let mut assist_entry = UserAssistReg {
            regs: Vec::new(),
            reg_file: hive.full_path,
        };
        let reg_results = get_registry_keys_by_ref(
            start_path,
            &assist_regex,
            hive.reg_reference,
            &mut ntfs_parser,
        );
        match reg_results {
            Ok(result) => {
                assist_entry.regs.append(&mut filter_userassist(&result));
                userassist_data.push(assist_entry);
            }
            Err(err) => {
                error!(
                    "[userassist] Could not parse {}: {err:?}",
                    assist_entry.reg_file
                );
            }
        }
    }
    Ok(userassist_data)
}

/// Parse `UserAssist` at provided path
pub(crate) fn alt_userassist(path: &str) -> Result<Vec<UserAssistReg>, UserAssistError> {
    let start_path = "";
    let assist_regex =
        create_regex(r".*\\software\\microsoft\\windows\\currentversion\\explorer\\userassist")
            .unwrap(); // always valid

    let reg_results = get_registry_keys(start_path, &assist_regex, path);
    let reg_data = match reg_results {
        Ok(results) => results,
        Err(err) => {
            error!("[userassist] Could not parse {path}: {err:?}",);
            return Err(UserAssistError::RegistryFiles);
        }
    };

    let regs = filter_userassist(&reg_data);
    let userassist_result = UserAssistReg {
        regs,
        reg_file: path.to_string(),
    };

    Ok(vec![userassist_result])
}

/// Filter Registry that only contain `Count` in the key name
fn filter_userassist(reg_data: &[RegistryData]) -> Vec<RegistryData> {
    let mut userassist_entries: Vec<RegistryData> = Vec::new();
    for entry in reg_data {
        if entry.name != "Count" {
            continue;
        }
        userassist_entries.push(entry.clone());
    }
    userassist_entries
}

#[cfg(test)]
#[cfg(target_os = "windows")]
mod tests {
    use super::{alt_userassist, get_userassist_drive};
    use crate::{
        artifacts::os::windows::{
            registry::helper::get_registry_keys_by_ref, userassist::registry::filter_userassist,
        },
        filesystem::ntfs::{raw_files::get_user_registry_files, setup::setup_ntfs_parser},
        utils::regex_options::create_regex,
    };
    use std::path::PathBuf;

    #[test]
    fn test_get_userassist_drive() {
        let results = get_userassist_drive(&'C').unwrap();
        assert!(results.len() > 0);
    }

    #[test]
    fn test_filter_userassist() {
        let assist_regex = create_regex("").unwrap(); // always valid
        let start_path = "ROOT\\SOFTWARE\\Microsoft\\Windows\\CurrentVersion\\Explorer\\UserAssist";
        let user_hives = get_user_registry_files(&'C').unwrap();
        let mut ntfs_parser = setup_ntfs_parser(&'C').unwrap();
        for hive in user_hives {
            if hive.filename != "NTUSER.DAT" || hive.full_path.contains("Default") {
                continue;
            }
            let reg_results = get_registry_keys_by_ref(
                start_path,
                &assist_regex,
                &hive.reg_reference,
                &mut ntfs_parser,
            )
            .unwrap();
            let _results = filter_userassist(&reg_results);
        }
    }

    #[test]
    fn test_alt_userassist() {
        let mut test_location = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
        test_location.push("tests\\test_data\\windows\\registry\\win10\\NTUSER.DAT");
        let result = alt_userassist(test_location.to_str().unwrap()).unwrap();
        assert_eq!(result.len(), 1);
    }
}
