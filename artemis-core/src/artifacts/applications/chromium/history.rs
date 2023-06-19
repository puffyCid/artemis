use super::error::ChromiumHistoryError;
use crate::{filesystem::directory::get_user_paths, utils::time::webkit_time_to_uniexepoch};
use log::error;
use rusqlite::{Connection, OpenFlags};
use serde::Serialize;
use std::path::Path;

#[derive(Serialize)]
pub(crate) struct ChromiumHistory {
    pub(crate) history: Vec<History>,
    pub(crate) path: String,
    pub(crate) user: String,
}

#[derive(Serialize)]
pub(crate) struct History {
    pub(crate) id: i64,
    pub(crate) url: String,   // Can be null
    pub(crate) title: String, // Can be null
    pub(crate) visit_count: i64,
    pub(crate) typed_count: i64,
    pub(crate) last_visit_time: i64,
    pub(crate) hidden: i64,
    pub(crate) visits_id: i64,
    pub(crate) from_visit: i64, // Can be null
    pub(crate) transition: i64,
    pub(crate) segment_id: i64, // Can be null
    pub(crate) visit_duration: i64,
    pub(crate) opener_visit: i64, // Can be null
}

impl ChromiumHistory {
    /// Parse and get the Chromium history data
    pub(crate) fn get_history() -> Result<Vec<ChromiumHistory>, ChromiumHistoryError> {
        let user_paths_result = get_user_paths();
        let user_paths = match user_paths_result {
            Ok(result) => result,
            Err(err) => {
                error!("[chromium] Failed to get user paths: {err:?}");
                return Err(ChromiumHistoryError::PathError);
            }
        };
        let mut chromium_history: Vec<ChromiumHistory> = Vec::new();

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
            let history = ChromiumHistory::history_query(&path)?;

            #[cfg(target_os = "macos")]
            let user = users.replace("/Users/", "");

            #[cfg(target_os = "windows")]
            {
                let user_data: Vec<&str> = users.split("\\").collect();
                let user = user_data.last().unwrap_or(&"");
            }
            #[cfg(target_os = "linux")]
            {
                let user_data: Vec<&str> = users.split("/").collect();
                let user = user_data.last().unwrap_or(&"");
            }

            let history_data = ChromiumHistory {
                history,
                path,
                user,
            };

            chromium_history.push(history_data);
        }
        Ok(chromium_history)
    }

    /// Query the URL history tables
    pub(crate) fn history_query(path: &str) -> Result<Vec<History>, ChromiumHistoryError> {
        // Bypass SQLITE file lock
        let history_file = format!("file:{path}?immutable=1");
        let connection = Connection::open_with_flags(
            history_file,
            OpenFlags::SQLITE_OPEN_READ_ONLY | OpenFlags::SQLITE_OPEN_URI,
        );
        let conn = match connection {
            Ok(connect) => connect,
            Err(err) => {
                error!("[chromium] Failed to read Chromium SQLITE history file {err:?}");
                return Err(ChromiumHistoryError::SQLITEParseError);
            }
        };

        let  statement = conn.prepare("SELECT urls.id as urls_id, urls.url as urls_url, title, visit_count, typed_count, last_visit_time, hidden, visits.id as visits_id, from_visit, 
        transition, segment_id, visit_duration, opener_visit FROM urls join visits on urls.id = visits.url");
        let mut stmt = match statement {
            Ok(query) => query,
            Err(err) => {
                error!("[chromium] Failed to compose Chromium Histoy SQL query {err:?}");
                return Err(ChromiumHistoryError::BadSQL);
            }
        };

        // Get browser history data
        let history_data = stmt.query_map([], |row| {
            Ok(History {
                id: row.get("urls_id")?,
                url: row.get("urls_url").unwrap_or_default(),
                title: row.get("title").unwrap_or_default(),
                visit_count: row.get("visit_count")?,
                typed_count: row.get("typed_count")?,
                last_visit_time: row.get("last_visit_time")?,
                hidden: row.get("hidden")?,
                visits_id: row.get("visits_id")?,
                from_visit: row.get("from_visit").unwrap_or_default(),
                transition: row.get("transition")?,
                segment_id: row.get("segment_id").unwrap_or_default(),
                visit_duration: row.get("visit_duration")?,
                opener_visit: row.get("opener_visit").unwrap_or_default(),
            })
        });

        match history_data {
            Ok(history_iter) => {
                let mut history_vec: Vec<History> = Vec::new();

                for history in history_iter {
                    match history {
                        Ok(mut history_data) => {
                            let adjust_time = 1000000;
                            history_data.last_visit_time = webkit_time_to_uniexepoch(
                                &(history_data.last_visit_time / adjust_time),
                            );
                            history_vec.push(history_data);
                        }
                        Err(err) => {
                            error!("[chromium] Failed to iterate Chromium history data: {err:?}");
                        }
                    }
                }

                Ok(history_vec)
            }
            Err(err) => {
                error!("[chromium] Failed to get Chromium history data: {err:?}");
                Err(ChromiumHistoryError::SQLITEParseError)
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::ChromiumHistory;
    use std::path::PathBuf;

    #[test]
    fn test_get_users_history() {
        let _result = ChromiumHistory::get_history().unwrap();
    }

    #[test]
    fn test_history_query() {
        let mut test_location = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
        test_location.push("tests/test_data/browser/chromium/History");
        let test_path: &str = &test_location.display().to_string();
        let history = ChromiumHistory::history_query(test_path).unwrap();

        let mut correct_url = false;
        let mut correct_title = false;
        let mut correct_time = false;
        let mut correct_count = false;
        let mut correct_duration = false;
        for history_data in history {
            if history_data.url == "https://www.google.com/search?q=is+c+safer+than+rust&oq=is+c+safer+than+rust&aqs=chrome..69i57j0i22i30j0i390l2.4300j0j7&client=ubuntu&sourceid=chrome&ie=UTF-8" {
                correct_url = true;
            }
            if history_data.title == "Install PowerShell on Linux - PowerShell | Microsoft Docs" {
                correct_title = true;
            }
            if history_data.last_visit_time == 1645510339 {
                correct_time = true;
            }
            if history_data.visit_count == 3 {
                correct_count = true;
            }
            if history_data.visit_duration == 1974937 {
                correct_duration = true;
            }
        }

        assert_eq!(correct_url, true);
        assert_eq!(correct_title, true);
        assert_eq!(correct_time, true);
        assert_eq!(correct_count, true);
        assert_eq!(correct_duration, true);
    }
}
