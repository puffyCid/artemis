use crate::artifacts::os::macos::plist::{
    error::PlistError,
    property_list::{get_boolean, get_data, get_signed_int, get_string, parse_plist_file_dict},
};
use log::warn;
use plist::{Dictionary, Value};
use std::time::{SystemTime, UNIX_EPOCH};

#[derive(Debug)]
pub(crate) struct DownloadsPlist {
    pub(crate) bookmark_blob: Vec<u8>,
    pub(crate) download_entry_progress_total_to_load: i64,
    pub(crate) download_entry_progress_bytes_so_far: i64,
    pub(crate) download_entry_date_added_key: u64,
    pub(crate) download_entry_date_finished_key: u64,
    pub(crate) download_entry_should_use_request_url_as_origin: bool,
    pub(crate) download_identifier: String,
    pub(crate) download_url: String,
    pub(crate) download_path: String,
    pub(crate) download_sandbox_id: String,
    pub(crate) download_remove_when_done: bool,
    pub(crate) uuid: String,
}

impl DownloadsPlist {
    /// Parse a PLIST file at provided path
    pub(crate) fn parse_safari_plist(path: &str) -> Result<Vec<DownloadsPlist>, PlistError> {
        let downloads: Dictionary = parse_plist_file_dict(path)?;
        let mut downloads_data: Vec<DownloadsPlist> = Vec::new();
        for (key, value) in downloads {
            if key != "DownloadHistory" {
                continue;
            }

            match value {
                Value::Array(_) => {
                    // Parse the array of dictionaries
                    downloads_data = DownloadsPlist::get_array_values(value)?;
                }
                _ => {
                    warn!("Empty PLIST Array data");
                }
            }
        }

        Ok(downloads_data)
    }

    /// Loop through Array values and get the downloads metadata
    fn get_array_values(value: Value) -> Result<Vec<DownloadsPlist>, PlistError> {
        let mut downloads_data: Vec<DownloadsPlist> = Vec::new();
        let results = value.into_array();
        match results {
            Some(data_results) => {
                for data in data_results {
                    match data {
                        // Each download should be a dictionary containing some metadata
                        Value::Dictionary(_) => {
                            let dict_bookmark = data.as_dictionary();
                            let mut downloads_metadata = DownloadsPlist {
                                bookmark_blob: Vec::new(),
                                download_entry_progress_total_to_load: 0,
                                download_entry_progress_bytes_so_far: 0,
                                download_entry_date_added_key: 0,
                                download_entry_date_finished_key: 0,
                                download_entry_should_use_request_url_as_origin: false,
                                download_identifier: String::new(),
                                download_url: String::new(),
                                download_path: String::new(),
                                download_sandbox_id: String::new(),
                                download_remove_when_done: false,
                                uuid: String::new(),
                            };
                            match dict_bookmark {
                                Some(dict) => {
                                    for (dict_key, dict_data) in dict {
                                        match dict_key.as_str() {
                                            "DownloadEntryBookmarkBlob" => downloads_metadata.bookmark_blob = get_data(dict_data).unwrap_or_default(),
                                            "DownloadEntryProgressTotalToLoad" => downloads_metadata.download_entry_progress_total_to_load = get_signed_int(dict_data).unwrap_or_default(),
                                            "DownloadEntryProgressBytesSoFar" => downloads_metadata.download_entry_progress_bytes_so_far = get_signed_int(dict_data).unwrap_or_default(),
                                            "DownloadEntryDateAddedKey" => downloads_metadata.download_entry_date_added_key = DownloadsPlist::get_safari_timestamp(dict_data),
                                            "DownloadEntryDateFinishedKey" => downloads_metadata.download_entry_date_finished_key = DownloadsPlist::get_safari_timestamp(dict_data),
                                            "DownloadEntryShouldUseRequestURLAsOriginURLIfNecessaryKey" => downloads_metadata.download_entry_should_use_request_url_as_origin = get_boolean(dict_data).unwrap_or_default(),
                                            "DownloadEntryIdentifier" => downloads_metadata.download_identifier = get_string(dict_data).unwrap_or_default(),
                                            "DownloadEntryURL" => downloads_metadata.download_url = get_string(dict_data).unwrap_or_default(),
                                            "DownloadEntryPath" => downloads_metadata.download_path = get_string(dict_data).unwrap_or_default(),
                                            "DownloadEntrySandboxIdentifier" => downloads_metadata.download_sandbox_id = get_string(dict_data).unwrap_or_default(),
                                            "DownloadEntryRemoveWhenDoneKey" => downloads_metadata.download_remove_when_done = get_boolean(dict_data).unwrap_or_default(),
                                            "DownloadEntryProfileUUIDStringKey" => downloads_metadata.uuid = get_string(dict_data).unwrap_or_default(),
                                            _ => warn!("Unknown Safari download key: {dict_key}")
                                        }
                                    }
                                    downloads_data.push(downloads_metadata);
                                }
                                None => continue,
                            }
                        }
                        _ => continue,
                    }
                }
            }
            None => return Ok(downloads_data),
        }
        Ok(downloads_data)
    }

