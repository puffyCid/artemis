use crate::artifacts::os::macos::{
    bookmarks::parser::parse_bookmark,
    plist::{
        error::PlistError,
        property_list::{get_dictionary, parse_plist_file_dict},
    },
};
use common::macos::LoginItemsData;
use plist::Value;

/// Parse PLIST file and get Vec of bookmark data
pub(crate) fn get_bookmarks(path: &str) -> Result<Vec<LoginItemsData>, PlistError> {
    let login_items = parse_plist_file_dict(path)?;
    for (key, value) in login_items {
        if key != "$objects" {
            continue;
        }
        if let Value::Array(value_array) = value {
            let results = get_array_values(value_array, path)?;
            return Ok(results);
        }
    }
    Ok(Vec::new())
}

/// Loop through Array values and identify bookmark data (should be at least 48 bytes in size and have signature `book`
fn get_array_values(
    data_results: Vec<Value>,
    source: &str,
) -> Result<Vec<LoginItemsData>, PlistError> {
    let mut loginitems = Vec::new();
    for data in data_results {
        match data {
            Value::Data(value) => match collect_bookmarks(&value, source) {
                Ok(value) => loginitems.push(value),
                Err(_err) => {}
            },
            Value::Dictionary(_) => {
                let dict_result = get_dictionary(&data);
                let dict = match dict_result {
                    Ok(result) => result,
                    Err(_) => {
                        continue;
                    }
                };

                for (_dict_key, dict_data) in dict {
                    if let Value::Data(value) = dict_data {
                        match collect_bookmarks(&value, source) {
                            Ok(value) => loginitems.push(value),
                            Err(_err) => {}
                        }
                    }
                }
            }
            Value::String(value) => {
                if !value.contains("Contents/Library/LoginItems") {
                    continue;
                }
                let item = LoginItemsData {
                    path: value,
                    cnid_path: String::new(),
                    created: String::new(),
                    volume_path: String::new(),
                    volume_url: String::new(),
                    volume_name: String::new(),
                    volume_uuid: String::new(),
                    volume_size: 0,
                    volume_created: String::new(),
                    volume_flags: Vec::new(),
                    volume_root: false,
                    localized_name: String::new(),
                    security_extension_rw: String::new(),
                    security_extension_ro: String::new(),
                    target_flags: Vec::new(),
                    username: String::new(),
                    folder_index: 0,
                    uid: 0,
                    creation_options: Vec::new(),
                    is_bundled: true,
                    app_id: String::new(),
                    app_binary: String::new(),
                    is_executable: false,
                    file_ref_flag: false,
                    source_path: source.to_string(),
                };
                loginitems.push(item);
            }
            _ => (),
        }
    }

    Ok(loginitems)
}

/// Grab all data that meets the minimum bookmark size and starts with signature (`book`)
fn collect_bookmarks(value: &[u8], source: &str) -> Result<LoginItemsData, PlistError> {
    let min_bookmark_size = 48;
    if !value.starts_with(&[98, 111, 111, 107]) && value.len() < min_bookmark_size {
        return Err(PlistError::Array);
    }

    let bookmark = match parse_bookmark(value) {
        Ok(result) => result,
        Err(_err) => {
            return Err(PlistError::Array);
        }
    };
    let loginitem_data = LoginItemsData {
        path: bookmark.path,
        cnid_path: bookmark.cnid_path,
        created: bookmark.created,
        volume_path: bookmark.volume_path,
        volume_url: bookmark.volume_url,
        volume_name: bookmark.volume_name,
        volume_uuid: bookmark.volume_uuid,
        volume_size: bookmark.volume_size,
        volume_created: bookmark.volume_created,
        volume_flags: bookmark.volume_flags,
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
        source_path: source.to_string(),
    };

    Ok(loginitem_data)
}

#[cfg(test)]
mod tests {
    use super::{collect_bookmarks, get_array_values, get_bookmarks};
    use crate::artifacts::os::macos::plist::property_list::parse_plist_file_dict;
    use plist::Value;
    use std::path::PathBuf;

    #[test]
    fn test_get_bookmarks() {
        let mut test_location = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
        test_location.push("tests/test_data/macos/loginitems/backgrounditems_sierra.btm");

        let bookmarks = get_bookmarks(&test_location.display().to_string()).unwrap();
        assert_eq!(bookmarks.len(), 1);
    }

    #[test]
    fn test_get_array_values() {
        let mut test_location = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
        test_location.push("tests/test_data/macos/loginitems/backgrounditems_sierra.btm");

        let login_items = parse_plist_file_dict(&test_location.display().to_string()).unwrap();

        let mut results = Vec::new();
        for (key, value) in login_items {
            if key.as_str() != "$objects" {
                continue;
            }

            if let Value::Array(value_array) = value {
                results = get_array_values(value_array, "test").unwrap();
            }
        }
        assert_eq!(results.len(), 1);
    }

