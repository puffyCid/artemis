use super::{error::AccountError, users::parse_user_info};
use crate::{structs::artifacts::os::windows::UserOptions, utils::environment::get_systemdrive};
use common::windows::UserInfo;
use log::error;

/// Get Windows `Users` for based on optional drive, otherwise default drive letter is used
pub(crate) fn grab_users(options: &UserOptions) -> Result<Vec<UserInfo>, AccountError> {
    if let Some(alt_drive) = options.alt_drive {
        return parse_user_info(&alt_drive);
    }
    let drive_result = get_systemdrive();
    let drive = match drive_result {
        Ok(result) => result,
        Err(err) => {
            error!("[accounts] Could not get default systemdrive letter: {err:?}");
            return Err(AccountError::DefaultDrive);
        }
    };

    parse_user_info(&drive)
}

#[cfg(test)]
mod tests {
    use crate::{
        artifacts::os::windows::accounts::parser::grab_users,
        structs::artifacts::os::windows::UserOptions,
    };

    #[test]
    fn test_grab_users() {
        let options = UserOptions { alt_drive: None };
        let result = grab_users(&options).unwrap();
        assert!(result.len() > 2);
    }
}
