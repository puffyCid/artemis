use super::error::ChromiumHistoryError;
use crate::{
    filesystem::directory::get_user_paths,
    utils::time::{unixepoch_to_iso, webkit_time_to_unixepoch},
};
use common::applications::{ChromiumDownload, ChromiumDownloads};
use log::{error, warn};
use rusqlite::{Connection, OpenFlags};
use std::path::Path;

/// Get `Chromium` SQLITE History file for all users to get browser downloads
pub(crate) fn get_chromium_downloads() -> Result<Vec<ChromiumDownloads>, ChromiumHistoryError> {
    // Get all user directories
    let user_paths_result = get_user_paths();
    let user_paths = match user_paths_result {
        Ok(result) => result,
        Err(err) => {
            error!("[chromium] Failed to get user paths: {err:?}");
            return Err(ChromiumHistoryError::PathError);
        }
    };
    let mut chromium_downloads: Vec<ChromiumDownloads> = Vec::new();

    // Check for Chromium profiles for all users
    for users in user_paths {
        #[cfg(target_os = "macos")]
        let chromium_path = Path::new(&format!(
            "{users}/Library/Application Support/Chromium/Default/History"
        ))
        .to_path_buf();
        #[cfg(target_os = "windows")]
        let chromium_path = Path::new(&format!(
            "{users}\\AppData\\Local\\Chromium\\User Data\\Default\\History"
        ))
        .to_path_buf();

        #[cfg(target_os = "linux")]
        let chromium_path =
            Path::new(&format!("{users}/.config/chromium/Default/History")).to_path_buf();

        // Verify if History file is on disk
        if !chromium_path.is_file() {
            continue;
        }
        let path = chromium_path.display().to_string();
        let downloads = downloads_query(&path)?;
        let user;

        #[cfg(target_os = "macos")]
        {
            user = users.replace("/Users/", "");
        }

        #[cfg(target_os = "windows")]
        {
            let user_data: Vec<&str> = users.split('\\').collect();
            user = (*user_data.last().unwrap_or(&"")).to_string();
        }
        #[cfg(target_os = "linux")]
        {
            let user_data: Vec<&str> = users.split('/').collect();
            user = (*user_data.last().unwrap_or(&"")).to_string();
        }

        let downloads_data = ChromiumDownloads {
            downloads,
            path,
            user,
        };
        chromium_downloads.push(downloads_data);
    }

    Ok(chromium_downloads)
}