    // Safari uses Apple Cocoa Core Data timestamp. Number of seconds since 2001-01-01 00:00:00 UTC
    // The Rust PLIST crate handles converting to UNIX Epoch seconds
    fn get_safari_timestamp(dict_data: &Value) -> u64 {
        let date_added = if let Some(results) = dict_data.as_date() {
            results
        } else {
            warn!("No timestamp in PLIST file");
            return 0;
        };

        let date_time: SystemTime = date_added.into();
        let epoch_time_results = date_time.duration_since(UNIX_EPOCH);
        let epoch_time = match epoch_time_results {
            Ok(results) => results,
            Err(err) => {
                warn!("Failed to parse Timestamp in PLIST file: {err:?}");
                return 0;
            }
        };
        epoch_time.as_secs()
    }
}

#[cfg(test)]
mod tests {
    use plist::{Dictionary, Value};
    use std::{path::PathBuf, time::UNIX_EPOCH};

    use super::DownloadsPlist;

    #[test]
    fn test_parse_safari_plist() {
        let mut test_location = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
        test_location.push("tests/test_data/browser/safari/Downloads.plist");

        let results =
            DownloadsPlist::parse_safari_plist(&test_location.display().to_string()).unwrap();

        let bookmark_blob = [
            98, 111, 111, 107, 208, 2, 0, 0, 0, 0, 4, 16, 48, 0, 0, 0, 56, 108, 226, 233, 179, 84,
            237, 152, 124, 140, 84, 89, 175, 6, 172, 193, 190, 107, 33, 234, 153, 156, 121, 80,
            209, 98, 72, 198, 97, 36, 76, 221, 204, 1, 0, 0, 4, 0, 0, 0, 3, 3, 0, 0, 0, 24, 0, 40,
            5, 0, 0, 0, 1, 1, 0, 0, 85, 115, 101, 114, 115, 0, 0, 0, 8, 0, 0, 0, 1, 1, 0, 0, 112,
            117, 102, 102, 121, 99, 105, 100, 9, 0, 0, 0, 1, 1, 0, 0, 68, 111, 119, 110, 108, 111,
            97, 100, 115, 0, 0, 0, 30, 0, 0, 0, 1, 1, 0, 0, 112, 111, 119, 101, 114, 115, 104, 101,
            108, 108, 45, 55, 46, 50, 46, 53, 45, 111, 115, 120, 45, 97, 114, 109, 54, 52, 46, 112,
            107, 103, 0, 0, 16, 0, 0, 0, 1, 6, 0, 0, 16, 0, 0, 0, 32, 0, 0, 0, 48, 0, 0, 0, 68, 0,
            0, 0, 8, 0, 0, 0, 4, 3, 0, 0, 79, 83, 0, 0, 0, 0, 0, 0, 8, 0, 0, 0, 4, 3, 0, 0, 11,
            128, 5, 0, 0, 0, 0, 0, 8, 0, 0, 0, 4, 3, 0, 0, 62, 128, 5, 0, 0, 0, 0, 0, 8, 0, 0, 0,
            4, 3, 0, 0, 104, 141, 63, 2, 0, 0, 0, 0, 16, 0, 0, 0, 1, 6, 0, 0, 132, 0, 0, 0, 148, 0,
            0, 0, 164, 0, 0, 0, 180, 0, 0, 0, 8, 0, 0, 0, 0, 4, 0, 0, 65, 196, 52, 107, 24, 237,
            13, 97, 24, 0, 0, 0, 1, 2, 0, 0, 1, 0, 0, 0, 0, 0, 0, 0, 15, 0, 0, 0, 0, 0, 0, 0, 0, 0,
            0, 0, 0, 0, 0, 0, 8, 0, 0, 0, 4, 3, 0, 0, 2, 0, 0, 0, 0, 0, 0, 0, 4, 0, 0, 0, 3, 3, 0,
            0, 245, 1, 0, 0, 8, 0, 0, 0, 1, 9, 0, 0, 102, 105, 108, 101, 58, 47, 47, 47, 12, 0, 0,
            0, 1, 1, 0, 0, 77, 97, 99, 105, 110, 116, 111, 115, 104, 32, 72, 68, 8, 0, 0, 0, 4, 3,
            0, 0, 0, 112, 196, 208, 209, 1, 0, 0, 8, 0, 0, 0, 0, 4, 0, 0, 65, 195, 229, 4, 81, 128,
            0, 0, 36, 0, 0, 0, 1, 1, 0, 0, 57, 54, 70, 66, 52, 49, 67, 48, 45, 54, 67, 69, 57, 45,
            52, 68, 65, 50, 45, 56, 52, 51, 53, 45, 51, 53, 66, 67, 49, 57, 67, 55, 51, 53, 65, 51,
            24, 0, 0, 0, 1, 2, 0, 0, 129, 0, 0, 0, 1, 0, 0, 0, 239, 19, 0, 0, 1, 0, 0, 0, 0, 0, 0,
            0, 0, 0, 0, 0, 1, 0, 0, 0, 1, 1, 0, 0, 47, 0, 0, 0, 0, 0, 0, 0, 1, 5, 0, 0, 204, 0, 0,
            0, 254, 255, 255, 255, 1, 0, 0, 0, 0, 0, 0, 0, 16, 0, 0, 0, 4, 16, 0, 0, 108, 0, 0, 0,
            0, 0, 0, 0, 5, 16, 0, 0, 196, 0, 0, 0, 0, 0, 0, 0, 16, 16, 0, 0, 236, 0, 0, 0, 0, 0, 0,
            0, 64, 16, 0, 0, 220, 0, 0, 0, 0, 0, 0, 0, 2, 32, 0, 0, 184, 1, 0, 0, 0, 0, 0, 0, 5,
            32, 0, 0, 40, 1, 0, 0, 0, 0, 0, 0, 16, 32, 0, 0, 56, 1, 0, 0, 0, 0, 0, 0, 17, 32, 0, 0,
            108, 1, 0, 0, 0, 0, 0, 0, 18, 32, 0, 0, 76, 1, 0, 0, 0, 0, 0, 0, 19, 32, 0, 0, 92, 1,
            0, 0, 0, 0, 0, 0, 32, 32, 0, 0, 152, 1, 0, 0, 0, 0, 0, 0, 48, 32, 0, 0, 196, 1, 0, 0,
            0, 0, 0, 0, 1, 192, 0, 0, 12, 1, 0, 0, 0, 0, 0, 0, 17, 192, 0, 0, 32, 0, 0, 0, 0, 0, 0,
            0, 18, 192, 0, 0, 28, 1, 0, 0, 0, 0, 0, 0, 16, 208, 0, 0, 4, 0, 0, 0, 0, 0, 0, 0,
        ];
        assert_eq!(results[0].bookmark_blob, bookmark_blob);
        assert_eq!(results[0].download_entry_progress_total_to_load, 63055607);
        assert_eq!(results[0].download_entry_progress_bytes_so_far, 63055607);
        assert_eq!(results[0].download_entry_date_added_key, 1656266417);
        assert_eq!(results[0].download_entry_date_finished_key, 1656266422);
        assert_eq!(
            results[0].download_entry_should_use_request_url_as_origin,
            false
        );
        assert_eq!(
            results[0].download_identifier,
            "835D414A-492E-4DBB-BD6B-E8FACD4ED84D"
        );
        assert_eq!(results[0].download_url, "https://objects.githubusercontent.com/github-production-release-asset-2e65be/49609581/97b2b465-4242-42c6-ae6f-16437ee71f12?X-Amz-Algorithm=AWS4-HMAC-SHA256&X-Amz-Credential=AKIAIWNJYAX4CSVEH53A%2F20220626%2Fus-east-1%2Fs3%2Faws4_request&X-Amz-Date=20220626T180026Z&X-Amz-Expires=300&X-Amz-Signature=7f403834d25930916a71894a1960b7624e6479cdd493c40b96644d4a01ffdf41&X-Amz-SignedHeaders=host&actor_id=0&key_id=0&repo_id=49609581&response-content-disposition=attachment%3B%20filename%3Dpowershell-7.2.5-osx-arm64.pkg&response-content-type=application%2Foctet-stream");
        assert_eq!(
            results[0].download_path,
            "/Users/puffycid/Downloads/powershell-7.2.5-osx-arm64.pkg"
        );
        assert_eq!(
            results[0].download_sandbox_id,
            "DBA9EBA4-D23B-43C5-9DEB-131566E7BD8B"
        );
        assert_eq!(results[0].download_remove_when_done, false);

        let bookmark_blob = [
            98, 111, 111, 107, 204, 2, 0, 0, 0, 0, 4, 16, 48, 0, 0, 0, 118, 226, 194, 136, 16, 92,
            78, 163, 243, 45, 21, 244, 123, 52, 133, 130, 108, 22, 210, 185, 190, 79, 227, 3, 111,
            161, 134, 159, 76, 39, 205, 112, 200, 1, 0, 0, 4, 0, 0, 0, 3, 3, 0, 0, 0, 24, 0, 40, 5,
            0, 0, 0, 1, 1, 0, 0, 85, 115, 101, 114, 115, 0, 0, 0, 8, 0, 0, 0, 1, 1, 0, 0, 112, 117,
            102, 102, 121, 99, 105, 100, 9, 0, 0, 0, 1, 1, 0, 0, 68, 111, 119, 110, 108, 111, 97,
            100, 115, 0, 0, 0, 28, 0, 0, 0, 1, 1, 0, 0, 112, 111, 119, 101, 114, 115, 104, 101,
            108, 108, 45, 55, 46, 50, 46, 53, 45, 111, 115, 120, 45, 120, 54, 52, 46, 112, 107,
            103, 16, 0, 0, 0, 1, 6, 0, 0, 16, 0, 0, 0, 32, 0, 0, 0, 48, 0, 0, 0, 68, 0, 0, 0, 8, 0,
            0, 0, 4, 3, 0, 0, 79, 83, 0, 0, 0, 0, 0, 0, 8, 0, 0, 0, 4, 3, 0, 0, 11, 128, 5, 0, 0,
            0, 0, 0, 8, 0, 0, 0, 4, 3, 0, 0, 62, 128, 5, 0, 0, 0, 0, 0, 8, 0, 0, 0, 4, 3, 0, 0, 97,
            141, 63, 2, 0, 0, 0, 0, 16, 0, 0, 0, 1, 6, 0, 0, 128, 0, 0, 0, 144, 0, 0, 0, 160, 0, 0,
            0, 176, 0, 0, 0, 8, 0, 0, 0, 0, 4, 0, 0, 65, 196, 52, 107, 21, 181, 253, 134, 24, 0, 0,
            0, 1, 2, 0, 0, 1, 0, 0, 0, 0, 0, 0, 0, 15, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0,
            8, 0, 0, 0, 4, 3, 0, 0, 2, 0, 0, 0, 0, 0, 0, 0, 4, 0, 0, 0, 3, 3, 0, 0, 245, 1, 0, 0,
            8, 0, 0, 0, 1, 9, 0, 0, 102, 105, 108, 101, 58, 47, 47, 47, 12, 0, 0, 0, 1, 1, 0, 0,
            77, 97, 99, 105, 110, 116, 111, 115, 104, 32, 72, 68, 8, 0, 0, 0, 4, 3, 0, 0, 0, 112,
            196, 208, 209, 1, 0, 0, 8, 0, 0, 0, 0, 4, 0, 0, 65, 195, 229, 4, 81, 128, 0, 0, 36, 0,
            0, 0, 1, 1, 0, 0, 57, 54, 70, 66, 52, 49, 67, 48, 45, 54, 67, 69, 57, 45, 52, 68, 65,
            50, 45, 56, 52, 51, 53, 45, 51, 53, 66, 67, 49, 57, 67, 55, 51, 53, 65, 51, 24, 0, 0,
            0, 1, 2, 0, 0, 129, 0, 0, 0, 1, 0, 0, 0, 239, 19, 0, 0, 1, 0, 0, 0, 0, 0, 0, 0, 0, 0,
            0, 0, 1, 0, 0, 0, 1, 1, 0, 0, 47, 0, 0, 0, 0, 0, 0, 0, 1, 5, 0, 0, 204, 0, 0, 0, 254,
            255, 255, 255, 1, 0, 0, 0, 0, 0, 0, 0, 16, 0, 0, 0, 4, 16, 0, 0, 104, 0, 0, 0, 0, 0, 0,
            0, 5, 16, 0, 0, 192, 0, 0, 0, 0, 0, 0, 0, 16, 16, 0, 0, 232, 0, 0, 0, 0, 0, 0, 0, 64,
            16, 0, 0, 216, 0, 0, 0, 0, 0, 0, 0, 2, 32, 0, 0, 180, 1, 0, 0, 0, 0, 0, 0, 5, 32, 0, 0,
            36, 1, 0, 0, 0, 0, 0, 0, 16, 32, 0, 0, 52, 1, 0, 0, 0, 0, 0, 0, 17, 32, 0, 0, 104, 1,
            0, 0, 0, 0, 0, 0, 18, 32, 0, 0, 72, 1, 0, 0, 0, 0, 0, 0, 19, 32, 0, 0, 88, 1, 0, 0, 0,
            0, 0, 0, 32, 32, 0, 0, 148, 1, 0, 0, 0, 0, 0, 0, 48, 32, 0, 0, 192, 1, 0, 0, 0, 0, 0,
            0, 1, 192, 0, 0, 8, 1, 0, 0, 0, 0, 0, 0, 17, 192, 0, 0, 32, 0, 0, 0, 0, 0, 0, 0, 18,
            192, 0, 0, 24, 1, 0, 0, 0, 0, 0, 0, 16, 208, 0, 0, 4, 0, 0, 0, 0, 0, 0, 0,
        ];
        assert_eq!(results[1].bookmark_blob, bookmark_blob);
        assert_eq!(results[1].download_entry_progress_total_to_load, 66784330);
        assert_eq!(results[1].download_entry_progress_bytes_so_far, 66784330);
        assert_eq!(results[1].download_entry_date_added_key, 1656266411);
        assert_eq!(results[1].download_entry_date_finished_key, 1656266415);
        assert_eq!(
            results[1].download_entry_should_use_request_url_as_origin,
            false
        );
        assert_eq!(
            results[1].download_identifier,
            "8AD5A4E7-BAF8-41EF-9AEF-2141132C68A0"
        );
        assert_eq!(results[1].download_url, "https://objects.githubusercontent.com/github-production-release-asset-2e65be/49609581/dd9e0ebc-a22a-4d10-bf74-0bf7dd26cea9?X-Amz-Algorithm=AWS4-HMAC-SHA256&X-Amz-Credential=AKIAIWNJYAX4CSVEH53A%2F20220626%2Fus-east-1%2Fs3%2Faws4_request&X-Amz-Date=20220626T180020Z&X-Amz-Expires=300&X-Amz-Signature=2eae2eebef5d244239929d7d4bd5e10690cb27518ed6f1ff79d79ee49c9cfe4d&X-Amz-SignedHeaders=host&actor_id=0&key_id=0&repo_id=49609581&response-content-disposition=attachment%3B%20filename%3Dpowershell-7.2.5-osx-x64.pkg&response-content-type=application%2Foctet-stream");
        assert_eq!(
            results[1].download_path,
            "/Users/puffycid/Downloads/powershell-7.2.5-osx-x64.pkg"
        );
        assert_eq!(
            results[1].download_sandbox_id,
            "17FAE646-5D9F-4CF3-877D-EB9C64133134"
        );
        assert_eq!(results[1].download_remove_when_done, false);

        let bookmark_blob = [
            98, 111, 111, 107, 204, 2, 0, 0, 0, 0, 4, 16, 48, 0, 0, 0, 25, 80, 214, 97, 233, 70,
            167, 130, 97, 151, 138, 239, 48, 170, 54, 73, 84, 24, 30, 240, 159, 186, 31, 177, 18,
            151, 125, 25, 108, 224, 134, 3, 200, 1, 0, 0, 4, 0, 0, 0, 3, 3, 0, 0, 0, 24, 0, 40, 5,
            0, 0, 0, 1, 1, 0, 0, 85, 115, 101, 114, 115, 0, 0, 0, 8, 0, 0, 0, 1, 1, 0, 0, 112, 117,
            102, 102, 121, 99, 105, 100, 9, 0, 0, 0, 1, 1, 0, 0, 68, 111, 119, 110, 108, 111, 97,
            100, 115, 0, 0, 0, 28, 0, 0, 0, 1, 1, 0, 0, 112, 111, 119, 101, 114, 115, 104, 101,
            108, 108, 45, 55, 46, 50, 46, 53, 45, 111, 115, 120, 45, 120, 54, 52, 46, 112, 107,
            103, 16, 0, 0, 0, 1, 6, 0, 0, 16, 0, 0, 0, 32, 0, 0, 0, 48, 0, 0, 0, 68, 0, 0, 0, 8, 0,
            0, 0, 4, 3, 0, 0, 79, 83, 0, 0, 0, 0, 0, 0, 8, 0, 0, 0, 4, 3, 0, 0, 11, 128, 5, 0, 0,
            0, 0, 0, 8, 0, 0, 0, 4, 3, 0, 0, 62, 128, 5, 0, 0, 0, 0, 0, 8, 0, 0, 0, 4, 3, 0, 0, 80,
            130, 63, 2, 0, 0, 0, 0, 16, 0, 0, 0, 1, 6, 0, 0, 128, 0, 0, 0, 144, 0, 0, 0, 160, 0, 0,
            0, 176, 0, 0, 0, 8, 0, 0, 0, 0, 4, 0, 0, 65, 196, 52, 105, 33, 7, 26, 85, 24, 0, 0, 0,
            1, 2, 0, 0, 1, 0, 0, 0, 0, 0, 0, 0, 15, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 8,
            0, 0, 0, 4, 3, 0, 0, 2, 0, 0, 0, 0, 0, 0, 0, 4, 0, 0, 0, 3, 3, 0, 0, 245, 1, 0, 0, 8,
            0, 0, 0, 1, 9, 0, 0, 102, 105, 108, 101, 58, 47, 47, 47, 12, 0, 0, 0, 1, 1, 0, 0, 77,
            97, 99, 105, 110, 116, 111, 115, 104, 32, 72, 68, 8, 0, 0, 0, 4, 3, 0, 0, 0, 112, 196,
            208, 209, 1, 0, 0, 8, 0, 0, 0, 0, 4, 0, 0, 65, 195, 229, 4, 81, 128, 0, 0, 36, 0, 0, 0,
            1, 1, 0, 0, 57, 54, 70, 66, 52, 49, 67, 48, 45, 54, 67, 69, 57, 45, 52, 68, 65, 50, 45,
            56, 52, 51, 53, 45, 51, 53, 66, 67, 49, 57, 67, 55, 51, 53, 65, 51, 24, 0, 0, 0, 1, 2,
            0, 0, 129, 0, 0, 0, 1, 0, 0, 0, 239, 19, 0, 0, 1, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 1,
            0, 0, 0, 1, 1, 0, 0, 47, 0, 0, 0, 0, 0, 0, 0, 1, 5, 0, 0, 204, 0, 0, 0, 254, 255, 255,
            255, 1, 0, 0, 0, 0, 0, 0, 0, 16, 0, 0, 0, 4, 16, 0, 0, 104, 0, 0, 0, 0, 0, 0, 0, 5, 16,
            0, 0, 192, 0, 0, 0, 0, 0, 0, 0, 16, 16, 0, 0, 232, 0, 0, 0, 0, 0, 0, 0, 64, 16, 0, 0,
            216, 0, 0, 0, 0, 0, 0, 0, 2, 32, 0, 0, 180, 1, 0, 0, 0, 0, 0, 0, 5, 32, 0, 0, 36, 1, 0,
            0, 0, 0, 0, 0, 16, 32, 0, 0, 52, 1, 0, 0, 0, 0, 0, 0, 17, 32, 0, 0, 104, 1, 0, 0, 0, 0,
            0, 0, 18, 32, 0, 0, 72, 1, 0, 0, 0, 0, 0, 0, 19, 32, 0, 0, 88, 1, 0, 0, 0, 0, 0, 0, 32,
            32, 0, 0, 148, 1, 0, 0, 0, 0, 0, 0, 48, 32, 0, 0, 192, 1, 0, 0, 0, 0, 0, 0, 1, 192, 0,
            0, 8, 1, 0, 0, 0, 0, 0, 0, 17, 192, 0, 0, 32, 0, 0, 0, 0, 0, 0, 0, 18, 192, 0, 0, 24,
            1, 0, 0, 0, 0, 0, 0, 16, 208, 0, 0, 4, 0, 0, 0, 0, 0, 0, 0,
        ];
        assert_eq!(results[2].bookmark_blob, bookmark_blob);
        assert_eq!(results[2].download_entry_progress_total_to_load, 66784330);
        assert_eq!(results[2].download_entry_progress_bytes_so_far, 66784330);
        assert_eq!(results[2].download_entry_date_added_key, 1656265410);
        assert_eq!(results[2].download_entry_date_finished_key, 1656265414);
        assert_eq!(
            results[2].download_entry_should_use_request_url_as_origin,
            false
        );
        assert_eq!(
            results[2].download_identifier,
            "FA11CA6A-3A6D-46AF-89EA-5BF3ECDB8907"
        );
        assert_eq!(results[2].download_url, "https://objects.githubusercontent.com/github-production-release-asset-2e65be/49609581/dd9e0ebc-a22a-4d10-bf74-0bf7dd26cea9?X-Amz-Algorithm=AWS4-HMAC-SHA256&X-Amz-Credential=AKIAIWNJYAX4CSVEH53A%2F20220626%2Fus-east-1%2Fs3%2Faws4_request&X-Amz-Date=20220626T174338Z&X-Amz-Expires=300&X-Amz-Signature=d43536a1b5a74ad6d64f32a4611d0fdb1f7fb543d8b6327bcbe2d5ce4b9a7428&X-Amz-SignedHeaders=host&actor_id=0&key_id=0&repo_id=49609581&response-content-disposition=attachment%3B%20filename%3Dpowershell-7.2.5-osx-x64.pkg&response-content-type=application%2Foctet-stream");
        assert_eq!(
            results[2].download_path,
            "/Users/puffycid/Downloads/powershell-7.2.5-osx-x64.pkg"
        );
        assert_eq!(
            results[2].download_sandbox_id,
            "7FFFC852-3A18-4978-8949-0CFFABF7776A"
        );
        assert_eq!(results[2].download_remove_when_done, false);
    }

