use crate::filesystem::files::is_file;
use crate::{
    artifacts::os::unix::shell_history::error::ShellError,
    filesystem::{
        directory::{get_root_home, get_user_paths},
        files::get_filename,
    },
};
use log::{error, warn};
use serde::Serialize;
use std::{
    fs::File,
    io::{BufRead, BufReader},
};

#[derive(Debug, Serialize)]
pub(crate) struct PythonHistory {
    pub(crate) history: Vec<PythonHistoryData>,
    pub(crate) path: String,
    pub(crate) user: String,
}

#[derive(Debug, Serialize)]
pub(crate) struct PythonHistoryData {
    pub(crate) history: String,
    pub(crate) line: usize,
}

impl PythonHistory {
    /// Get all python history and session data for all users
    pub(crate) fn get_user_python_history() -> Result<Vec<PythonHistory>, ShellError> {
        let user_path_result = get_user_paths();
        let mut user_paths = match user_path_result {
            Ok(result) => result,
            Err(err) => {
                error!("[shell_history] Could not get user paths for python history: {err:?}");
                return Err(ShellError::UserPaths);
            }
        };
        let root_path_result = get_root_home();
        match root_path_result {
            Ok(result) => user_paths.push(result),
            Err(err) => {
                error!("[shell_history] Could not get root home for python history: {err:?}",);
            }
        }
        PythonHistory::python(&user_paths)
    }

    /// Read all users python history file
    fn python(user_paths: &[String]) -> Result<Vec<PythonHistory>, ShellError> {
        let mut shell_history: Vec<PythonHistory> = Vec::new();

        for path in user_paths {
            let python_history = format!("{path}/.python_history");
            if is_file(&python_history) {
                let python_data = PythonHistory::parse_python(&python_history)?;
                let python_history = PythonHistory {
                    history: python_data,
                    path: python_history,
                    user: get_filename(path),
                };
                shell_history.push(python_history);
            }
        }
        Ok(shell_history)
    }

    // Parse the `python_history` file
    fn parse_python(python_history: &str) -> Result<Vec<PythonHistoryData>, ShellError> {
        let mut python_data: Vec<PythonHistoryData> = Vec::new();

        let python_file_result = File::open(python_history);
        let python_file = match python_file_result {
            Ok(results) => results,
            Err(err) => {
                error!(
                    "[shell_history] Failed to open python file {python_history}, error: {err:?}",
                );
                return Err(ShellError::File);
            }
        };

        let python_reader = BufReader::new(python_file);

        // Read each line and parse the associated data
        for (line_number, entry) in python_reader.lines().enumerate() {
            let python_entry = match entry {
                Ok(result) => result,
                Err(err) => {
                    warn!(
                        "[shell_history] Failed to read python line in file {python_history}, error: {err:?}"
                    );
                    continue;
                }
            };
            let python_history = PythonHistoryData {
                history: python_entry,
                line: line_number + 1,
            };
            python_data.push(python_history);
        }

        Ok(python_data)
    }
}

#[cfg(test)]
mod tests {
    use crate::filesystem::directory::get_user_paths;

    use super::PythonHistory;
    use std::path::PathBuf;

    #[test]
    fn test_get_macos_user_python_history() {
        let _ = PythonHistory::get_user_python_history().unwrap();
    }

    #[test]
    fn test_get_python_history() {
        let start_path = get_user_paths().unwrap();

        let _ = PythonHistory::python(&start_path).unwrap();
    }

    #[test]
    fn test_parse_python() {
        let mut test_location = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
        test_location.push("tests/test_data/unix/python/python_history");

        let results = PythonHistory::parse_python(&test_location.display().to_string()).unwrap();
        assert_eq!(results.len(), 373);

        assert_eq!(results[0].history, "import lief");
        assert_eq!(results[0].line, 1);

        assert_eq!(results[372].history, "results = lief.parse(\"/System/Library/PrivateFrameworks/UserActivity.framework/Agents/useractivityd\")");
        assert_eq!(results[372].line, 373);
    }
}
