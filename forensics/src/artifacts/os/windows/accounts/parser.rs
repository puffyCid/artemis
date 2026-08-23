use super::{error::AccountError, users::parse_user_info};
use crate::{
    structs::artifacts::os::windows::WindowsUserOptions, utils::environment::get_systemdrive,
};
use common::windows::UserInfo;
use tracing::error;

/// Get Windows `Users` for based on optional drive, otherwise default drive letter is used
pub(crate) fn grab_users(options: &WindowsUserOptions) -> Result<Vec<UserInfo>, AccountError> {
    let path = if let Some(file) = &options.alt_file {
        file.clone()
    } else {
        let drive_result = get_systemdrive();
        let drive = match drive_result {
            Ok(result) => result,
            Err(err) => {
                error!("Could not get default system drive letter: {err:?}");
                return Err(AccountError::DefaultDrive);
            }
        };
        // Account info could be found in multiple Registry files, currently only focusing on SAM
        format!("ntfs:{drive}:\\Windows\\System32\\config\\SAM")
    };

    parse_user_info(&path)
}

#[cfg(test)]
#[cfg(target_os = "windows")]
mod tests {
    use crate::{
        artifacts::os::windows::accounts::parser::grab_users,
        structs::artifacts::os::windows::WindowsUserOptions,
    };

    #[test]
    fn test_grab_users() {
        let options = WindowsUserOptions { alt_file: None };
        let result = grab_users(&options).unwrap();
        assert!(result.len() > 2);
    }
}
