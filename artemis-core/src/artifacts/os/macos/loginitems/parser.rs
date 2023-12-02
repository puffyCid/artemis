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
use super::{
    error::LoginItemError,
    loginitem::{loginitem_apps_system, parse_loginitems},
};
use crate::filesystem::{directory::get_user_paths, metadata::glob_paths};
use common::macos::LoginItemsData;
use log::error;
use std::path::Path;

/// Parse `LoginItem` paths on macOS system
pub(crate) fn grab_loginitems() -> Result<Vec<LoginItemsData>, LoginItemError> {
    let loginitems_path =
        "/Library/Application Support/com.apple.backgroundtaskmanagementagent/backgrounditems.btm";

    let mut loginitems_data: Vec<LoginItemsData> = Vec::new();

    let user_paths_result = get_user_paths();
    let user_paths = match user_paths_result {
        Ok(result) => result,
        Err(_) => return Err(LoginItemError::Path),
    };
    for dir in user_paths {
        let path = format!("{dir}{loginitems_path}");
        let full_path = Path::new(&path);

        if full_path.is_file() {
            let plist_path = full_path.display().to_string();
            let results = parse_loginitems(&plist_path);
            match results {
                Ok(mut data) => loginitems_data.append(&mut data),
                Err(err) => return Err(err),
            }
        }
    }

    // Starting on Ventura `LoginItems` file now also contains Launch daemons and agents
    // We still only want loginitems
    let ventura_loginitems = "/var/db/com.apple.backgroundtaskmanagement/BackgroundItems-v*.btm";
    let loginitems_glob = glob_paths(ventura_loginitems);
    let items = match loginitems_glob {
        Ok(results) => results,
        Err(err) => {
            error!("[loginitems] Failed to glob {ventura_loginitems}: {err:?}");
            Vec::new()
        }
    };

    for item in items {
        if !item.is_file {
            continue;
        }

        let results = parse_loginitems(&item.full_path);
        match results {
            Ok(mut data) => loginitems_data.append(&mut data),
            Err(err) => {
                error!("[loginitem] Could not parse modern loginitems: {err:?}");
            }
        }
    }

    let mut app_loginitems = loginitem_apps_system()?;
    loginitems_data.append(&mut app_loginitems);

    Ok(loginitems_data)
}

#[cfg(test)]
mod tests {
    use super::grab_loginitems;

    #[test]
    fn test_grab_loginitems() {
        let _ = grab_loginitems().unwrap();
    }
}
