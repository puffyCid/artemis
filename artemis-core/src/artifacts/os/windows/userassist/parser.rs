/**
 * Windows `UserAssist` is a Registry artifact that records applications executed via Windows Explorer.
 * These entries are typically ROT13 encoded (though this can be disabled)
 *
 * References:
 * `https://winreg-kb.readthedocs.io/en/latest/sources/explorer-keys/User-assist.html`
 *
 * Other Parsers:
 *  `https://github.com/Velocidex/velociraptor`
 */
use super::{assist::UserAssistEntry, error::UserAssistError, registry::get_userassist_data};
use crate::{
    structs::artifacts::os::windows::UserAssistOptions, utils::environment::get_systemdrive,
};
use log::error;

/// Parse `UserAssist` based on `UserAssistOptions`
pub(crate) fn grab_userassist(
    options: &UserAssistOptions,
) -> Result<Vec<UserAssistEntry>, UserAssistError> {
    if let Some(alt_drive) = options.alt_drive {
        return alt_drive_userassist(&alt_drive);
    }
    default_userassist()
}

/// Get `UserAssist` entries using default system drive
fn default_userassist() -> Result<Vec<UserAssistEntry>, UserAssistError> {
    let drive_result = get_systemdrive();
    let drive = match drive_result {
        Ok(result) => result,
        Err(err) => {
            error!("[userassist] Could not determine systemdrive: {err:?}");
            return Err(UserAssistError::DriveLetter);
        }
    };
    parse_userassist(&drive)
}

/// Get `UserAssist` entries on a different system drive
fn alt_drive_userassist(drive: &char) -> Result<Vec<UserAssistEntry>, UserAssistError> {
    parse_userassist(drive)
}

/// Get `UserAssist` entries for all users in NTUSER.DAT files. Then parse the `UserAssist` data
fn parse_userassist(drive: &char) -> Result<Vec<UserAssistEntry>, UserAssistError> {
    let entries = get_userassist_data(drive)?;
    UserAssistEntry::parse_userassist(&entries)
}

#[cfg(test)]
mod tests {
    use crate::{
        artifacts::os::windows::userassist::parser::{
            alt_drive_userassist, default_userassist, grab_userassist, parse_userassist,
        },
        structs::artifacts::os::windows::UserAssistOptions,
    };

    #[test]
    fn test_default_userassist() {
        let results = default_userassist().unwrap();
        assert!(results.len() > 3);
    }

    #[test]
    fn test_alt_drive_userassist() {
        let results = alt_drive_userassist(&'C').unwrap();
        assert!(results.len() > 3);
    }

    #[test]
    fn test_parse_userassist() {
        let results = parse_userassist(&'C').unwrap();
        assert!(results.len() > 3);
    }

    #[test]
    fn test_grab_userassist() {
        let options = UserAssistOptions { alt_drive: None };

        let results = grab_userassist(&options).unwrap();
        assert!(results.len() > 5);
    }
}
