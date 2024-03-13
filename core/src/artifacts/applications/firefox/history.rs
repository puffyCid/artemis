/**
 *  Parse Firefox Places SQLITE file
 *  Provides functions to parse Firefox History data.
 * */
use super::error::FirefoxHistoryError;
use crate::filesystem::directory::get_user_paths;
use common::applications::{FirefoxHistory, FirefoxHistoryEntry};
use log::{error, warn};
use rusqlite::{Connection, OpenFlags};
use std::{
    fs::read_dir,
    path::{Path, PathBuf},
};

/// Get `Firefox` History for users
pub(crate) fn get_firefox_history() -> Result<Vec<FirefoxHistory>, FirefoxHistoryError> {
    let user_paths_result = get_user_paths();
    let user_paths = match user_paths_result {
        Ok(result) => result,
        Err(err) => {
            error!("[firefox] Failed to get user paths: {err:?}");
            return Err(FirefoxHistoryError::PathError);
        }
    };
    let mut firefox_history: Vec<FirefoxHistory> = Vec::new();

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
            let history = history_query(&path)?;

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

            let history_data = FirefoxHistory {
                history,
                path,
                user,
            };

            firefox_history.push(history_data);
        }
    }
    Ok(firefox_history)
}

/// Get the Firefox `places.sqlite` for the user
pub(crate) fn user_data(firefox_path: &PathBuf) -> Vec<String> {
    let target_file = "places.sqlite";

    let mut firefox_data: Vec<String> = Vec::new();
    let profiles_results = read_dir(firefox_path);
    let profiles = match profiles_results {
        Ok(results) => results,
        Err(_err) => return firefox_data,
    };

    for entry_result in profiles {
        let entry = match entry_result {
            Ok(result) => result,
            Err(_err) => continue,
        };
        let full_path = entry.path();
        if !full_path.is_dir() || !full_path.display().to_string().ends_with("default-release") {
            continue;
        }
        #[cfg(target_os = "windows")]
        let path = format!("{}\\{target_file}", full_path.display());
        #[cfg(target_family = "unix")]
        let path = format!("{}/{target_file}", full_path.display());
        firefox_data.push(path);
    }
    firefox_data
}

/// Query the URL history tables
pub(crate) fn history_query(path: &str) -> Result<Vec<FirefoxHistoryEntry>, FirefoxHistoryError> {
    // Bypass SQLITE file lock
    let history_file = format!("file:{path}?immutable=1");
    let connection = Connection::open_with_flags(
        history_file,
        OpenFlags::SQLITE_OPEN_READ_ONLY | OpenFlags::SQLITE_OPEN_URI,
    );
    let conn = match connection {
        Ok(connect) => connect,
        Err(err) => {
            error!("[firefox] Failed to read Firefox SQLITE history file {err:?}");
            return Err(FirefoxHistoryError::SqliteParse);
        }
    };

    let  statement = conn.prepare("SELECT moz_places.id as moz_places_id, url, title, rev_host, visit_count, hidden, typed, last_visit_date, guid, foreign_count, url_hash, description, preview_image_url, origin_id, prefix, host, moz_origins.frecency as frequency FROM moz_places join moz_origins on moz_places.origin_id = moz_origins.id");
    let mut stmt = match statement {
        Ok(query) => query,
        Err(err) => {
            error!("[firefox] Failed to compose Firefox History SQL query {err:?}");
            return Err(FirefoxHistoryError::BadSQL);
        }
    };

    // Get browser history data
    let history_data = stmt.query_map([], |row| {
        Ok(FirefoxHistoryEntry {
            moz_places_id: row.get("moz_places_id")?,
            url: row.get("url").unwrap_or_default(),
            title: row.get("title").unwrap_or_default(),
            rev_host: row.get("rev_host")?,
            visit_count: row.get("visit_count")?,
            hidden: row.get("hidden")?,
            typed: row.get("typed")?,
            frequency: row.get("frequency")?,
            last_visit_date: row.get("last_visit_date").unwrap_or_default(),
            guid: row.get("guid")?,
            foreign_count: row.get("foreign_count").unwrap_or_default(),
            url_hash: row.get("url_hash")?,
            description: row.get("description").unwrap_or_default(),
            preview_image_url: row.get("preview_image_url").unwrap_or_default(),
            prefix: row.get("prefix")?,
            host: row.get("host")?,
        })
    });

    match history_data {
        Ok(history_iter) => {
            let mut history_vec: Vec<FirefoxHistoryEntry> = Vec::new();
            // Grab all Firefox history entries
            for history in history_iter {
                match history {
                    Ok(mut history_data) => {
                        let adjust_time = 1000000;
                        history_data.last_visit_date /= adjust_time;

                        history_vec.push(history_data);
                    }
                    Err(err) => {
                        warn!("[firefox] Failed to iterate through Firefox history data: {err:?}");
                    }
                }
            }
            Ok(history_vec)
        }
        Err(err) => {
            error!("[firefox]  Failed to get Firefox history data: {err:?}");
            Err(FirefoxHistoryError::SqliteParse)
        }
    }
}

#[cfg(test)]
mod tests {
    use super::get_firefox_history;
    use crate::artifacts::applications::firefox::history::history_query;
    use std::path::PathBuf;

    #[test]
    fn test_get_firefox_history() {
        let _result = get_firefox_history().unwrap();
    }

    #[test]
    fn test_history_query() {
        let mut test_location = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
        test_location.push("tests/test_data/browser/firefox/places.sqlite");
        let test_path: &str = &test_location.display().to_string();
        let history = history_query(test_path).unwrap();

        let mut correct_url = false;
        for history_data in history {
            if history_data.url == "https://github.com/libyal/dtformats/blob/main/documentation/Jump%20lists%20format.asciidoc" {
                correct_url = true;
                assert_eq!(history_data.moz_places_id, 1114);
                assert_eq!(history_data.title, "dtformats/Jump lists format.asciidoc at main · libyal/dtformats · GitHub");
                assert_eq!(history_data.rev_host, "moc.buhtig.");
                assert_eq!(history_data.visit_count, 0);
                assert_eq!(history_data.hidden, 0);
                assert_eq!(history_data.typed, 0);
                assert_eq!(history_data.frequency, 15517);
                assert_eq!(history_data.last_visit_date, 0);
                assert_eq!(history_data.guid, "3WEJ95Stho90");
                assert_eq!(history_data.foreign_count, 1);
                assert_eq!(history_data.url_hash, 47359319030241);
                assert_eq!(history_data.description, "Collection of data formats. Contribute to libyal/dtformats development by creating an account on GitHub.");
                assert_eq!(history_data.preview_image_url, "https://opengraph.githubassets.com/6bd05c89d87c25c78a6ac7d8355f46d2f967af8150abeacdf681070e4eccadcc/libyal/dtformats");
                assert_eq!(history_data.prefix, "https://");
                assert_eq!(history_data.host, "github.com");
            }
        }

        assert!(correct_url);
    }
}
