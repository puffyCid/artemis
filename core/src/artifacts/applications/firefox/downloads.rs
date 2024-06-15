/**
 *  Parse Firefox Places SQLITE file
 *  Provides functions to parse Firefox Downloads data.
 * */
use super::{error::FirefoxHistoryError, history::user_data};
use crate::{filesystem::directory::get_user_paths, utils::time::unixepoch_microseconds_to_iso};
use common::applications::{FirefoxDownload, FirefoxDownloads, FirefoxHistoryEntry};
use log::{error, warn};
use rusqlite::{Connection, OpenFlags};
use std::path::Path;

/// Get `Firefox` downloads for users
pub(crate) fn get_firefox_downloads() -> Result<Vec<FirefoxDownloads>, FirefoxHistoryError> {
    // Get all user directories
    let user_paths_result = get_user_paths();
    let user_paths = match user_paths_result {
        Ok(result) => result,
        Err(err) => {
            error!("[firefox] Failed to get user paths: {err:?}");
            return Err(FirefoxHistoryError::PathError);
        }
    };
    let mut firefox_downloads: Vec<FirefoxDownloads> = Vec::new();

    // Check for Firefox profiles for all users
    for users in user_paths {
        #[cfg(target_os = "macos")]
        let firefox_path = Path::new(&format!(
            "{users}/Library/Application Support/Firefox/Profiles"
        ))
        .to_path_buf();
        #[cfg(target_os = "windows")]
        let firefox_path = Path::new(&format!(
            "{users}\\AppData\\Roaming\\Mozilla\\Firefox\\Profiles"
        ))
        .to_path_buf();

        #[cfg(target_os = "linux")]
        let firefox_path = Path::new(&format!("{users}/.mozilla/firefox")).to_path_buf();

        // Verify if Profile directory is on disk
        if !firefox_path.is_dir() {
            continue;
        }
        let firefox_data = user_data(&firefox_path);
        for path in firefox_data {
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

            let downloads_data = FirefoxDownloads {
                downloads,
                path,
                user,
            };
            firefox_downloads.push(downloads_data);
        }
    }
    Ok(firefox_downloads)
}

/// Query the downloads history tables
pub(crate) fn downloads_query(path: &str) -> Result<Vec<FirefoxDownload>, FirefoxHistoryError> {
    // Bypass SQLITE file lock
    let downloads_file = format!("file:{path}?immutable=1");
    let connection = Connection::open_with_flags(
        downloads_file,
        OpenFlags::SQLITE_OPEN_READ_ONLY | OpenFlags::SQLITE_OPEN_URI,
    );
    let conn = match connection {
        Ok(connect) => connect,
        Err(err) => {
            error!("[firefox] Failed to read Firefox file for downloads {err:?}");
            return Err(FirefoxHistoryError::SqliteParse);
        }
    };

    let statement = conn.prepare("SELECT moz_annos.id as downloads_id, place_id, anno_attribute_id, content, flags, expiration, type, dateAdded, lastModified, moz_places.id as moz_places_id, url, title, rev_host, visit_count, hidden, typed, last_visit_date, guid, foreign_count, url_hash, description, preview_image_url, name  FROM moz_annos join moz_places on moz_annos.place_id = moz_places.id join moz_anno_attributes on anno_attribute_id = moz_anno_attributes.id");
    let mut stmt = match statement {
        Ok(query) => query,
        Err(err) => {
            error!("[firefox] Failed to compose Firefox Downloads SQL query {err:?}");
            return Err(FirefoxHistoryError::BadSQL);
        }
    };

    // Get browser downloads data
    let downloads_data = stmt.query_map([], |row| {
        Ok(FirefoxDownload {
            id: row.get("downloads_id")?,
            place_id: row.get("place_id")?,
            anno_attribute_id: row.get("anno_attribute_id")?,
            content: row.get("content").unwrap_or_default(),
            flags: row.get("flags")?,
            expiration: row.get("expiration")?,
            download_type: row.get("type")?,
            date_added: {
                let value: i64 = row.get("dateAdded")?;
                unixepoch_microseconds_to_iso(&value)
            },
            last_modified: {
                let value: i64 = row.get("lastModified")?;
                unixepoch_microseconds_to_iso(&value)
            },
            name: row.get("name")?,
            history: FirefoxHistoryEntry {
                moz_places_id: row.get("moz_places_id")?,
                url: row.get("url").unwrap_or_default(),
                title: row.get("title").unwrap_or_default(),
                rev_host: row.get("rev_host")?,
                visit_count: row.get("visit_count")?,
                hidden: row.get("hidden")?,
                typed: row.get("typed")?,
                frequency: 0,
                last_visit_date: {
                    let value: i64 = row.get("last_visit_date").unwrap_or_default();
                    unixepoch_microseconds_to_iso(&value)
                },
                guid: row.get("guid")?,
                foreign_count: row.get("foreign_count").unwrap_or_default(),
                url_hash: row.get("url_hash")?,
                description: row.get("description").unwrap_or_default(),
                preview_image_url: row.get("preview_image_url").unwrap_or_default(),
                prefix: String::new(),
                host: String::new(),
            },
        })
    });

    match downloads_data {
        Ok(download_iter) => {
            let mut download_vec: Vec<FirefoxDownload> = Vec::new();
            for download in download_iter {
                match download {
                    Ok(download_data) => {
                        download_vec.push(download_data);
                    }
                    Err(err) => {
                        warn!("[firefox] Failed to iterate Firefox download data: {err:?}");
                    }
                }
            }
            Ok(download_vec)
        }
        Err(err) => {
            error!("[firefox] Failed to get Firefox download data: {err:?}");
            Err(FirefoxHistoryError::SqliteParse)
        }
    }
}

