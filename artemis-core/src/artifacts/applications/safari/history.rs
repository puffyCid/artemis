use super::error::SafariError;
use crate::{
    filesystem::{directory::get_user_paths, files::is_file},
    utils::time::cocoatime_to_unixepoch,
};
use log::{error, warn};
use rusqlite::{Connection, OpenFlags};
use serde::Serialize;

#[derive(Debug, Serialize)]
pub(crate) struct SafariHistory {
    pub(crate) results: Vec<History>,
    pub(crate) path: String,
    pub(crate) user: String,
}

#[derive(Debug, Serialize)]
pub(crate) struct History {
    pub(crate) id: i64,
    pub(crate) url: String,
    pub(crate) domain_expansion: String, // Can be null
    pub(crate) visit_count: i64,
    pub(crate) daily_visit_counts: Vec<u8>,    // Can be null
    pub(crate) weekly_visit_counts: Vec<u8>,   // Can be null
    pub(crate) autocomplete_triggers: Vec<u8>, // Can be null
    pub(crate) should_recompute_derived_visit_counts: i64,
    pub(crate) visit_count_score: i64,
    pub(crate) status_code: i64,
    pub(crate) visit_time: i64,
    pub(crate) load_successful: bool,
    pub(crate) title: String, // Can be null
    pub(crate) attributes: f64,
    pub(crate) score: f64,
}

impl SafariHistory {
    /// Get Safari SQLITE History file for all users to get browser history
    pub(crate) fn get_history() -> Result<Vec<SafariHistory>, SafariError> {
        // Get all user directories
        let user_paths_result = get_user_paths();
        let user_paths = match user_paths_result {
            Ok(result) => result,
            Err(err) => {
                error!("[safari] Failed to get user paths: {err:?}");
                return Err(SafariError::PathError);
            }
        };

        let history_path = "/Library/Safari/History.db";
        let mut safari_history: Vec<SafariHistory> = Vec::new();

        for users in user_paths {
            let path = format!("{users}{history_path}");
            if !is_file(&path) {
                continue;
            }
            let results = SafariHistory::history_query(&path)?;

            let username = users.replace("/Users/", "");
            let history = SafariHistory {
                results,
                path,
                user: username,
            };

            safari_history.push(history);
        }
        Ok(safari_history)
    }

