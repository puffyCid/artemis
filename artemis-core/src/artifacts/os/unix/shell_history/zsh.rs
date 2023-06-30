use crate::filesystem::files::file_lines;
use crate::filesystem::{
    directory::is_directory,
    files::{file_extension, is_file, list_files},
};
use crate::utils::regex_options::create_regex;
use crate::{
    artifacts::os::unix::shell_history::error::ShellError,
    filesystem::{
        directory::{get_root_home, get_user_paths},
        files::get_filename,
    },
};
use log::{error, info, warn};
use regex::Regex;
use serde::Serialize;

#[derive(Debug, Serialize)]
pub(crate) struct ZshHistory {
    pub(crate) history: Vec<ZshHistoryData>,
    pub(crate) path: String,
    pub(crate) user: String,
}

#[derive(Debug, Serialize)]
pub(crate) struct ZshHistoryData {
    pub(crate) history: String,
    pub(crate) timestamp: u64,
    pub(crate) line: usize,
    pub(crate) duration: u64,
}

impl ZshHistory {
    /// Get zsh history and session data for all users
    pub(crate) fn get_user_zsh_history() -> Result<Vec<ZshHistory>, ShellError> {
        let user_path_result = get_user_paths();
        let mut user_paths = match user_path_result {
            Ok(result) => result,
            Err(err) => {
                error!("[shell_history] Could not get user paths for zsh history: {err:?}");
                return Err(ShellError::UserPaths);
            }
        };
        let root_path_result = get_root_home();
        match root_path_result {
            Ok(result) => user_paths.push(result),
            Err(err) => {
                error!("[shell_history] Could not get root home for zsh history: {err:?}");
            }
        }
        ZshHistory::zsh(&user_paths)
    }

    /// Get zsh and any zsh session data
    fn zsh(user_paths: &[String]) -> Result<Vec<ZshHistory>, ShellError> {
        let mut shell_history: Vec<ZshHistory> = Vec::new();
        for path in user_paths {
            let zsh_history = format!("{path}/.zsh_history");
            let zsh_sessions = format!("{path}/.zsh_sessions");
            if is_file(&zsh_history) {
                let zsh_data_result = ZshHistory::parse_zsh(&zsh_history);
                let zsh_data = match zsh_data_result {
                    Ok(result) => result,
                    Err(err) => {
                        warn!("[shell_history] Could not parse zsh file at {path}: {err:?}");
                        continue;
                    }
                };
                let zsh_history = ZshHistory {
                    history: zsh_data,
                    path: zsh_history,
                    user: get_filename(path),
                };
                shell_history.push(zsh_history);
            }
            if is_directory(&zsh_sessions) {
                let zsh_data_result = ZshHistory::parse_zsh_sessions(path, &get_filename(path));
                let mut zsh_data = match zsh_data_result {
                    Ok(result) => result,
                    Err(err) => {
                        warn!(
                            "[shell_history] Could not parse zsh session file at {path}: {err:?}"
                        );
                        continue;
                    }
                };
                shell_history.append(&mut zsh_data);
            }
        }
        Ok(shell_history)
    }

    // Parse each history session file for a user
    fn parse_zsh_sessions(
        zsh_session_path: &str,
        username: &str,
    ) -> Result<Vec<ZshHistory>, ShellError> {
        let mut shell_sessions: Vec<ZshHistory> = Vec::new();
        let session_result = list_files(zsh_session_path);
        let sessions = match session_result {
            Ok(result) => result,
            Err(err) => {
                warn!(
                    "[shell_history] Could not list zsh session files at {zsh_session_path}: {err:?}"
                );
                return Err(ShellError::SessionPath);
            }
        };

        for session in sessions {
            let extension = file_extension(&session);

            if extension == "history" && is_file(&session) {
                let zsh_data = ZshHistory::parse_zsh(&session)?;
                let zsh_history = ZshHistory {
                    history: zsh_data,
                    path: session,
                    user: username.to_string(),
                };
                shell_sessions.push(zsh_history);
            }
        }
        Ok(shell_sessions)
    }

    // Parse the zsh_history file
    fn parse_zsh(zsh_history: &str) -> Result<Vec<ZshHistoryData>, ShellError> {
        let mut zsh_data: Vec<ZshHistoryData> = Vec::new();
        let file_result = file_lines(zsh_history);
        let zsh_iter = match file_result {
            Ok(result) => result,
            Err(err) => {
                error!("[shell_history] Could not read bash_history lines: {err:?}");
                return Err(ShellError::File);
            }
        };

        // Regex if zsh_history timestamp is enabled. Ex: ": 1659414442:0;cargo test --release"
        let zsh_regex = create_regex(r"^: {0,10}([0-9]{1,11}):[0-9]+;(.*)$").unwrap();
        let mut line_number = 1;

        // Read each line and parse the associated data. Potentially: timestamp, duration, command
        for line_entry in zsh_iter {
            let zsh_entry = match line_entry {
                Ok(result) => result,
                Err(err) => {
                    warn!(
                        "[shell_history] Failed to read zsh line in file {zsh_history}, error: {err:?}"
                    );
                    continue;
                }
            };

            let zsh_history_results = ZshHistory::parse_line(&zsh_entry, &zsh_regex);
            match zsh_history_results {
                Ok(mut zsh_history_data) => {
                    zsh_history_data.line = line_number;
                    zsh_data.push(zsh_history_data);
                }
                Err(err) => warn!("[shell_history] Failed to parse zsh line entry: {err:?}"),
            }
            line_number += 1;
        }

        Ok(zsh_data)
    }

