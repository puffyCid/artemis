use super::error::LoginItemError;
use super::plist::get_bookmarks;
use crate::artifacts::os::macos::plist::property_list::parse_plist_file_dict;
use crate::{
    artifacts::os::macos::bookmarks::parser::parse_bookmark, filesystem::files::list_files,
};
use common::macos::LoginItemsData;
use log::{error, warn};

/// Parse User `LoginItems` from provided path
pub(crate) fn parse_loginitems(path: &str) -> Result<Vec<LoginItemsData>, LoginItemError> {
    // Parse PLIST file and get any bookmark data
    let loginitems_results = get_bookmarks(path);
    let loginitems_data = match loginitems_results {
        Ok(data) => data,
        Err(err) => {
            error!("[loginitem] Failed to read plist {path}: {err:?}");
            return Err(LoginItemError::Plist);
        }
    };
    if loginitems_data.is_empty() {
        return Ok(Vec::new());
    }

    let mut loginitems_results: Vec<LoginItemsData> = Vec::new();
    for data in loginitems_data {
        let results = parse_bookmark(&data);
        let bookmark = match results {
            Ok(bookmark_data) => bookmark_data,
            Err(err) => {
                // Even if we fail to parse one (1) entry keep trying the others
                // Plist may contain non-bookmark binary data
                error!("Failed to parse bookmark data: {err:?}");
                continue;
            }
        };
        let mut loginitem_data = LoginItemsData {
            path: bookmark.path,
            cnid_path: bookmark.cnid_path,
            created: bookmark.created,
            volume_path: bookmark.volume_path,
            volume_url: bookmark.volume_url,
            volume_name: bookmark.volume_name,
            volume_uuid: bookmark.volume_uuid,
            volume_size: bookmark.volume_size,
            volume_created: bookmark.volume_created,
            volume_flag: bookmark.volume_flag,
            volume_root: bookmark.volume_root,
            localized_name: bookmark.localized_name,
            security_extension_rw: bookmark.security_extension_rw,
            security_extension_ro: bookmark.security_extension_ro,
            target_flags: bookmark.target_flags,
            username: bookmark.username,
            folder_index: bookmark.folder_index,
            uid: bookmark.uid,
            creation_options: bookmark.creation_options,
            file_ref_flag: bookmark.file_ref_flag,
            is_bundled: false,
            app_id: String::new(),
            app_binary: String::new(),
            is_executable: bookmark.is_executable,
            source_path: String::new(),
        };
        loginitem_data.source_path = path.to_string();
        loginitems_results.push(loginitem_data);
    }

    Ok(loginitems_results)
}

/// Parse `LoginItems` associated with bundled apps
pub(crate) fn loginitems_bundled_apps_path(
    path: &str,
) -> Result<Vec<LoginItemsData>, LoginItemError> {
    let files_result = list_files(path);
    let files = match files_result {
        Ok(result) => result,
        Err(err) => {
            error!("[loginitems] Failed to read LoginItem bundled App directory: {err:?}");
            return Err(LoginItemError::Path);
        }
    };
    let mut loginitems_vec: Vec<LoginItemsData> = Vec::new();

    for file in files {
        if !file.contains("loginitems") {
            continue;
        }

        let loginitems_plist = parse_plist_file_dict(&file);
        match loginitems_plist {
            Ok(data) => {
                for (key, value) in data {
                    let mut loginitems_data = LoginItemsData {
                        path: Vec::new(),
                        cnid_path: Vec::new(),
                        created: 0,
                        volume_path: String::new(),
                        volume_url: String::new(),
                        volume_name: String::new(),
                        volume_uuid: String::new(),
                        volume_size: 0,
                        volume_created: 0,
                        volume_flag: Vec::new(),
                        volume_root: false,
                        localized_name: String::new(),
                        security_extension_rw: String::new(),
                        security_extension_ro: String::new(),
                        target_flags: Vec::new(),
                        username: String::new(),
                        folder_index: 0,
                        uid: 0,
                        creation_options: 0,
                        is_bundled: true,
                        app_id: String::new(),
                        app_binary: String::new(),
                        is_executable: false,
                        file_ref_flag: false,
                        source_path: String::new(),
                    };

                    if key.starts_with("version") {
                        continue;
                    }

                    if let Some(app_id) = value.into_string() {
                        loginitems_data.app_id = app_id;
                    } else {
                        warn!("[loginitems] No app id associated with bundled");
                    }

                    loginitems_data.app_binary = key;
                    loginitems_data.source_path = file.clone();
                    loginitems_vec.push(loginitems_data);
                }
            }
            Err(err) => {
                warn!("[loginitems] Failed to parse plist: {file} {err:?}");
            }
        }
    }

    Ok(loginitems_vec)
}

