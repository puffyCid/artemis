/**
 * Windows `Jumplists` files track opened files via applications in the Taskbar or Start Menu
 * Jumplists contain `lnk` data and therefore can show evidence of file interaction.
 * There are two (2) types of Jumplist files:
 *
 * - Custom - Files that are pinned to Taskbar applications
 * - Automatic - Files that are not pinned to Taskbar applications
 *
 * References:
 * `https://github.com/libyal/dtformats/blob/main/documentation/Jump%20lists%20format.asciidoc`
 * `https://binaryforay.blogspot.com/2016/02/jump-lists-in-depth-understand-format.html`
 *
 * Other parsers:
 * `https://ericzimmerman.github.io/#!index.md`
 */
use super::{
    error::JumplistError,
    jumplist::{get_jumplist_path, get_jumplists},
};
use crate::{
    filesystem::metadata::glob_paths, structs::artifacts::os::windows::JumplistsOptions,
    utils::environment::get_systemdrive,
};
use common::windows::JumplistEntry;
use log::error;

/// Grab `Jumplists` based on provided options
pub(crate) fn grab_jumplists(
    options: &JumplistsOptions,
) -> Result<Vec<JumplistEntry>, JumplistError> {
    if let Some(file) = &options.alt_file {
        return grab_jumplist_file(file);
    }
    let systemdrive_result = get_systemdrive();
    let drive = match systemdrive_result {
        Ok(result) => result,
        Err(err) => {
            error!("[jumplist] Could not get systemdrive: {err:?}");
            return Err(JumplistError::Systemdrive);
        }
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

    get_jumplists(&glob_paths)
}

/// Parse single `Jumplist` file. Supports both Custom and Automatic `Jumplist` files
fn grab_jumplist_file(path: &str) -> Result<Vec<JumplistEntry>, JumplistError> {
    get_jumplist_path(path)
}

#[cfg(test)]
#[cfg(target_os = "windows")]
mod tests {
    use super::grab_jumplists;
    use crate::{
        artifacts::os::windows::jumplists::parser::grab_jumplist_file,
        structs::artifacts::os::windows::JumplistsOptions,
    };
    use common::windows::ListType;
    use std::path::PathBuf;

    #[test]
    fn test_grab_jumplists() {
        let options = JumplistsOptions { alt_file: None };
        let _ = grab_jumplists(&options).unwrap();
    }

    #[test]
    fn test_grab_jumplist_file() {
        let mut test_location = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
        test_location.push(
            "tests\\test_data\\windows\\jumplists\\win10\\custom\\1ced32d74a95c7bc.customDestinations-ms",
        );

        let result = grab_jumplist_file(&test_location.display().to_string()).unwrap();
        assert_eq!(result.len(), 8);
        assert_eq!(result[0].jumplist_type, ListType::Custom);
        assert_eq!(result[0].lnk_info.created, "2019-10-21T05:48:39.000Z");
        assert_eq!(result[0].lnk_info.modified, "2023-06-14T13:21:20.000Z");
        assert_eq!(result[0].lnk_info.accessed, "2023-08-06T23:53:22.000Z");
        assert_eq!(result[0].lnk_info.file_size, 149416368);
    }
}