    // Parse each line of the zsh_history file
    fn parse_line<'a>(
        zsh_line: &'a str,
        zsh_regex: &'a Regex,
    ) -> Result<ZshHistoryData, ShellError> {
        let mut zsh_history_data = ZshHistoryData {
            history: String::new(),
            timestamp: 0,
            line: 0,
            duration: 0,
        };
        if zsh_regex.is_match(zsh_line) {
            let value_size = 3;
            for value in zsh_regex.captures_iter(zsh_line) {
                if value.len() < value_size {
                    continue;
                }
                let zsh_timestamp_result = value[1].parse::<u64>();
                match zsh_timestamp_result {
                    Ok(zsh_timestamp) => zsh_history_data.timestamp = zsh_timestamp,
                    Err(err) => info!("[shell_history] Failed to parse zsh timestamp: {err:?}"),
                }

                let zsh_duration_result = value[2].parse::<u64>();
                match zsh_duration_result {
                    Ok(zsh_duration) => zsh_history_data.duration = zsh_duration,
                    Err(err) => info!("[shell_history] Failed to parse zsh duration: {err:?}"),
                }

                zsh_history_data.history = value[2].to_string();
            }
        } else {
            zsh_history_data.history = zsh_line.to_string();
        }
        Ok(zsh_history_data)
    }
}

#[cfg(test)]
mod tests {
    use crate::filesystem::directory::get_user_paths;

    use super::ZshHistory;
    use regex::Regex;
    use std::path::PathBuf;

    #[test]
    fn test_get_macos_user_zsh_history() {
        let _ = ZshHistory::get_user_zsh_history().unwrap();
    }

    #[test]
    fn test_get_zsh_history() {
        let start_path = get_user_paths().unwrap();

        let _ = ZshHistory::zsh(&start_path).unwrap();
    }

    #[test]
    fn test_parse_zsh_sessions() {
        let mut test_location = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
        test_location.push("tests/test_data/unix/zsh");

        let results =
            ZshHistory::parse_zsh_sessions(&test_location.display().to_string(), "test").unwrap();
        assert_eq!(results.len(), 1);

        assert_eq!(results[0].user, "test");
        assert_eq!(results[0].history.len(), 5);
        assert_eq!(results[0].path.ends_with("zsh_sesssion.history"), true);

        assert_eq!(results[0].history[0].history, "./osquery/osqueryi");
        assert_eq!(results[0].history[0].line, 1);
        assert_eq!(results[0].history[0].timestamp, 0);
        assert_eq!(results[0].history[0].duration, 0);
    }

    #[test]
    fn test_parse_zsh() {
        let mut test_location = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
        test_location.push("tests/test_data/unix/zsh/zsh_history");

        let results = ZshHistory::parse_zsh(&test_location.display().to_string()).unwrap();
        assert_eq!(results.len(), 5);

        assert_eq!(results[0].history, "pwd");
        assert_eq!(results[0].line, 1);
        assert_eq!(results[0].timestamp, 1659414442);
        assert_eq!(results[0].duration, 0);

        assert_eq!(results[1].history, "cd ~/Projects/Rust/macos-bookmarks");
        assert_eq!(results[1].line, 2);
        assert_eq!(results[1].timestamp, 1659414442);
        assert_eq!(results[1].duration, 0);

        assert_eq!(results[2].history, "cargo test --release");
        assert_eq!(results[2].line, 3);
        assert_eq!(results[2].timestamp, 1659414442);
        assert_eq!(results[2].duration, 0);
    }

    #[test]
    fn test_parse_line() {
        let test_line = ": 1659414442:0;cd ~/Projects/Rust/macos-bookmarks";
        let test_regex = Regex::new(r"^: {0,10}([0-9]{1,11}):[0-9]+;(.*)$").unwrap();
        let results = ZshHistory::parse_line(test_line, &test_regex).unwrap();

        assert_eq!(results.history, "cd ~/Projects/Rust/macos-bookmarks");
        assert_eq!(results.line, 0);
        assert_eq!(results.timestamp, 1659414442);
        assert_eq!(results.duration, 0);
    }

    #[test]
    fn test_bad_timestamp_parse_line() {
        let test_line = ": 1a659414442:0;cd ~/Projects/Rust/macos-bookmarks";
        let test_regex = Regex::new(r"^: {0,10}([0-9]{1,11}):[0-9]+;(.*)$").unwrap();
        let results = ZshHistory::parse_line(test_line, &test_regex).unwrap();

        assert_eq!(
            results.history,
            ": 1a659414442:0;cd ~/Projects/Rust/macos-bookmarks"
        );
        assert_eq!(results.line, 0);
        assert_eq!(results.timestamp, 0);
        assert_eq!(results.duration, 0);
    }
}
