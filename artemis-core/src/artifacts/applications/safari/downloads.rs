use super::{error::SafariError, plist::DownloadsPlist};
use crate::{
    artifacts::os::macos::bookmarks::parser::parse_bookmark,
    filesystem::{directory::get_user_paths, files::is_file},
};
use log::error;
use serde::Serialize;

#[derive(Debug, Serialize)]
pub(crate) struct Downloads {
    pub(crate) source_url: String,
    pub(crate) download_path: String,
    pub(crate) sandbox_id: String,
    pub(crate) download_bytes: i64,
    pub(crate) download_id: String,
    pub(crate) download_entry_date: u64,
    pub(crate) download_entry_finish: u64,
    pub(crate) path: Vec<String>,             // Path to binary to run
    pub(crate) cnid_path: Vec<i64>,           // Path represented as Catalog Node ID
    pub(crate) created: i64,                  // Created timestamp of binary target
    pub(crate) volume_path: String,           // Root
    pub(crate) volume_url: String,            // URL type
    pub(crate) volume_name: String,           // Name of Volume
    pub(crate) volume_uuid: String,           // Volume UUID string
    pub(crate) volume_size: i64,              // Size of Volume
    pub(crate) volume_created: i64,           // Created timestamp of Volume
    pub(crate) volume_flag: Vec<u64>,         // Volume Property flags
    pub(crate) volume_root: bool,             // If Volume is filesystem root
    pub(crate) localized_name: String,        // Optional localized name of target binary
    pub(crate) security_extension_rw: String, // Optional Security extension of target binary
    pub(crate) security_extension_ro: String, // Optional Security extension of target binary
    pub(crate) target_flags: Vec<u64>,        // Resource property flags
    pub(crate) username: String,              // Username related to bookmark
    pub(crate) folder_index: i64,             // Folder index number
    pub(crate) uid: i32,                      // User UID
    pub(crate) creation_options: i32,         // Bookmark creation options
    pub(crate) is_executable: bool,           // Can target be executed
    pub(crate) file_ref_flag: bool,
}

#[derive(Debug, Serialize)]
pub(crate) struct SafariDownloads {
    pub(crate) results: Vec<Downloads>,
    pub(crate) path: String,
    pub(crate) user: String,
}

impl SafariDownloads {
    /// Get Safari Downloads PLIST file for all users
    pub(crate) fn get_downloads() -> Result<Vec<SafariDownloads>, SafariError> {
        // Get all user directories
        let user_paths_result = get_user_paths();
        let user_paths = match user_paths_result {
            Ok(result) => result,
            Err(err) => {
                error!("[safari] Failed to get user paths: {err:?}");
                return Err(SafariError::PathError);
            }
        };

        let downloads_path = "/Library/Safari/Downloads.plist";
        let mut safari_downloads: Vec<SafariDownloads> = Vec::new();
        for users in user_paths {
            let path = format!("{users}{downloads_path}");
            if !is_file(&path) {
                continue;
            }

            let results = SafariDownloads::downloads_query(&path)?;
            let username = users.replace("/Users/", "");
            let downloads = SafariDownloads {
                results,
                path,
                user: username,
            };
            safari_downloads.push(downloads);
        }

        Ok(safari_downloads)
    }

    /// Parse the Safari Downloads PLIST file
    pub(crate) fn downloads_query(path: &str) -> Result<Vec<Downloads>, SafariError> {
        // Parse the initial binary PLIST file
        let downloads_results = DownloadsPlist::parse_safari_plist(path);
        let downloads_data = match downloads_results {
            Ok(results) => results,
            Err(err) => {
                error!("Failed to parse PLIST file at {path}: {err:?}");
                return Err(SafariError::Plist);
            }
        };
        let mut safari_downloads: Vec<Downloads> = Vec::new();

        for data in downloads_data {
            // Parse the Bookmarks blob. Contains similar data as the PLIST file
            let bookmark_results = parse_bookmark(&data.bookmark_blob);

            let bookmark = match bookmark_results {
                Ok(results) => results,
                Err(err) => {
                    error!("Failed to parse Safari downloads bookmark data at {path}: {err:?}");
                    return Err(SafariError::Bookmark);
                }
            };
            let safari_data = Downloads {
                source_url: data.download_url,
                download_path: data.download_path,
                sandbox_id: data.download_sandbox_id,
                download_bytes: data.download_entry_progress_total_to_load,
                download_id: data.download_identifier,
                download_entry_date: data.download_entry_date_added_key,
                download_entry_finish: data.download_entry_date_finished_key,
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
                is_executable: bookmark.is_executable,
                file_ref_flag: bookmark.file_ref_flag,
            };
            safari_downloads.push(safari_data);
        }
        Ok(safari_downloads)
    }
}

