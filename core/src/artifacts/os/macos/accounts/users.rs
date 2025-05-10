/**
 * Grab local macOS `Users` information by parsing the PLIST files at `/var/db/dslocal/nodes/Default/users`
 */
use super::opendirectory::parse_users_plist;
use crate::{filesystem::files::list_files, structs::artifacts::os::macos::MacosUsersOptions};
use common::macos::OpendirectoryUsers;
use log::{error, warn};

/// Get users on a macOS system. Requires root
pub(crate) fn grab_users(options: &MacosUsersOptions) -> Vec<OpendirectoryUsers> {
    let path = if let Some(alt_path) = &options.alt_path {
        alt_path
    } else {
        // Need root permissions to access files in dslocal directories
        "/var/db/dslocal/nodes/Default/users"
    };

    let mut user_data: Vec<OpendirectoryUsers> = Vec::new();
    let files_result = list_files(path);
    let files = match files_result {
        Ok(result) => result,
        Err(err) => {
            warn!("[users] Failed to list files, error: {err:?}");
            return user_data;
        }
    };
    for user in files {
        let opendirectoryd_users = parse_users_plist(&user);
        match opendirectoryd_users {
            Ok(results) => user_data.push(results),
            Err(err) => error!("[users] Failed to parse file {user}. Error: {err:?}"),
        }
    }
    user_data
}

#[cfg(test)]
mod tests {
    use super::grab_users;
    use crate::structs::artifacts::os::macos::MacosUsersOptions;

    #[test]
    fn test_grab_users() {
        let results = grab_users(&MacosUsersOptions { alt_path: None });
        for data in results {
            if data.uid.len() == 1 && data.uid[0] == "0" {
                assert_eq!(data.real_name[0], "System Administrator");
                assert_eq!(data.name[0], "root");
                assert_eq!(data.gid[0], "0");
                assert_eq!(data.account_photo.len(), 0);
                assert_eq!(data.password_last_set, "1970-01-01T00:00:00.000Z");
                assert_eq!(data.shell[0].is_empty(), false);
                assert_eq!(data.unlock_options.len(), 0);
                assert_eq!(data.home_path[0], "/var/root");
                assert_eq!(data.uuid[0], "FFFFEEEE-DDDD-CCCC-BBBB-AAAA00000000");
            }
        }
    }
}