/// Get `LoginItems` data from embedded `LoginItems` in Apps
pub(crate) fn loginitem_apps_system() -> Result<Vec<LoginItemsData>, LoginItemError> {
    let default_path = "/var/db/com.apple.xpc.launchd/";
    loginitems_bundled_apps_path(default_path)
}

#[cfg(test)]
mod tests {
    use super::loginitem_apps_system;
    use crate::artifacts::os::macos::loginitems::loginitem::{
        loginitems_bundled_apps_path, parse_loginitems,
    };
    use std::path::PathBuf;

    #[test]
    fn test_loginitem_apps_system() {
        let _ = loginitem_apps_system().unwrap();
    }

    #[test]
    fn test_loginitems_bundled_apps_path() {
        let mut test_location = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
        test_location.push("tests/test_data/macos/loginitems/bundled_app");

        let data = loginitems_bundled_apps_path(&test_location.display().to_string()).unwrap();
        assert_eq!(data.len(), 2);
        assert_eq!(data[0].is_bundled, true);
        assert_eq!(data[0].app_binary, "com.docker.helper");
        assert_eq!(data[0].app_id, "com.docker.docker");

        assert_eq!(data[1].is_bundled, true);
        assert_eq!(data[1].app_binary, "com.csaba.fitzl.shield.ShieldHelper");
        assert_eq!(data[1].app_id, "com.csaba.fitzl.shield");
    }

    #[test]
    fn test_parse_loginitems() {
        let mut test_location = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
        test_location.push("tests/test_data/macos/loginitems/backgrounditems_sierra.btm");
        let data = parse_loginitems(&test_location.display().to_string()).unwrap();

        assert_eq!(data.len(), 1);
        assert_eq!(data[0].path, ["Applications", "Syncthing.app"]);
        assert_eq!(data[0].created, 1643781189);
        assert_eq!(data[0].cnid_path, [103, 706090]);
        assert_eq!(data[0].volume_path, "/");
        assert_eq!(data[0].volume_url, "file:///");
        assert_eq!(data[0].volume_name, "Macintosh HD");
        assert_eq!(data[0].volume_uuid, "0A81F3B1-51D9-3335-B3E3-169C3640360D");
        assert_eq!(data[0].volume_size, 160851517440);
        assert_eq!(data[0].volume_created, 1219441716);
        assert_eq!(data[0].volume_flag, [4294967425, 4294972399, 0]);
        assert_eq!(data[0].volume_root, true);
        assert_eq!(data[0].localized_name, "Syncthing");
        assert_eq!(data[0].security_extension_rw, "64cb7eaa9a1bbccc4e1397c9f2a411ebe539cd29;00000000;00000000;0000000000000020;com.apple.app-sandbox.read-write;01;01000004;00000000000ac62a;/applications/syncthing.app\0");
        assert_eq!(data[0].security_extension_ro, "");
        assert_eq!(data[0].target_flags, [2, 15, 0]);
        assert_eq!(data[0].username, String::new());
        assert_eq!(data[0].folder_index, 0);
        assert_eq!(data[0].uid, 0);
        assert_eq!(data[0].is_bundled, false);
        assert_eq!(data[0].app_id, String::new());
        assert_eq!(data[0].app_binary, String::new());
        assert_eq!(data[0].is_executable, false);
    }
}