#[cfg(test)]
mod tests {
    use crate::artifacts::applications::safari::downloads::SafariDownloads;
    use std::path::PathBuf;

    #[test]
    #[ignore = "Get live users Safari downloads"]
    fn test_get_downloads() {
        let result = SafariDownloads::get_downloads().unwrap();
        assert!(result.len() > 0);
    }

    #[test]
    fn test_downloads_query() {
        let mut test_location = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
        test_location.push("tests/test_data/browser/safari/Downloads.plist");
        let test_path: &str = &test_location.display().to_string();
        let results = SafariDownloads::downloads_query(test_path).unwrap();
        assert_eq!(results.len(), 3);

        assert_eq!(results[0].source_url, "https://objects.githubusercontent.com/github-production-release-asset-2e65be/49609581/97b2b465-4242-42c6-ae6f-16437ee71f12?X-Amz-Algorithm=AWS4-HMAC-SHA256&X-Amz-Credential=AKIAIWNJYAX4CSVEH53A%2F20220626%2Fus-east-1%2Fs3%2Faws4_request&X-Amz-Date=20220626T180026Z&X-Amz-Expires=300&X-Amz-Signature=7f403834d25930916a71894a1960b7624e6479cdd493c40b96644d4a01ffdf41&X-Amz-SignedHeaders=host&actor_id=0&key_id=0&repo_id=49609581&response-content-disposition=attachment%3B%20filename%3Dpowershell-7.2.5-osx-arm64.pkg&response-content-type=application%2Foctet-stream");
        assert_eq!(
            results[0].download_path,
            "/Users/puffycid/Downloads/powershell-7.2.5-osx-arm64.pkg"
        );
        assert_eq!(
            results[0].sandbox_id,
            "DBA9EBA4-D23B-43C5-9DEB-131566E7BD8B"
        );
        assert_eq!(results[0].download_bytes, 63055607);
        assert_eq!(
            results[0].download_id,
            "835D414A-492E-4DBB-BD6B-E8FACD4ED84D"
        );
        assert_eq!(results[0].download_entry_date, 1656266417);
        assert_eq!(results[0].download_entry_finish, 1656266422);
        assert_eq!(
            results[0].path,
            [
                "Users",
                "puffycid",
                "Downloads",
                "powershell-7.2.5-osx-arm64.pkg"
            ]
        );
        assert_eq!(results[0].cnid_path, [21327, 360459, 360510, 37719400]);
        assert_eq!(results[0].volume_path, "/");
        assert_eq!(results[0].created, 1656266417);
        assert_eq!(results[0].volume_url, "file:///");
        assert_eq!(results[0].volume_name, "Macintosh HD");
        assert_eq!(
            results[0].volume_uuid,
            "96FB41C0-6CE9-4DA2-8435-35BC19C735A3"
        );
        assert_eq!(results[0].volume_size, 2000662327296);
        assert_eq!(results[0].volume_flag, [4294967425, 4294972399, 0]);
        assert_eq!(results[0].volume_created, 1645859107);
        assert_eq!(results[0].volume_root, true);
        assert_eq!(results[0].localized_name, "");
        assert_eq!(results[0].security_extension_ro, "");
        assert_eq!(results[0].security_extension_rw, "");
        assert_eq!(results[0].target_flags, [1, 15, 0]);
        assert_eq!(results[0].username, "puffycid");
        assert_eq!(results[0].folder_index, 2);
        assert_eq!(results[0].uid, 501);
        assert_eq!(results[0].creation_options, 671094784);
        assert_eq!(results[0].is_executable, false);
        assert_eq!(results[0].file_ref_flag, false);
    }
}
