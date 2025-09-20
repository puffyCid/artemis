/**
 * Windows `UserAssist` is a Registry artifact that records applications executed via Windows Explorer.
 * These entries are typically ROT13 encoded (though this can be disabled)
 *
 * References:
 * `https://winreg-kb.readthedocs.io/en/latest/sources/explorer-keys/User-assist.html`
 *
 * Other Parsers:
 * `https://github.com/Velocidex/velociraptor`
 */
use super::{
    assist::parse_userassist_data,
    error::UserAssistError,
    registry::{alt_userassist, get_userassist_drive},
};
use crate::{
    structs::artifacts::os::windows::UserAssistOptions, utils::environment::get_systemdrive,
};
use common::windows::UserAssistEntry;
use log::error;

/// Parse `UserAssist` based on `UserAssistOptions`
pub(crate) fn grab_userassist(
    options: &UserAssistOptions,
) -> Result<Vec<UserAssistEntry>, UserAssistError> {
    let resolve = options.resolve_descriptions.unwrap_or(false);

    if let Some(path) = &options.alt_file {
        let entries = alt_userassist(path)?;
        return parse_userassist_data(&entries, resolve);
    }

    let drive_result = get_systemdrive();
    let drive = match drive_result {
        Ok(result) => result,
        Err(err) => {
            error!("[userassist] Could not determine systemdrive: {err:?}");
            return Err(UserAssistError::DriveLetter);
        }
    };
    parse_userassist(drive, resolve)
}

/// Get `UserAssist` entries for all users in NTUSER.DAT files. Then parse the `UserAssist` data
fn parse_userassist(drive: char, resolve: bool) -> Result<Vec<UserAssistEntry>, UserAssistError> {
    let entries = get_userassist_drive(drive)?;
    parse_userassist_data(&entries, resolve)
}

#[cfg(test)]
#[cfg(target_os = "windows")]
mod tests {
    use crate::{
        artifacts::os::windows::userassist::parser::{grab_userassist, parse_userassist},
        structs::artifacts::os::windows::UserAssistOptions,
    };

    #[test]
    fn test_parse_userassist() {
        let results = parse_userassist('C', false).unwrap();
        if results.is_empty() {
            return;
        }
        assert!(results.len() > 1);
    }

    #[test]
    fn test_grab_userassist() {
        let options = UserAssistOptions {
            alt_file: None,
            resolve_descriptions: None,
        };

        let results = grab_userassist(&options).unwrap();
        if results.is_empty() {
            return;
        }
        assert!(results.len() > 1);
    }
}