    /// Query the URL history tables based on provided path
    pub(crate) fn history_query(path: &str) -> Result<Vec<History>, SafariError> {
        // Bypass SQLITE file lock
        let history_file = format!("file:{path}?immutable=1");
        let connection = Connection::open_with_flags(
            history_file,
            OpenFlags::SQLITE_OPEN_READ_ONLY | OpenFlags::SQLITE_OPEN_URI,
        );
        let conn = match connection {
            Ok(connect) => connect,
            Err(err) => {
                error!("Failed to read Safari SQLITE history file {err:?}");
                return Err(SafariError::SqliteParse);
            }
        };

        let  statement = conn.prepare("SELECT history_items.id as history_item_id, url, domain_expansion, visit_count, daily_visit_counts,weekly_visit_counts,autocomplete_triggers,should_recompute_derived_visit_counts,visit_count_score,status_code,cast(visit_time as INT) as visit_time,title,load_successful,http_non_get,synthesized,redirect_destination,origin,generation,attributes,score FROM history_items JOIN history_visits ON history_visits.history_item = history_items.id");
        let mut stmt = match statement {
            Ok(query) => query,
            Err(err) => {
                error!("Failed to compose Safari History SQL query {err:?}");
                return Err(SafariError::BadSQL);
            }
        };

        // Get browser history data
        let history_data = stmt.query_map([], |row| {
            Ok(History {
                id: row.get("history_item_id")?,
                url: row.get("url")?,
                title: row.get("title").unwrap_or_default(),
                visit_count: row.get("visit_count")?,
                domain_expansion: row.get("domain_expansion").unwrap_or_default(),
                daily_visit_counts: row.get("daily_visit_counts").unwrap_or_default(),
                weekly_visit_counts: row.get("weekly_visit_counts").unwrap_or_default(),
                autocomplete_triggers: row.get("autocomplete_triggers").unwrap_or_default(),
                should_recompute_derived_visit_counts: row
                    .get("should_recompute_derived_visit_counts")?,
                visit_count_score: row.get("visit_count_score")?,
                status_code: row.get("status_code")?,
                visit_time: row.get("visit_time")?,
                load_successful: row.get("load_successful")?,
                attributes: row.get("attributes")?,
                score: row.get("score")?,
            })
        });

        match history_data {
            Ok(history_iter) => {
                let mut history_vec: Vec<History> = Vec::new();

                for history in history_iter {
                    match history {
                        Ok(mut history_data) => {
                            history_data.visit_time =
                                cocoatime_to_unixepoch(&(history_data.visit_time as f64));
                            history_vec.push(history_data);
                        }
                        Err(err) => {
                            warn!("Failed to iterate through Safari history data: {err:?}");
                        }
                    }
                }

                Ok(history_vec)
            }
            Err(err) => {
                error!("Failed to get Safari history data from SQLITE file: {err:?}");
                Err(SafariError::SqliteParse)
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::SafariHistory;
    use std::path::PathBuf;

    #[test]
    #[ignore = "Get live users Safari history"]
    fn test_get_history() {
        let result = SafariHistory::get_history().unwrap();
        assert!(result.len() > 0);
    }

    #[test]
    fn test_history_query() {
        let mut test_location = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
        test_location.push("tests/test_data/browser/safari/History.db");
        let history = SafariHistory::history_query(&test_location.display().to_string()).unwrap();

        assert_eq!(history.len(), 42);
        assert_eq!(history[0].id, 167);
        assert_eq!(
            history[0].url,
            "https://www.google.com/search?client=safari&rls=en&q=duckduckgo&ie=UTF-8&oe=UTF-8"
        );
        assert_eq!(history[0].domain_expansion, "google");
        assert_eq!(history[0].visit_count, 2);
        assert_eq!(history[0].daily_visit_counts, [100, 0, 0, 0]);
        assert_eq!(history[0].weekly_visit_counts.is_empty(), true);
        assert_eq!(history[0].autocomplete_triggers.is_empty(), true);
        assert_eq!(history[0].should_recompute_derived_visit_counts, 0);
        assert_eq!(history[0].visit_count_score, 100);
        assert_eq!(history[0].status_code, 0);
        assert_eq!(history[0].visit_time, 1655693243);
        assert_eq!(history[0].load_successful, true);
        assert_eq!(history[0].title, "duckduckgo - Google Search");
        assert_eq!(history[0].attributes, 0.0);
        assert_eq!(history[0].score, 100.0);

        assert_eq!(history[9].id, 173);
        assert_eq!(
            history[9].url,
            "https://docs.microsoft.com/en-us/powershell/scripting/overview"
        );
        assert_eq!(history[9].domain_expansion, "docs.microsoft");
        assert_eq!(history[9].visit_count, 1);
        assert_eq!(history[9].daily_visit_counts, [100, 0, 0, 0]);
        assert_eq!(history[9].weekly_visit_counts.is_empty(), true);
        assert_eq!(history[9].autocomplete_triggers.is_empty(), true);
        assert_eq!(history[9].should_recompute_derived_visit_counts, 0);
        assert_eq!(history[9].visit_count_score, 100);
        assert_eq!(history[9].status_code, 0);
        assert_eq!(history[9].visit_time, 1655695244);
        assert_eq!(history[9].load_successful, true);
        assert_eq!(history[9].title, "");
        assert_eq!(history[9].attributes, 0.0);
        assert_eq!(history[9].score, 100.0);
    }
}
