use std::collections::HashMap;

use super::{error::AccountError, users::parse_user_info};
use crate::{structs::artifacts::os::windows::UserOptions, utils::environment::get_systemdrive};
use common::windows::UserInfo;
use log::error;

/// Get Windows `Users` for based on optional drive, otherwise default drive letter is used
pub(crate) fn grab_users(options: &UserOptions) -> Result<Vec<UserInfo>, AccountError> {
    if let Some(file) = &options.alt_file {
        return parse_user_info(file);
    }
    let drive_result = get_systemdrive();
    let drive = match drive_result {
        Ok(result) => result,
        Err(err) => {
            error!("[accounts] Could not get default systemdrive letter: {err:?}");
            return Err(AccountError::DefaultDrive);
        }
    };
    // Account info could be found in multiple Registry files, currently only focusing on SAM
    let path = format!("{drive}:\\Windows\\System32\\config\\SAM");

    parse_user_info(&path)
}

/// Get hashmap of users
pub(crate) fn get_users() -> Result<HashMap<String, String>, AccountError> {
    let drive_result = get_systemdrive();
    let drive = match drive_result {
        Ok(result) => result,
        Err(err) => {
            error!("[accounts] Could not get default systemdrive letter: {err:?}");
            return Err(AccountError::DefaultDrive);
        }
    };
    // Account info could be found in multiple Registry files, currently only focusing on SAM
    let path = format!("{drive}:\\Windows\\System32\\config\\SAM");

    let mut users = HashMap::new();
    let entries = parse_user_info(&path)?;

    for entry in entries {
        users.insert(entry.sid.clone(), entry.username);
    }

    Ok(users)
}

#[cfg(test)]
#[cfg(target_os = "windows")]
mod tests {
    use crate::{
        artifacts::os::windows::accounts::parser::{get_users, grab_users},
        structs::artifacts::os::windows::UserOptions,
    };

    #[test]
    fn test_grab_users() {
        let options = UserOptions { alt_file: None };
        let result = grab_users(&options).unwrap();
        assert!(result.len() > 2);
    }

    #[test]
    fn test_get_users() {
        let result = get_users().unwrap();
        assert!(result.len() > 2);
    }
}