    #[test]
    fn test_get_array_values() {
        let mut test_location = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
        test_location.push("tests/test_data/browser/safari/Downloads.plist");

        let downloads: Dictionary = plist::from_file(test_location.display().to_string()).unwrap();
        let mut results: Vec<DownloadsPlist> = Vec::new();
        for (key, value) in downloads {
            if key != "DownloadHistory" {
                continue;
            }

            match value {
                Value::Array(_) => {
                    // Parse the array of dictionaries
                    results = DownloadsPlist::get_array_values(value).unwrap();
                }
                _ => {}
            }
        }
        let bookmark_blob = [
            98, 111, 111, 107, 208, 2, 0, 0, 0, 0, 4, 16, 48, 0, 0, 0, 56, 108, 226, 233, 179, 84,
            237, 152, 124, 140, 84, 89, 175, 6, 172, 193, 190, 107, 33, 234, 153, 156, 121, 80,
            209, 98, 72, 198, 97, 36, 76, 221, 204, 1, 0, 0, 4, 0, 0, 0, 3, 3, 0, 0, 0, 24, 0, 40,
            5, 0, 0, 0, 1, 1, 0, 0, 85, 115, 101, 114, 115, 0, 0, 0, 8, 0, 0, 0, 1, 1, 0, 0, 112,
            117, 102, 102, 121, 99, 105, 100, 9, 0, 0, 0, 1, 1, 0, 0, 68, 111, 119, 110, 108, 111,
            97, 100, 115, 0, 0, 0, 30, 0, 0, 0, 1, 1, 0, 0, 112, 111, 119, 101, 114, 115, 104, 101,
            108, 108, 45, 55, 46, 50, 46, 53, 45, 111, 115, 120, 45, 97, 114, 109, 54, 52, 46, 112,
            107, 103, 0, 0, 16, 0, 0, 0, 1, 6, 0, 0, 16, 0, 0, 0, 32, 0, 0, 0, 48, 0, 0, 0, 68, 0,
            0, 0, 8, 0, 0, 0, 4, 3, 0, 0, 79, 83, 0, 0, 0, 0, 0, 0, 8, 0, 0, 0, 4, 3, 0, 0, 11,
            128, 5, 0, 0, 0, 0, 0, 8, 0, 0, 0, 4, 3, 0, 0, 62, 128, 5, 0, 0, 0, 0, 0, 8, 0, 0, 0,
            4, 3, 0, 0, 104, 141, 63, 2, 0, 0, 0, 0, 16, 0, 0, 0, 1, 6, 0, 0, 132, 0, 0, 0, 148, 0,
            0, 0, 164, 0, 0, 0, 180, 0, 0, 0, 8, 0, 0, 0, 0, 4, 0, 0, 65, 196, 52, 107, 24, 237,
            13, 97, 24, 0, 0, 0, 1, 2, 0, 0, 1, 0, 0, 0, 0, 0, 0, 0, 15, 0, 0, 0, 0, 0, 0, 0, 0, 0,
            0, 0, 0, 0, 0, 0, 8, 0, 0, 0, 4, 3, 0, 0, 2, 0, 0, 0, 0, 0, 0, 0, 4, 0, 0, 0, 3, 3, 0,
            0, 245, 1, 0, 0, 8, 0, 0, 0, 1, 9, 0, 0, 102, 105, 108, 101, 58, 47, 47, 47, 12, 0, 0,
            0, 1, 1, 0, 0, 77, 97, 99, 105, 110, 116, 111, 115, 104, 32, 72, 68, 8, 0, 0, 0, 4, 3,
            0, 0, 0, 112, 196, 208, 209, 1, 0, 0, 8, 0, 0, 0, 0, 4, 0, 0, 65, 195, 229, 4, 81, 128,
            0, 0, 36, 0, 0, 0, 1, 1, 0, 0, 57, 54, 70, 66, 52, 49, 67, 48, 45, 54, 67, 69, 57, 45,
            52, 68, 65, 50, 45, 56, 52, 51, 53, 45, 51, 53, 66, 67, 49, 57, 67, 55, 51, 53, 65, 51,
            24, 0, 0, 0, 1, 2, 0, 0, 129, 0, 0, 0, 1, 0, 0, 0, 239, 19, 0, 0, 1, 0, 0, 0, 0, 0, 0,
            0, 0, 0, 0, 0, 1, 0, 0, 0, 1, 1, 0, 0, 47, 0, 0, 0, 0, 0, 0, 0, 1, 5, 0, 0, 204, 0, 0,
            0, 254, 255, 255, 255, 1, 0, 0, 0, 0, 0, 0, 0, 16, 0, 0, 0, 4, 16, 0, 0, 108, 0, 0, 0,
            0, 0, 0, 0, 5, 16, 0, 0, 196, 0, 0, 0, 0, 0, 0, 0, 16, 16, 0, 0, 236, 0, 0, 0, 0, 0, 0,
            0, 64, 16, 0, 0, 220, 0, 0, 0, 0, 0, 0, 0, 2, 32, 0, 0, 184, 1, 0, 0, 0, 0, 0, 0, 5,
            32, 0, 0, 40, 1, 0, 0, 0, 0, 0, 0, 16, 32, 0, 0, 56, 1, 0, 0, 0, 0, 0, 0, 17, 32, 0, 0,
            108, 1, 0, 0, 0, 0, 0, 0, 18, 32, 0, 0, 76, 1, 0, 0, 0, 0, 0, 0, 19, 32, 0, 0, 92, 1,
            0, 0, 0, 0, 0, 0, 32, 32, 0, 0, 152, 1, 0, 0, 0, 0, 0, 0, 48, 32, 0, 0, 196, 1, 0, 0,
            0, 0, 0, 0, 1, 192, 0, 0, 12, 1, 0, 0, 0, 0, 0, 0, 17, 192, 0, 0, 32, 0, 0, 0, 0, 0, 0,
            0, 18, 192, 0, 0, 28, 1, 0, 0, 0, 0, 0, 0, 16, 208, 0, 0, 4, 0, 0, 0, 0, 0, 0, 0,
        ];
        assert_eq!(results[0].bookmark_blob, bookmark_blob);
        assert_eq!(results[0].download_entry_progress_total_to_load, 63055607);
        assert_eq!(results[0].download_entry_progress_bytes_so_far, 63055607);
        assert_eq!(results[0].download_entry_date_added_key, 1656266417);
        assert_eq!(results[0].download_entry_date_finished_key, 1656266422);
        assert_eq!(
            results[0].download_entry_should_use_request_url_as_origin,
            false
        );
        assert_eq!(
            results[0].download_identifier,
            "835D414A-492E-4DBB-BD6B-E8FACD4ED84D"
        );
        assert_eq!(results[0].download_url, "https://objects.githubusercontent.com/github-production-release-asset-2e65be/49609581/97b2b465-4242-42c6-ae6f-16437ee71f12?X-Amz-Algorithm=AWS4-HMAC-SHA256&X-Amz-Credential=AKIAIWNJYAX4CSVEH53A%2F20220626%2Fus-east-1%2Fs3%2Faws4_request&X-Amz-Date=20220626T180026Z&X-Amz-Expires=300&X-Amz-Signature=7f403834d25930916a71894a1960b7624e6479cdd493c40b96644d4a01ffdf41&X-Amz-SignedHeaders=host&actor_id=0&key_id=0&repo_id=49609581&response-content-disposition=attachment%3B%20filename%3Dpowershell-7.2.5-osx-arm64.pkg&response-content-type=application%2Foctet-stream");
        assert_eq!(
            results[0].download_path,
            "/Users/puffycid/Downloads/powershell-7.2.5-osx-arm64.pkg"
        );
        assert_eq!(
            results[0].download_sandbox_id,
            "DBA9EBA4-D23B-43C5-9DEB-131566E7BD8B"
        );
        assert_eq!(results[0].download_remove_when_done, false);

        let bookmark_blob = [
            98, 111, 111, 107, 204, 2, 0, 0, 0, 0, 4, 16, 48, 0, 0, 0, 118, 226, 194, 136, 16, 92,
            78, 163, 243, 45, 21, 244, 123, 52, 133, 130, 108, 22, 210, 185, 190, 79, 227, 3, 111,
            161, 134, 159, 76, 39, 205, 112, 200, 1, 0, 0, 4, 0, 0, 0, 3, 3, 0, 0, 0, 24, 0, 40, 5,
            0, 0, 0, 1, 1, 0, 0, 85, 115, 101, 114, 115, 0, 0, 0, 8, 0, 0, 0, 1, 1, 0, 0, 112, 117,
            102, 102, 121, 99, 105, 100, 9, 0, 0, 0, 1, 1, 0, 0, 68, 111, 119, 110, 108, 111, 97,
            100, 115, 0, 0, 0, 28, 0, 0, 0, 1, 1, 0, 0, 112, 111, 119, 101, 114, 115, 104, 101,
            108, 108, 45, 55, 46, 50, 46, 53, 45, 111, 115, 120, 45, 120, 54, 52, 46, 112, 107,
            103, 16, 0, 0, 0, 1, 6, 0, 0, 16, 0, 0, 0, 32, 0, 0, 0, 48, 0, 0, 0, 68, 0, 0, 0, 8, 0,
            0, 0, 4, 3, 0, 0, 79, 83, 0, 0, 0, 0, 0, 0, 8, 0, 0, 0, 4, 3, 0, 0, 11, 128, 5, 0, 0,
            0, 0, 0, 8, 0, 0, 0, 4, 3, 0, 0, 62, 128, 5, 0, 0, 0, 0, 0, 8, 0, 0, 0, 4, 3, 0, 0, 97,
            141, 63, 2, 0, 0, 0, 0, 16, 0, 0, 0, 1, 6, 0, 0, 128, 0, 0, 0, 144, 0, 0, 0, 160, 0, 0,
            0, 176, 0, 0, 0, 8, 0, 0, 0, 0, 4, 0, 0, 65, 196, 52, 107, 21, 181, 253, 134, 24, 0, 0,
            0, 1, 2, 0, 0, 1, 0, 0, 0, 0, 0, 0, 0, 15, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0,
            8, 0, 0, 0, 4, 3, 0, 0, 2, 0, 0, 0, 0, 0, 0, 0, 4, 0, 0, 0, 3, 3, 0, 0, 245, 1, 0, 0,
            8, 0, 0, 0, 1, 9, 0, 0, 102, 105, 108, 101, 58, 47, 47, 47, 12, 0, 0, 0, 1, 1, 0, 0,
            77, 97, 99, 105, 110, 116, 111, 115, 104, 32, 72, 68, 8, 0, 0, 0, 4, 3, 0, 0, 0, 112,
            196, 208, 209, 1, 0, 0, 8, 0, 0, 0, 0, 4, 0, 0, 65, 195, 229, 4, 81, 128, 0, 0, 36, 0,
            0, 0, 1, 1, 0, 0, 57, 54, 70, 66, 52, 49, 67, 48, 45, 54, 67, 69, 57, 45, 52, 68, 65,
            50, 45, 56, 52, 51, 53, 45, 51, 53, 66, 67, 49, 57, 67, 55, 51, 53, 65, 51, 24, 0, 0,
            0, 1, 2, 0, 0, 129, 0, 0, 0, 1, 0, 0, 0, 239, 19, 0, 0, 1, 0, 0, 0, 0, 0, 0, 0, 0, 0,
            0, 0, 1, 0, 0, 0, 1, 1, 0, 0, 47, 0, 0, 0, 0, 0, 0, 0, 1, 5, 0, 0, 204, 0, 0, 0, 254,
            255, 255, 255, 1, 0, 0, 0, 0, 0, 0, 0, 16, 0, 0, 0, 4, 16, 0, 0, 104, 0, 0, 0, 0, 0, 0,
            0, 5, 16, 0, 0, 192, 0, 0, 0, 0, 0, 0, 0, 16, 16, 0, 0, 232, 0, 0, 0, 0, 0, 0, 0, 64,
            16, 0, 0, 216, 0, 0, 0, 0, 0, 0, 0, 2, 32, 0, 0, 180, 1, 0, 0, 0, 0, 0, 0, 5, 32, 0, 0,
            36, 1, 0, 0, 0, 0, 0, 0, 16, 32, 0, 0, 52, 1, 0, 0, 0, 0, 0, 0, 17, 32, 0, 0, 104, 1,
            0, 0, 0, 0, 0, 0, 18, 32, 0, 0, 72, 1, 0, 0, 0, 0, 0, 0, 19, 32, 0, 0, 88, 1, 0, 0, 0,
            0, 0, 0, 32, 32, 0, 0, 148, 1, 0, 0, 0, 0, 0, 0, 48, 32, 0, 0, 192, 1, 0, 0, 0, 0, 0,
            0, 1, 192, 0, 0, 8, 1, 0, 0, 0, 0, 0, 0, 17, 192, 0, 0, 32, 0, 0, 0, 0, 0, 0, 0, 18,
            192, 0, 0, 24, 1, 0, 0, 0, 0, 0, 0, 16, 208, 0, 0, 4, 0, 0, 0, 0, 0, 0, 0,
        ];
        assert_eq!(results[1].bookmark_blob, bookmark_blob);
        assert_eq!(results[1].download_entry_progress_total_to_load, 66784330);
        assert_eq!(results[1].download_entry_progress_bytes_so_far, 66784330);
        assert_eq!(results[1].download_entry_date_added_key, 1656266411);
        assert_eq!(results[1].download_entry_date_finished_key, 1656266415);
        assert_eq!(
            results[1].download_entry_should_use_request_url_as_origin,
            false
        );
        assert_eq!(
            results[1].download_identifier,
            "8AD5A4E7-BAF8-41EF-9AEF-2141132C68A0"
        );
        assert_eq!(results[1].download_url, "https://objects.githubusercontent.com/github-production-release-asset-2e65be/49609581/dd9e0ebc-a22a-4d10-bf74-0bf7dd26cea9?X-Amz-Algorithm=AWS4-HMAC-SHA256&X-Amz-Credential=AKIAIWNJYAX4CSVEH53A%2F20220626%2Fus-east-1%2Fs3%2Faws4_request&X-Amz-Date=20220626T180020Z&X-Amz-Expires=300&X-Amz-Signature=2eae2eebef5d244239929d7d4bd5e10690cb27518ed6f1ff79d79ee49c9cfe4d&X-Amz-SignedHeaders=host&actor_id=0&key_id=0&repo_id=49609581&response-content-disposition=attachment%3B%20filename%3Dpowershell-7.2.5-osx-x64.pkg&response-content-type=application%2Foctet-stream");
        assert_eq!(
            results[1].download_path,
            "/Users/puffycid/Downloads/powershell-7.2.5-osx-x64.pkg"
        );
        assert_eq!(
            results[1].download_sandbox_id,
            "17FAE646-5D9F-4CF3-877D-EB9C64133134"
        );
        assert_eq!(results[1].download_remove_when_done, false);
    }

    #[test]
    fn test_get_safari_timestamp() {
        let test: Value = Value::Date(plist::Date::from(UNIX_EPOCH));
        let results = DownloadsPlist::get_safari_timestamp(&test);

        assert_eq!(results, 0);
    }
}
