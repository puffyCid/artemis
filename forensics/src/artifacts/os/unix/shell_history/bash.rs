use crate::artifacts::os::unix::shell_history::error::ShellError;
use crate::filesystem::directory::{get_root_home, get_user_paths};
use crate::filesystem::files::{file_lines, get_filename, list_files};
use crate::filesystem::{
    directory::is_directory,
    files::{file_extension, is_file},
};
use crate::utils::regex_options::create_regex;
use crate::utils::time::unixepoch_to_iso;
use common::unix::{BashHistory, BashHistoryData};
use log::{error, warn};
use regex::Regex;

/// Get all bash history and session data for all users
pub(crate) fn get_user_bash_history() -> Result<Vec<BashHistory>, ShellError> {
    let user_path_result = get_user_paths();
    let mut user_paths = match user_path_result {
        Ok(result) => result,
        Err(err) => {
            error!("[shell_history] Could not get user paths for bash history: {err:?}");
            return Err(ShellError::UserPaths);
        }
    };
    let root_path_result = get_root_home();
    match root_path_result {
        Ok(result) => user_paths.push(result),
        Err(err) => {
            error!("[shell_history] Could not get root home for bash history: {err:?}");
        }
    }
    bash(&user_paths)
}

/// Get `bash_history` and any bash session data for all users
fn bash(user_paths: &[String]) -> Result<Vec<BashHistory>, ShellError> {
    let mut shell_history: Vec<BashHistory> = Vec::new();

    for path in user_paths {
        let bash_history = format!("{path}/.bash_history");
        let bash_sessions = format!("{path}/.bash_sessions");
        if is_file(&bash_history) {
            let bash_data = parse_bash(&bash_history)?;
            let bash_history = BashHistory {
                history: bash_data,
                path: bash_history,
                user: get_filename(path),
            };
            shell_history.push(bash_history);
        }
        if is_directory(&bash_sessions) {
            let mut bash_data = parse_bash_sessions(path, &get_filename(path))?;
            shell_history.append(&mut bash_data);
        }
    }
    Ok(shell_history)
}

/// Parse each history session file for a user
fn parse_bash_sessions(
    bash_session_path: &str,
    username: &str,
) -> Result<Vec<BashHistory>, ShellError> {
    let mut shell_sessions: Vec<BashHistory> = Vec::new();
    let session_result = list_files(bash_session_path);
    let sessions = match session_result {
        Ok(result) => result,
        Err(err) => {
            warn!(
                "[shell_history] Could not list bash session files at {bash_session_path}: {err:?}"
            );
            return Err(ShellError::SessionPath);
        }
    };

    for session in sessions {
        let extension = file_extension(&session);

        if extension == "history" && is_file(&session) {
            let bash_data_result = parse_bash(&session);
            let bash_data = match bash_data_result {
                Ok(result) => result,
                Err(err) => {
                    error!("[shell_history] Could not parse bash session file {session}: {err:?}",);
                    continue;
                }
            };
            let bash_history = BashHistory {
                history: bash_data,
                path: session,
                user: username.to_string(),
            };
            shell_sessions.push(bash_history);
        }
    }
    Ok(shell_sessions)
}

/// Parse the `bash_history` file
fn parse_bash(bash_history: &str) -> Result<Vec<BashHistoryData>, ShellError> {
    let mut bash_data: Vec<BashHistoryData> = Vec::new();
    let file_result = file_lines(bash_history);
    let mut bash_iter = match file_result {
        Ok(result) => result,
        Err(err) => {
            error!("[shell_history] Could not read bash_history lines: {err:?}");
            return Err(ShellError::File);
        }
    };

    let mut line_number = 1;
    // Regex if bash_history timestamp is enabled. Ex: "#1659581179"
    let bash_regex = create_regex(r"^#([0-9]+)$").unwrap();

    // Iterate through bash history looking for any timestamps and associated history entry
    while let Some(line_entry) = bash_iter.next() {
        let bash_entry = match line_entry {
            Ok(result) => result,
            Err(err) => {
                warn!(
                    "[shell_history] Failed to read bash line in file {bash_history}, error: {err:?}",
                );
                continue;
            }
        };
        let mut bash_history_data = BashHistoryData {
            history: String::new(),
            timestamp: String::new(),
            line: 0,
        };

        if bash_regex.is_match(&bash_entry) {
            // Parse and the the timestamp entry
            let timestamp = parse_line(&bash_entry, &bash_regex);
            bash_history_data.timestamp = match timestamp {
                Ok(bash_timestamp) => unixepoch_to_iso(bash_timestamp as i64),
                Err(err) => {
                    warn!(
                        "[shell_history] Failed to get timestamp data for bash line {bash_entry}, error: {err:?}"
                    );
                    bash_history_data.history = bash_entry;
                    bash_data.push(bash_history_data);

                    continue;
                }
            };

            // Grab next entry associated with timestamp
            let history_value = bash_iter.next();
            let history_entry = match history_value {
                Some(result) => match result {
                    Ok(history) => history,
                    Err(err) => {
                        error!(
                            "[shell_history] No history entry in bash line: {bash_entry}, error: {err:?}"
                        );
                        String::new()
                    }
                },
                _ => String::new(),
            };

            bash_history_data.history = history_entry;
            bash_history_data.line = line_number;

            bash_data.push(bash_history_data);
            line_number += 1;
            continue;
        }

        // Grab entry, no timestamp associated with it
        bash_history_data.line = line_number;
        bash_history_data.history = bash_entry;
        bash_data.push(bash_history_data);

        line_number += 1;
    }
    Ok(bash_data)
}

