/**
 * Grab local macOS `Groups` information by parsing the PLIST files at `/var/db/dslocal/nodes/Default/groups`
 */
use super::opendirectory::parse_groups_plist;
use crate::{filesystem::files::list_files, structs::artifacts::os::macos::MacosGroupsOptions};
use common::macos::OpendirectoryGroups;
use log::{error, warn};

/// Get users on a macOS system. Requires root
pub(crate) fn grab_groups(options: &MacosGroupsOptions) -> Vec<OpendirectoryGroups> {
    let path = if let Some(alt_path) = &options.alt_path {
        alt_path
    } else {
        // Need root permissions to access files in dslocal directories
        "/var/db/dslocal/nodes/Default/groups"
    };

    let mut group_data: Vec<OpendirectoryGroups> = Vec::new();
    let files_result = list_files(path);
    let files = match files_result {
        Ok(result) => result,
        Err(err) => {
            warn!("[groups] Failed to list files, error: {err:?}");
            return group_data;
        }
    };
    for group in files {
        let opendirectoryd_users = parse_groups_plist(&group);
        match opendirectoryd_users {
            Ok(results) => {
                group_data.push(results);
            }
            Err(err) => error!("[groups] Failed to parse groups {group}: {err:?}"),
        }
    }

    group_data
}

#[cfg(test)]
#[cfg(target_os = "macos")]
mod tests {
    use crate::{
        artifacts::os::macos::accounts::groups::grab_groups,
        structs::artifacts::os::macos::MacosGroupsOptions,
    };

    #[test]
    fn test_grab_groups() {
        let results = grab_groups(&MacosGroupsOptions { alt_path: None });
        assert!(results.len() > 10);

        for data in results {
            if data.gid.len() == 1 && data.gid[0] == "0" {
                assert_eq!(data.real_name[0], "System Group");
                assert_eq!(data.users[0], "root");
                assert_eq!(data.name[0], "wheel");
                assert_eq!(data.groupmembers[0], "FFFFEEEE-DDDD-CCCC-BBBB-AAAA00000000");
            }
        }
    }
}
