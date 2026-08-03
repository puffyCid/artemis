/**
 * macOS `LoginItems` can be used to achieve persistence on macOS systems
 *
 * They exist per user account at:
 *   `/Users/%/Library/Application Support/com.apple.backgroundtaskmanagementagent/backgrounditems.btm` (pre-Ventura)
 *   `/var/db/com.apple.backgroundtaskmanagement/BackgroundItems-v*.btm` (Ventura+)
 *
 * References:
 *   `https://www.sentinelone.com/blog/how-malware-persists-on-macos/`
 */
use super::error::LoginItemError;
use crate::{
    accessor::{
        access::Accessor,
        entry::handle::{EntryKind, GlobMatch},
    },
    artifacts::os::macos::loginitems::plist::{bundle_plist, get_bookmarks},
    structs::artifacts::os::macos::LoginitemsOptions,
};
use common::macos::LoginItemsData;
use tracing::warn;

/// Parse `LoginItem` paths on macOS system
pub(crate) fn grab_loginitems(
    options: &LoginitemsOptions,
) -> Result<Vec<LoginItemsData>, LoginItemError> {
    let paths = if let Some(alt_file) = &options.alt_file {
        vec![alt_file.as_str()]
    } else {
        vec![
            "/Users/*/Library/Application Support/com.apple.backgroundtaskmanagementagent/backgrounditems.btm",
            "/var/db/com.apple.backgroundtaskmanagement/BackgroundItems-v*.btm",
            "/var/db/com.apple.xpc.launchd/*",
        ]
    };

    let mut accessor = Accessor::with_defaults();
    let mut items = Vec::new();

    for path in paths {
        let item_files = match accessor.globfs(path) {
            Ok(result) => result,
            Err(err) => {
                warn!("Failed to glob '{path}': {err:?}");
                continue;
            }
        };

        items.append(&mut extract_loginitem(item_files, &mut accessor));
    }

    Ok(items)
}

/// Get `LoginItems` from btm and plist files
fn extract_loginitem(paths: Vec<GlobMatch>, accessor: &mut Accessor) -> Vec<LoginItemsData> {
    let mut values = Vec::new();
    for entry in paths {
        if entry.meta.kind != EntryKind::File {
            continue;
        }

        let Some(file_handle) = entry.handle.as_file() else {
            continue;
        };

        let bytes = match accessor.read_file_handle(file_handle) {
            Ok(result) => result,
            Err(err) => {
                warn!(
                    "Could not read file '{}': {err:?}",
                    file_handle.display_path()
                );
                continue;
            }
        };

        if file_handle.display_path().ends_with(".btm") {
            let mut items = match get_bookmarks(&bytes, &file_handle.display_path()) {
                Ok(result) => result,
                Err(_err) => continue,
            };
            values.append(&mut items);
        } else if file_handle.display_path().contains("loginitems") {
            let mut items = match bundle_plist(&bytes, &file_handle.display_path()) {
                Ok(result) => result,
                Err(_err) => continue,
            };
            values.append(&mut items);
        }
    }

    values
}

#[cfg(test)]
mod tests {
    use super::grab_loginitems;
    use crate::{
        accessor::access::Accessor, artifacts::os::macos::loginitems::parser::extract_loginitem,
        structs::artifacts::os::macos::LoginitemsOptions,
    };
    use common::macos::{TargetFlags, VolumeFlags};
    use std::path::PathBuf;

    #[test]
    fn test_grab_loginitems() {
        let _ = grab_loginitems(&LoginitemsOptions { alt_file: None }).unwrap();
    }

    #[test]
    fn test_loginitems_bundled_apps_path() {
        let mut test_location = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
        test_location.push("tests/test_data/macos/loginitems/bundled_app/*");
        let globs = Accessor::with_defaults()
            .globfs(test_location.to_str().unwrap())
            .unwrap();

        let data = extract_loginitem(globs, &mut Accessor::with_defaults());
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
        let globs = Accessor::with_defaults()
            .globfs(test_location.to_str().unwrap())
            .unwrap();
        let data = extract_loginitem(globs, &mut Accessor::with_defaults());
        assert_eq!(data.len(), 1);
        assert_eq!(data[0].path, "/Applications/Syncthing.app");
        assert_eq!(data[0].created, "2022-02-02T05:53:09.000Z");
        assert_eq!(data[0].cnid_path, "/103/706090");
        assert_eq!(data[0].volume_path, "/");
        assert_eq!(data[0].volume_url, "file:///");
        assert_eq!(data[0].volume_name, "Macintosh HD");
        assert_eq!(data[0].volume_uuid, "0A81F3B1-51D9-3335-B3E3-169C3640360D");
        assert_eq!(data[0].volume_size, 160851517440);
        assert_eq!(data[0].volume_created, "2008-08-22T21:48:36.000Z");
        assert_eq!(
            data[0].volume_flags,
            vec![
                VolumeFlags::Local,
                VolumeFlags::Internal,
                VolumeFlags::SupportsPersistentIds
            ]
        );
        assert_eq!(data[0].volume_root, true);
        assert_eq!(data[0].localized_name, "Syncthing");
        assert_eq!(
            data[0].security_extension_rw,
            "64cb7eaa9a1bbccc4e1397c9f2a411ebe539cd29;00000000;00000000;0000000000000020;com.apple.app-sandbox.read-write;01;01000004;00000000000ac62a;/applications/syncthing.app"
        );
        assert_eq!(data[0].security_extension_ro, "");
        assert_eq!(data[0].target_flags, vec![TargetFlags::Directory]);
        assert_eq!(data[0].username, String::new());
        assert_eq!(data[0].folder_index, 0);
        assert_eq!(data[0].uid, 0);
        assert_eq!(data[0].is_bundled, false);
        assert_eq!(data[0].app_id, String::new());
        assert_eq!(data[0].app_binary, String::new());
        assert_eq!(data[0].is_executable, false);
    }
}