#[cfg(test)]
mod tests {
    use super::get_firefox_downloads;
    use crate::artifacts::applications::firefox::downloads::downloads_query;
    use std::path::PathBuf;

    #[test]
    fn test_get_firefox_downloads() {
        let _result = get_firefox_downloads().unwrap();
    }

    #[test]
    fn test_downloads_query() {
        let mut test_location = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
        test_location.push("tests/test_data/browser/firefox/places_downloads.sqlite");
        let test_path: &str = &test_location.display().to_string();
        let results = downloads_query(test_path).unwrap();
        assert_eq!(results.len(), 2);

        for result in results {
            if result.id == 1 {
                assert_eq!(result.id, 1);
                assert_eq!(result.place_id, 1263);
                assert_eq!(result.anno_attribute_id, 1);
                assert_eq!(
                    result.content,
                    "file:///C:/Users/bob/Downloads/ChromeSetup.exe"
                );
                assert_eq!(result.flags, 0);
                assert_eq!(result.history.url, "https://dl.google.com/tag/s/appguid%3D%7B8A69D345-D564-463C-AFF1-A69D9E530F96%7D%26iid%3D%7BA3C66172-6F8B-5CBB-E30B-10AEBD46614A%7D%26lang%3Den%26browser%3D3%26usagestats%3D0%26appname%3DGoogle%2520Chrome%26needsadmin%3Dprefers%26ap%3Dx64-stable-statsdef_1%26installdataindex%3Dempty/update2/installers/ChromeSetup.exe");
                assert_eq!(result.expiration, 4);
                assert_eq!(result.download_type, 3);
                assert_eq!(result.date_added, "2022-06-18T22:21:48.198Z");
                assert_eq!(result.last_modified, "2022-06-18T22:21:48.198Z");
                assert_eq!(result.name, "downloads/destinationFileURI");
                assert_eq!(result.history.moz_places_id, 1263);
                assert_eq!(result.history.title, "ChromeSetup.exe");
                assert_eq!(result.history.rev_host, "moc.elgoog.ld.");
                assert_eq!(result.history.visit_count, 0);
                assert_eq!(result.history.hidden, 0);
                assert_eq!(result.history.last_visit_date, "2022-06-18T22:21:48.060Z");
                assert_eq!(result.history.guid, "I4m9vpx79Vuo");
                assert_eq!(result.history.foreign_count, 0);
                assert_eq!(result.history.url_hash, 47358292339056);
                assert_eq!(result.history.description, "");
                assert_eq!(result.history.preview_image_url, "");
                assert_eq!(result.history.prefix, "");
                assert_eq!(result.history.host, "");
            } else if result.id == 2 {
                assert_eq!(result.id, 2);
                assert_eq!(result.place_id, 1263);
                assert_eq!(result.anno_attribute_id, 2);
                assert_eq!(
                    result.content,
                    "{\"state\":1,\"deleted\":false,\"endTime\":1655590908348,\"fileSize\":1414600}"
                );
                assert_eq!(result.flags, 0);
                assert_eq!(result.history.url, "https://dl.google.com/tag/s/appguid%3D%7B8A69D345-D564-463C-AFF1-A69D9E530F96%7D%26iid%3D%7BA3C66172-6F8B-5CBB-E30B-10AEBD46614A%7D%26lang%3Den%26browser%3D3%26usagestats%3D0%26appname%3DGoogle%2520Chrome%26needsadmin%3Dprefers%26ap%3Dx64-stable-statsdef_1%26installdataindex%3Dempty/update2/installers/ChromeSetup.exe");
                assert_eq!(result.expiration, 4);
                assert_eq!(result.download_type, 3);
                assert_eq!(result.date_added, "2022-06-18T22:21:48.397Z");
                assert_eq!(result.last_modified, "2022-06-18T22:21:48.397Z");
                assert_eq!(result.name, "downloads/metaData");
                assert_eq!(result.history.moz_places_id, 1263);
                assert_eq!(result.history.title, "ChromeSetup.exe");
                assert_eq!(result.history.rev_host, "moc.elgoog.ld.");
                assert_eq!(result.history.visit_count, 0);
                assert_eq!(result.history.hidden, 0);
                assert_eq!(result.history.last_visit_date, "2022-06-18T22:21:48.060Z");
                assert_eq!(result.history.guid, "I4m9vpx79Vuo");
                assert_eq!(result.history.foreign_count, 0);
                assert_eq!(result.history.url_hash, 47358292339056);
                assert_eq!(result.history.description, "");
                assert_eq!(result.history.preview_image_url, "");
                assert_eq!(result.history.prefix, "");
                assert_eq!(result.history.host, "");
            }
        }
    }
}