/// Query the downloads history tables
pub(crate) fn downloads_query(path: &str) -> Result<Vec<ChromiumDownload>, ChromiumHistoryError> {
    // Bypass SQLITE file lock
    let downloads_file = format!("file:{path}?immutable=1");
    let connection = Connection::open_with_flags(
        downloads_file,
        OpenFlags::SQLITE_OPEN_READ_ONLY | OpenFlags::SQLITE_OPEN_URI,
    );
    let conn = match connection {
        Ok(connect) => connect,
        Err(err) => {
            error!("[chromium]  Failed to read Chromium SQLITE history file {err:?}");
            return Err(ChromiumHistoryError::SQLITEParseError);
        }
    };

    let statement = conn.prepare("SELECT downloads.id as downloads_id, guid, current_path, target_path, start_time, received_bytes, total_bytes, state, danger_type, 
        interrupt_reason, hash, end_time, opened, last_access_time, transient, referrer, site_url, tab_url, tab_referrer_url, http_method, by_ext_id, by_ext_name, etag, last_modified, 
        mime_type, original_mime_type, downloads_url_chains.id as downloads_url_chain_id, chain_index, url FROM downloads join downloads_url_chains on downloads_url_chains.id = downloads.id");
    let mut stmt = match statement {
        Ok(query) => query,
        Err(err) => {
            error!("[chromium]  Failed to compose Chromium Downloads SQL query {err:?}");
            return Err(ChromiumHistoryError::BadSQL);
        }
    };

    // Get browser downloads data
    let downloads_data = stmt.query_map([], |row| {
        Ok(ChromiumDownload {
            id: row.get("downloads_id")?,
            guid: row.get("guid")?,
            current_path: row.get("current_path")?,
            target_path: row.get("target_path")?,
            start_time: {
                let value: i64 = row.get("start_time")?;
                let adjust_time = 1000000;
                unixepoch_to_iso(&webkit_time_to_unixepoch(&(value / adjust_time)))
            },
            received_bytes: row.get("received_bytes")?,
            total_bytes: row.get("total_bytes")?,
            state: row.get("state")?,
            danger_type: row.get("danger_type")?,
            interrupt_reason: row.get("interrupt_reason")?,
            hash: row.get("hash")?,
            end_time: {
                let value: i64 = row.get("end_time")?;
                let adjust_time = 1000000;
                unixepoch_to_iso(&webkit_time_to_unixepoch(&(value / adjust_time)))
            },
            opened: row.get("opened")?,
            last_access_time: {
                let value: i64 = row.get("last_access_time")?;
                let adjust_time = 1000000;
                unixepoch_to_iso(&webkit_time_to_unixepoch(&(value / adjust_time)))
            },
            transient: row.get("transient")?,
            referrer: row.get("referrer")?,
            site_url: row.get("site_url")?,
            tab_url: row.get("tab_url")?,
            tab_referrer_url: row.get("tab_referrer_url")?,
            http_method: row.get("http_method")?,
            by_ext_id: row.get("by_ext_id")?,
            by_ext_name: row.get("by_ext_name")?,
            etag: row.get("etag")?,
            last_modified: row.get("last_modified")?,
            mime_type: row.get("mime_type")?,
            original_mime_type: row.get("original_mime_type")?,
            downloads_url_chain_id: row.get("downloads_url_chain_id")?,
            chain_index: row.get("chain_index")?,
            url: row.get("url")?,
        })
    });

    match downloads_data {
        Ok(download_iter) => {
            let mut download_vec: Vec<ChromiumDownload> = Vec::new();
            for download in download_iter {
                match download {
                    Ok(download_data) => {
                        download_vec.push(download_data);
                    }
                    Err(err) => {
                        warn!("[chromium] Failed to iterate Chromium download data: {err:?}");
                    }
                }
            }

            Ok(download_vec)
        }
        Err(err) => {
            error!("[chromium] Failed to get Chromium download data: {err:?}");
            Err(ChromiumHistoryError::SQLITEParseError)
        }
    }
}

#[cfg(test)]
mod tests {
    use super::get_chromium_downloads;
    use crate::artifacts::applications::chromium::downloads::downloads_query;
    use std::path::PathBuf;

    #[test]
    fn test_get_chromium_downloads() {
        let _result = get_chromium_downloads().unwrap();
    }

    #[test]
    fn test_downloads_query() {
        let mut test_location = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
        test_location.push("tests/test_data/browser/chromium/History");
        let test_path: &str = &test_location.display().to_string();
        let results = downloads_query(test_path).unwrap();
        assert!(results.len() > 1);

        assert_eq!(results[0].id, 1);
        assert_eq!(results[0].guid, "6c36a638-a5a6-4bf6-97fc-2859cb003a1f");
        assert_eq!(
            results[0].current_path,
            "/home/ubunty/Downloads/PowerShell-7.2.1-win-arm64.zip"
        );
        assert_eq!(
            results[0].target_path,
            "/home/ubunty/Downloads/PowerShell-7.2.1-win-arm64.zip"
        );
        assert_eq!(results[0].start_time, "2022-02-22T06:12:40.000Z");
        assert_eq!(results[0].received_bytes, 68907014);
        assert_eq!(results[0].state, 1);
        assert_eq!(results[0].danger_type, 0);
        assert_eq!(results[0].interrupt_reason, 0);
        assert_eq!(results[0].end_time, "2022-02-22T06:12:45.000Z");
        assert_eq!(results[0].opened, 0);
        assert_eq!(results[0].last_access_time, "1601-01-01T00:00:00.000Z");
        assert_eq!(
            results[0].referrer,
            "https://github.com/PowerShell/PowerShell/releases/tag/v7.2.1"
        );
        assert_eq!(results[0].site_url, "");
        assert_eq!(
            results[0].tab_url,
            "https://github.com/PowerShell/PowerShell/releases/tag/v7.2.1"
        );
        assert_eq!(results[0].tab_referrer_url, "https://docs.microsoft.com/");
        assert_eq!(results[0].http_method, "");
        assert_eq!(results[0].by_ext_id, "");
        assert_eq!(results[0].by_ext_name, "");
        assert_eq!(results[0].etag, "\"0x8D9BF2C4646CBEC\"");
        assert_eq!(results[0].last_modified, "Tue, 14 Dec 2021 18:05:12 GMT");
        assert_eq!(results[0].mime_type, "application/octet-stream");
        assert_eq!(results[0].original_mime_type, "application/octet-stream");
        assert_eq!(results[0].downloads_url_chain_id, 1);
        assert_eq!(results[0].chain_index, 0);
        assert_eq!(results[0].url, "https://github.com/PowerShell/PowerShell/releases/download/v7.2.1/PowerShell-7.2.1-win-arm64.zip");
    }
}
