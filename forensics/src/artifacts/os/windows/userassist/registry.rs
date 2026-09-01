use super::error::UserAssistError;
use crate::{
    accessor::{
        access::Accessor,
        entry::handle::{EntryKind, GlobMatch},
    },
    artifacts::os::windows::registry::{
        helper::get_registry_keys_handle, parser::user_registry_files,
    },
    utils::regex_options::create_regex,
};
use common::windows::RegistryData;
use tracing::{error, info};

pub(crate) struct UserAssistReg {
    pub(crate) regs: Vec<RegistryData>,
    pub(crate) reg_file: String,
}

/// Grab the `UserAssist` data from the Registry based on provided drive letter
pub(crate) fn get_userassist_drive(drive: char) -> Result<Vec<UserAssistReg>, UserAssistError> {
    let paths = match user_registry_files(drive) {
        Ok(result) => result,
        Err(err) => {
            error!("Could not get user hives: {err:?}");
            return Err(UserAssistError::RegistryFiles);
        }
    };

    extract_userassist(paths)
}

/// Parse `UserAssist` at provided input
pub(crate) fn alt_userassist(pattern: &str) -> Result<Vec<UserAssistReg>, UserAssistError> {
    let mut accessor = Accessor::with_defaults();
    let paths = match accessor.globfs(pattern) {
        Ok(results) => results,
        Err(err) => {
            error!("Could not glob {pattern} for UserAssist files: {err:?}");
            return Err(UserAssistError::RegistryFiles);
        }
    };

    extract_userassist(paths)
}

/// Extract `UserAssist` `Registry` keys
fn extract_userassist(paths: Vec<GlobMatch>) -> Result<Vec<UserAssistReg>, UserAssistError> {
    let mut userassist_data = Vec::new();
    for hive in paths {
        // UserAssist only exists in NTUSER.DAT hives
        if hive.meta.kind != EntryKind::File
            || !hive.meta.full_path.to_lowercase().ends_with("ntuser.dat")
        {
            continue;
        }

        let Some(handle) = hive.handle.as_file() else {
            continue;
        };

        info!("Reading UserAssist file '{}'", handle.display_path());

        let assist_regex =
            create_regex(r".*\\software\\microsoft\\windows\\currentversion\\explorer\\userassist")
                .unwrap(); // always valid
        let start_path = String::new();
        let mut assist_entry = UserAssistReg {
            regs: Vec::new(),
            reg_file: handle.display_path(),
        };

        let reg_results = get_registry_keys_handle(start_path, assist_regex, handle);
        match reg_results {
            Ok(result) => {
                assist_entry.regs.append(&mut filter_userassist(result));
                userassist_data.push(assist_entry);
            }
            Err(err) => {
                error!("Could not parse {}: {err:?}", assist_entry.reg_file);
            }
        }
    }
    Ok(userassist_data)
}

/// Filter Registry that only contain `Count` in the key name
fn filter_userassist(reg_data: Vec<RegistryData>) -> Vec<RegistryData> {
    let mut userassist_entries = Vec::new();
    for entry in reg_data {
        if entry.name != "Count" {
            continue;
        }
        userassist_entries.push(entry);
    }
    userassist_entries
}

#[cfg(test)]
#[cfg(target_os = "windows")]
mod tests {
    use super::{alt_userassist, get_userassist_drive};
    use crate::{
        artifacts::os::windows::{
            registry::{helper::get_registry_keys_handle, parser::user_registry_files},
            userassist::registry::filter_userassist,
        },
        utils::regex_options::create_regex,
    };
    use std::path::PathBuf;

    #[test]
    fn test_get_userassist_drive() {
        let results = get_userassist_drive('C').unwrap();
        assert!(results.len() > 0);
    }

    #[test]
    fn test_filter_userassist() {
        let user_hives = user_registry_files('C').unwrap();
        for hive in user_hives {
            let Some(handle) = hive.handle.as_file() else {
                continue;
            };
            let assist_regex = create_regex("").unwrap(); // always valid
            let start_path = String::from(
                "ROOT\\SOFTWARE\\Microsoft\\Windows\\CurrentVersion\\Explorer\\UserAssist",
            );
            let reg_results = get_registry_keys_handle(start_path, assist_regex, handle).unwrap();
            let _results = filter_userassist(reg_results);
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