/// Parse each line of the `bash_history` file
fn parse_line<'a>(bash_line: &'a str, bash_regex: &'a Regex) -> Result<u64, ShellError> {
    if let Some(value) = bash_regex.captures_iter(bash_line).next() {
        let value_empty = 0;
        if value.len() == value_empty {
            return Err(ShellError::Regex);
        }
        let bash_timestamp_result = value[1].parse::<u64>();
        return match bash_timestamp_result {
            Ok(bash_timestamp) => Ok(bash_timestamp),
            Err(err) => {
                warn!("[shell_history] Failed to parse bash timestamp: {err:?}");
                Err(ShellError::Timestamp)
            }
        };
    }
    warn!("[shell_history] Failed to get timestamp for bash entry {bash_line}");
    Err(ShellError::Regex)
}

#[cfg(test)]
mod tests {
    use super::{bash, get_user_bash_history};
    use crate::{
        artifacts::os::unix::shell_history::bash::{parse_bash, parse_bash_sessions, parse_line},
        filesystem::directory::get_user_paths,
    };
    use regex::Regex;
    use std::path::PathBuf;

    #[test]
    fn test_get_user_bash_history() {
        let _ = get_user_bash_history().unwrap();
    }

    #[test]
    fn test_get_bash_history() {
        let start_path = get_user_paths().unwrap();

        bash(&start_path).unwrap();
    }

    #[test]
    fn test_parse_bash_sessions() {
        let mut test_location = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
        test_location.push("tests/test_data/unix/bash");

        let results = parse_bash_sessions(&test_location.display().to_string(), "test").unwrap();
        assert_eq!(results.len(), 1);

        assert_eq!(results[0].user, "test");
        assert_eq!(results[0].history.len(), 64);
        assert_eq!(results[0].path.ends_with("bash_session.history"), true);

        assert_eq!(
            results[0].history[0].history,
            "sudo cp /.fseventsd ~/Desktop/"
        );
        assert_eq!(results[0].history[0].line, 1);
        assert_eq!(results[0].history[0].timestamp, "");
    }

    #[test]
    fn test_parse_bash() {
        let mut test_location = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
        test_location.push("tests/test_data/unix/bash/bash_history");

        let results = parse_bash(&test_location.display().to_string()).unwrap();
        assert_eq!(results.len(), 64);

        assert_eq!(results[59].history, "exit");
        assert_eq!(results[59].line, 60);
        assert_eq!(results[59].timestamp, "2022-08-04T02:46:19.000Z");

        assert_eq!(results[1].history, "sudo cp -r /.fseventsd ~/Desktop/");
        assert_eq!(results[1].line, 2);
        assert_eq!(results[1].timestamp, "");

        assert_eq!(results[63].history, "exit");
        assert_eq!(results[63].line, 64);
        assert_eq!(results[63].timestamp, "1973-07-10T00:11:51.000Z");

        assert_eq!(results[61].history, "#echo");
        assert_eq!(results[61].line, 62);
        assert_eq!(results[61].timestamp, "");
    }

    #[test]
    fn test_parse_line() {
        let test_line = "#1659581179";
        let test_regex = Regex::new(r"^#([0-9]+)$").unwrap();
        let results = parse_line(test_line, &test_regex).unwrap();

        assert_eq!(results, 1659581179);
    }

    #[test]
    #[should_panic(expected = "Regex")]
    fn test_bad_timestamp_parse_line() {
        let test_line = "#1659581179aaaaaaaa";
        let test_regex = Regex::new(r"^#([0-9]+)$").unwrap();
        let results = parse_line(test_line, &test_regex).unwrap();

        assert_eq!(results, 0);
    }
}
