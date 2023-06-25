/**
 * Grab local macOS `Users` information by parsing the PLIST files at `/var/db/dslocal/nodes/Default/users`
 */
use super::opendirectory::OpendirectoryUsers;
use crate::filesystem::files::list_files;
use log::{error, warn};

/// Get users on a macOS system. Requires root
pub(crate) fn grab_users() -> Vec<OpendirectoryUsers> {
    // Need root permissions to access files in dslocal directories
    let opendirectory_path = "/var/db/dslocal/nodes/Default/users";

    let mut user_data: Vec<OpendirectoryUsers> = Vec::new();
    let files_result = list_files(opendirectory_path);
    let files = match files_result {
        Ok(result) => result,
        Err(err) => {
            warn!("[users] Failed to list files, error: {err:?}");
            return user_data;
        }
    };
    for user in files {
        let opendirectoryd_users = OpendirectoryUsers::parse_users_plist(&user);
        match opendirectoryd_users {
            Ok(results) => user_data.push(results),
            Err(err) => {
                error!("[users] Failed to parse file {user}. Error: {err:?}");
                continue;
            }
        }
    }
    user_data
}

#[cfg(test)]
mod tests {
    use super::grab_users;

    #[test]
    fn test_grab_users() {
        let results = grab_users();
        for data in results {
            if data.uid.len() == 1 && data.uid[0] == "0" {
                assert_eq!(data.real_name[0], "System Administrator");
                assert_eq!(data.name[0], "root");
                assert_eq!(data.gid[0], "0");
                assert_eq!(data.account_photo.len(), 0);
                assert!(data.account_created > 1000.0);
                assert_eq!(data.password_last_set, 0.0);
                assert_eq!(data.shell[0], "/bin/sh");
                assert_eq!(data.unlock_options.len(), 0);
                assert_eq!(data.home_path[0], "/var/root");
                assert_eq!(data.uuid[0], "FFFFEEEE-DDDD-CCCC-BBBB-AAAA00000000");
            }
        }
    }
}