    #[test]
    fn test_collect_bookmarks() {
        let test_value = vec![
            98, 111, 111, 107, 244, 2, 0, 0, 0, 0, 4, 16, 48, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0,
            0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 8, 2, 0, 0, 12, 0,
            0, 0, 1, 1, 0, 0, 65, 112, 112, 108, 105, 99, 97, 116, 105, 111, 110, 115, 13, 0, 0, 0,
            1, 1, 0, 0, 83, 121, 110, 99, 116, 104, 105, 110, 103, 46, 97, 112, 112, 0, 0, 0, 8, 0,
            0, 0, 1, 6, 0, 0, 4, 0, 0, 0, 24, 0, 0, 0, 8, 0, 0, 0, 4, 3, 0, 0, 103, 0, 0, 0, 0, 0,
            0, 0, 8, 0, 0, 0, 4, 3, 0, 0, 42, 198, 10, 0, 0, 0, 0, 0, 8, 0, 0, 0, 1, 6, 0, 0, 64,
            0, 0, 0, 80, 0, 0, 0, 8, 0, 0, 0, 0, 4, 0, 0, 65, 195, 213, 41, 226, 128, 0, 0, 24, 0,
            0, 0, 1, 2, 0, 0, 2, 0, 0, 0, 0, 0, 0, 0, 15, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0,
            0, 8, 0, 0, 0, 1, 9, 0, 0, 102, 105, 108, 101, 58, 47, 47, 47, 12, 0, 0, 0, 1, 1, 0, 0,
            77, 97, 99, 105, 110, 116, 111, 115, 104, 32, 72, 68, 8, 0, 0, 0, 4, 3, 0, 0, 0, 96,
            127, 115, 37, 0, 0, 0, 8, 0, 0, 0, 0, 4, 0, 0, 65, 172, 190, 215, 104, 0, 0, 0, 36, 0,
            0, 0, 1, 1, 0, 0, 48, 65, 56, 49, 70, 51, 66, 49, 45, 53, 49, 68, 57, 45, 51, 51, 51,
            53, 45, 66, 51, 69, 51, 45, 49, 54, 57, 67, 51, 54, 52, 48, 51, 54, 48, 68, 24, 0, 0,
            0, 1, 2, 0, 0, 129, 0, 0, 0, 1, 0, 0, 0, 239, 19, 0, 0, 1, 0, 0, 0, 0, 0, 0, 0, 0, 0,
            0, 0, 1, 0, 0, 0, 1, 1, 0, 0, 47, 0, 0, 0, 0, 0, 0, 0, 1, 5, 0, 0, 9, 0, 0, 0, 1, 1, 0,
            0, 83, 121, 110, 99, 116, 104, 105, 110, 103, 0, 0, 0, 166, 0, 0, 0, 1, 2, 0, 0, 54,
            52, 99, 98, 55, 101, 97, 97, 57, 97, 49, 98, 98, 99, 99, 99, 52, 101, 49, 51, 57, 55,
            99, 57, 102, 50, 97, 52, 49, 49, 101, 98, 101, 53, 51, 57, 99, 100, 50, 57, 59, 48, 48,
            48, 48, 48, 48, 48, 48, 59, 48, 48, 48, 48, 48, 48, 48, 48, 59, 48, 48, 48, 48, 48, 48,
            48, 48, 48, 48, 48, 48, 48, 48, 50, 48, 59, 99, 111, 109, 46, 97, 112, 112, 108, 101,
            46, 97, 112, 112, 45, 115, 97, 110, 100, 98, 111, 120, 46, 114, 101, 97, 100, 45, 119,
            114, 105, 116, 101, 59, 48, 49, 59, 48, 49, 48, 48, 48, 48, 48, 52, 59, 48, 48, 48, 48,
            48, 48, 48, 48, 48, 48, 48, 97, 99, 54, 50, 97, 59, 47, 97, 112, 112, 108, 105, 99, 97,
            116, 105, 111, 110, 115, 47, 115, 121, 110, 99, 116, 104, 105, 110, 103, 46, 97, 112,
            112, 0, 0, 0, 180, 0, 0, 0, 254, 255, 255, 255, 1, 0, 0, 0, 0, 0, 0, 0, 14, 0, 0, 0, 4,
            16, 0, 0, 48, 0, 0, 0, 0, 0, 0, 0, 5, 16, 0, 0, 96, 0, 0, 0, 0, 0, 0, 0, 16, 16, 0, 0,
            128, 0, 0, 0, 0, 0, 0, 0, 64, 16, 0, 0, 112, 0, 0, 0, 0, 0, 0, 0, 2, 32, 0, 0, 48, 1,
            0, 0, 0, 0, 0, 0, 5, 32, 0, 0, 160, 0, 0, 0, 0, 0, 0, 0, 16, 32, 0, 0, 176, 0, 0, 0, 0,
            0, 0, 0, 17, 32, 0, 0, 228, 0, 0, 0, 0, 0, 0, 0, 18, 32, 0, 0, 196, 0, 0, 0, 0, 0, 0,
            0, 19, 32, 0, 0, 212, 0, 0, 0, 0, 0, 0, 0, 32, 32, 0, 0, 16, 1, 0, 0, 0, 0, 0, 0, 48,
            32, 0, 0, 60, 1, 0, 0, 0, 0, 0, 0, 23, 240, 0, 0, 68, 1, 0, 0, 0, 0, 0, 0, 128, 240, 0,
            0, 88, 1, 0, 0, 0, 0, 0, 0,
        ];

        let value = collect_bookmarks(&test_value, "test").unwrap();
        assert_eq!(value.path, "/Applications/Syncthing.app");
        assert_eq!(value.source_path, "test");
    }
}
