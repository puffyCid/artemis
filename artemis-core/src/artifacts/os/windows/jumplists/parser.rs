use super::{error::JumplistError, jumplist::JumplistEntry};
use crate::{
    filesystem::metadata::glob_paths, structs::artifacts::os::windows::JumplistsOptions,
    utils::environment::get_systemdrive,
};
use log::error;

/// Grab `Jumplists` based on provided options
pub(crate) fn grab_jumplists(
    options: &JumplistsOptions,
) -> Result<Vec<JumplistEntry>, JumplistError> {
    let drive = if let Some(alt) = options.alt_drive {
        alt
    } else {
        let systemdrive_result = get_systemdrive();
        let systemdrive = match systemdrive_result {
            Ok(result) => result,
            Err(err) => {
                error!("[jumplist] Could not get systemdrive: {err:?}");
                return Err(JumplistError::Systemdrive);
            }
        };
        systemdrive
    };

    let path = format!(
        "{drive}:\\Users\\*\\AppData\\Roaming\\Microsoft\\Windows\\Recent\\*Destinations\\*"
    );

    let glob_results = glob_paths(&path);
    let glob_paths = match glob_results {
        Ok(result) => result,
        Err(err) => {
            error!("[jumplist] Could not glob jumplist paths {path}: {err:?}");
            return Err(JumplistError::ReadFile);
        }
    };

    JumplistEntry::get_jumplists(&glob_paths)
}

#[cfg(test)]
mod tests {
    use super::grab_jumplists;
    use crate::structs::artifacts::os::windows::JumplistsOptions;

    #[test]
    fn test_grab_jumplists() {
        let options = JumplistsOptions { alt_drive: None };
        let _ = grab_jumplists(&options).unwrap();
    }
}
