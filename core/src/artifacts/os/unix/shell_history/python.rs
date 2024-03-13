use crate::filesystem::files::{file_lines, is_file};
use crate::{
    artifacts::os::unix::shell_history::error::ShellError,
    filesystem::{
        directory::{get_root_home, get_user_paths},
        files::get_filename,
    },
};
use common::unix::{PythonHistory, PythonHistoryData};
use log::{error, warn};

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
    python(&user_paths)
}

/// Read all users python history file
fn python(user_paths: &[String]) -> Result<Vec<PythonHistory>, ShellError> {
    let mut shell_history: Vec<PythonHistory> = Vec::new();

    for path in user_paths {
        let python_history = format!("{path}/.python_history");
        if is_file(&python_history) {
            let python_data = parse_python(&python_history)?;
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
    let file_result = file_lines(python_history);
    let py_iter = match file_result {
        Ok(result) => result,
        Err(err) => {
            error!("[shell_history] Could not read python_history lines: {err:?}");
            return Err(ShellError::File);
        }
    };

    let mut line_number = 1;
    // Read each line and parse the associated data
    for line_entry in py_iter {
        let python_entry = match line_entry {
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
            line: line_number,
        };
        python_data.push(python_history);
        line_number += 1;
    }

    Ok(python_data)
}

#[cfg(test)]
mod tests {
    use super::{get_user_python_history, python};
    use crate::{
        artifacts::os::unix::shell_history::python::parse_python,
        filesystem::directory::get_user_paths,
    };
    use std::path::PathBuf;

    #[test]
    fn test_get_macos_user_python_history() {
        let _ = get_user_python_history().unwrap();
    }

    #[test]
    fn test_get_python_history() {
        let start_path = get_user_paths().unwrap();

        let _ = python(&start_path).unwrap();
    }

    #[test]
    fn test_parse_python() {
        let mut test_location = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
        test_location.push("tests/test_data/unix/python/python_history");

        let results = parse_python(&test_location.display().to_string()).unwrap();
        assert_eq!(results.len(), 373);

        assert_eq!(results[0].history, "import lief");
        assert_eq!(results[0].line, 1);

        assert_eq!(results[372].history, "results = lief.parse(\"/System/Library/PrivateFrameworks/UserActivity.framework/Agents/useractivityd\")");
        assert_eq!(results[372].line, 373);
    }
}
