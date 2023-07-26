use crate::{
    artifacts::applications::chromium::{downloads::ChromiumDownloads, history::ChromiumHistory},
    runtime::error::RuntimeError,
};
use deno_core::{error::AnyError, op};
use log::error;

#[op]
/// Get `Chromium` history for all users
fn get_chromium_users_history() -> Result<String, AnyError> {
    let history_results = ChromiumHistory::get_history();
    let history = match history_results {
        Ok(results) => results,
        Err(err) => {
            error!("[runtime] Failed to get chromium history: {err:?}");
            return Err(RuntimeError::ExecuteScript.into());
        }
    };
    let results = serde_json::to_string(&history)?;
    Ok(results)
}

#[op]
/// Get `Chromium` history from provided path
fn get_chromium_history(path: String) -> Result<String, AnyError> {
    let history_results = ChromiumHistory::history_query(&path);
    let history = match history_results {
        Ok(results) => results,
        Err(err) => {
            error!("[runtime] Failed to get chromium history at {path}: {err:?}");
            return Err(RuntimeError::ExecuteScript.into());
        }
    };
    let results = serde_json::to_string(&history)?;
    Ok(results)
}

#[op]
/// Get `Chromium` downloads for all users
fn get_chromium_users_downloads() -> Result<String, AnyError> {
    let downloads_results = ChromiumDownloads::get_downloads();
    let downloads = match downloads_results {
        Ok(results) => results,
        Err(err) => {
            error!("[runtime] Failed to get chromium downloads: {err:?}");
            return Err(RuntimeError::ExecuteScript.into());
        }
    };
    let results = serde_json::to_string(&downloads)?;
    Ok(results)
}

#[op]
/// Get `Chromium` downloads from provided path
fn get_chromium_downloads(path: String) -> Result<String, AnyError> {
    let downloads_results = ChromiumDownloads::downloads_query(&path);
    let downloads = match downloads_results {
        Ok(results) => results,
        Err(err) => {
            error!("[runtime] Failed to get chromium downloads at {path}: {err:?}");
            return Err(RuntimeError::ExecuteScript.into());
        }
    };
    let results = serde_json::to_string(&downloads)?;
    Ok(results)
}

#[cfg(test)]
mod tests {
    use crate::{
        runtime::deno::execute_script, structs::artifacts::runtime::script::JSScript,
        utils::artemis_toml::Output,
    };

    fn output_options(name: &str, output: &str, directory: &str, compress: bool) -> Output {
        Output {
            name: name.to_string(),
            directory: directory.to_string(),
            format: String::from("jsonl"),
            compress,
            url: Some(String::new()),
            api_key: Some(String::new()),
            endpoint_id: String::from("abcd"),
            collection_id: 0,
            output: output.to_string(),
            filter_name: Some(String::new()),
            filter_script: Some(String::new()),
            logging: Some(String::new()),
        }
    }

    #[test]
    fn test_get_chromium_users_history() {
        let test = "Ly8gZGVuby1mbXQtaWdub3JlLWZpbGUKLy8gZGVuby1saW50LWlnbm9yZS1maWxlCi8vIFRoaXMgY29kZSB3YXMgYnVuZGxlZCB1c2luZyBgZGVubyBidW5kbGVgIGFuZCBpdCdzIG5vdCByZWNvbW1lbmRlZCB0byBlZGl0IGl0IG1hbnVhbGx5CgpmdW5jdGlvbiBnZXRfY2hyb21pdW1fdXNlcnNfaGlzdG9yeSgpIHsKICAgIGNvbnN0IGRhdGEgPSBEZW5vLmNvcmUub3BzLmdldF9jaHJvbWl1bV91c2Vyc19oaXN0b3J5KCk7CiAgICBjb25zdCBoaXN0b3J5ID0gSlNPTi5wYXJzZShkYXRhKTsKICAgIHJldHVybiBoaXN0b3J5Owp9CmZ1bmN0aW9uIGdldENocm9taXVtVXNlcnNIaXN0b3J5KCkgewogICAgcmV0dXJuIGdldF9jaHJvbWl1bV91c2Vyc19oaXN0b3J5KCk7Cn0KZnVuY3Rpb24gbWFpbigpIHsKICAgIGNvbnN0IGRhdGEgPSBnZXRDaHJvbWl1bVVzZXJzSGlzdG9yeSgpOwogICAgcmV0dXJuIGRhdGE7Cn0KbWFpbigpOw==";
        let mut output = output_options("runtime_test", "local", "./tmp", false);
        let script = JSScript {
            name: String::from("chromium_history"),
            script: test.to_string(),
        };
        execute_script(&mut output, &script).unwrap();
    }

    #[test]
    #[cfg(target_os = "macos")]
    fn test_get_chromium_history() {
        let test = "Ly8gaHR0cHM6Ly9yYXcuZ2l0aHVidXNlcmNvbnRlbnQuY29tL3B1ZmZ5Y2lkL2FydGVtaXMtYXBpL21hc3Rlci9zcmMvYXBwbGljYXRpb25zL2Nocm9taXVtLnRzCmZ1bmN0aW9uIGdldENocm9taXVtSGlzdG9yeShwYXRoKSB7CiAgY29uc3QgZGF0YSA9IERlbm8uY29yZS5vcHMuZ2V0X2Nocm9taXVtX2hpc3RvcnkocGF0aCk7CiAgY29uc3QgaGlzdG9yeSA9IEpTT04ucGFyc2UoZGF0YSk7CiAgcmV0dXJuIGhpc3Rvcnk7Cn0KCi8vIGh0dHBzOi8vcmF3LmdpdGh1YnVzZXJjb250ZW50LmNvbS9wdWZmeWNpZC9hcnRlbWlzLWFwaS9tYXN0ZXIvc3JjL2ZpbGVzeXN0ZW0vZGlyZWN0b3J5LnRzCmFzeW5jIGZ1bmN0aW9uIHJlYWREaXIocGF0aCkgewogIGNvbnN0IGRhdGEgPSBKU09OLnBhcnNlKGF3YWl0IGZzLnJlYWREaXIocGF0aCkpOwogIHJldHVybiBkYXRhOwp9CgovLyBtYWluLnRzCmFzeW5jIGZ1bmN0aW9uIG1haW4oKSB7CiAgcmV0dXJuIGF3YWl0IHJlY3Vyc2VfZGlyKCIvVXNlcnMiKTsKfQphc3luYyBmdW5jdGlvbiByZWN1cnNlX2RpcihzdGFydF9wYXRoKSB7CiAgbGV0IHJlc3VsdHMgPSBudWxsOwogIGZvciAoY29uc3QgZW50cnkgb2YgYXdhaXQgcmVhZERpcihzdGFydF9wYXRoKSkgewogICAgY29uc3QgcGF0aCA9IGAke3N0YXJ0X3BhdGh9LyR7ZW50cnkuZmlsZW5hbWV9YDsKICAgIGlmIChwYXRoLmluY2x1ZGVzKCJ0ZXN0X2RhdGEiKSAmJiBlbnRyeS5maWxlbmFtZSA9PSAiSGlzdG9yeSIgJiYgZW50cnkuaXNfZmlsZSkgewogICAgICByZXN1bHRzID0gZ2V0Q2hyb21pdW1IaXN0b3J5KHBhdGgpOwogICAgICByZXR1cm4gcmVzdWx0czsKICAgIH0KICAgIGlmIChlbnRyeS5pc19kaXJlY3RvcnkpIHsKICAgICAgdHJ5IHsKICAgICAgICByZXN1bHRzID0gYXdhaXQgcmVjdXJzZV9kaXIocGF0aCk7CiAgICAgICAgaWYgKHJlc3VsdHMgIT0gbnVsbCkgewogICAgICAgICAgcmV0dXJuIHJlc3VsdHM7CiAgICAgICAgfQogICAgICB9IGNhdGNoIChfZSkgewogICAgICAgIGNvbnRpbnVlOwogICAgICB9CiAgICB9CiAgfQogIHJldHVybiByZXN1bHRzOwp9Cm1haW4oKTsK";
        let mut output = output_options("runtime_test", "local", "./tmp", false);
        let script = JSScript {
            name: String::from("chromium_path_history"),
            script: test.to_string(),
        };
        execute_script(&mut output, &script).unwrap();
    }

    #[test]
    #[cfg(target_os = "windows")]
    fn test_get_chromium_history() {
        let test = "Ly8gaHR0cHM6Ly9yYXcuZ2l0aHVidXNlcmNvbnRlbnQuY29tL3B1ZmZ5Y2lkL2FydGVtaXMtYXBpL21hc3Rlci9zcmMvYXBwbGljYXRpb25zL2Nocm9taXVtLnRzCmZ1bmN0aW9uIGdldENocm9taXVtSGlzdG9yeShwYXRoKSB7CiAgY29uc3QgZGF0YSA9IERlbm8uY29yZS5vcHMuZ2V0X2Nocm9taXVtX2hpc3RvcnkocGF0aCk7CiAgY29uc3QgaGlzdG9yeSA9IEpTT04ucGFyc2UoZGF0YSk7CiAgcmV0dXJuIGhpc3Rvcnk7Cn0KCi8vIGh0dHBzOi8vcmF3LmdpdGh1YnVzZXJjb250ZW50LmNvbS9wdWZmeWNpZC9hcnRlbWlzLWFwaS9tYXN0ZXIvc3JjL2ZpbGVzeXN0ZW0vZGlyZWN0b3J5LnRzCmFzeW5jIGZ1bmN0aW9uIHJlYWREaXIocGF0aCkgewogIGNvbnN0IGRhdGEgPSBKU09OLnBhcnNlKGF3YWl0IGZzLnJlYWREaXIocGF0aCkpOwogIHJldHVybiBkYXRhOwp9CgovLyBtYWluLnRzCmFzeW5jIGZ1bmN0aW9uIG1haW4oKSB7CiAgcmV0dXJuIGF3YWl0IHJlY3Vyc2VfZGlyKCJDOlxcVXNlcnMiKTsKfQphc3luYyBmdW5jdGlvbiByZWN1cnNlX2RpcihzdGFydF9wYXRoKSB7CiAgbGV0IHJlc3VsdHMgPSBudWxsOwogIGZvciAoY29uc3QgZW50cnkgb2YgYXdhaXQgcmVhZERpcihzdGFydF9wYXRoKSkgewogICAgY29uc3QgcGF0aCA9IGAke3N0YXJ0X3BhdGh9LyR7ZW50cnkuZmlsZW5hbWV9YDsKICAgIGlmIChwYXRoLmluY2x1ZGVzKCJ0ZXN0X2RhdGEiKSAmJiBlbnRyeS5maWxlbmFtZSA9PSAiSGlzdG9yeSIgJiYgZW50cnkuaXNfZmlsZSkgewogICAgICByZXN1bHRzID0gZ2V0Q2hyb21pdW1IaXN0b3J5KHBhdGgpOwogICAgICByZXR1cm4gcmVzdWx0czsKICAgIH0KICAgIGlmIChlbnRyeS5pc19kaXJlY3RvcnkpIHsKICAgICAgdHJ5IHsKICAgICAgICByZXN1bHRzID0gYXdhaXQgcmVjdXJzZV9kaXIocGF0aCk7CiAgICAgICAgaWYgKHJlc3VsdHMgIT0gbnVsbCkgewogICAgICAgICAgcmV0dXJuIHJlc3VsdHM7CiAgICAgICAgfQogICAgICB9IGNhdGNoIChfZSkgewogICAgICAgIGNvbnRpbnVlOwogICAgICB9CiAgICB9CiAgfQogIHJldHVybiByZXN1bHRzOwp9Cm1haW4oKTsK";
        let mut output = output_options("runtime_test", "local", "./tmp", false);
        let script = JSScript {
            name: String::from("chromium_path_history"),
            script: test.to_string(),
        };
        execute_script(&mut output, &script).unwrap();
    }

    #[test]
    fn test_get_chromium_users_downloads() {
        let test = "Ly8gLi4vLi4vYXJ0ZW1pcy1hcGkvc3JjL2FwcGxpY2F0aW9ucy9jaHJvbWl1bS50cwpmdW5jdGlvbiBnZXRfY2hyb21pdW1fdXNlcnNfZG93bmxvYWRzKCkgewogIGNvbnN0IGRhdGEgPSBEZW5vLmNvcmUub3BzLmdldF9jaHJvbWl1bV91c2Vyc19kb3dubG9hZHMoKTsKICBjb25zdCBkb3dubG9hZHMgPSBKU09OLnBhcnNlKGRhdGEpOwogIHJldHVybiBkb3dubG9hZHM7Cn0KCi8vIC4uLy4uL2FydGVtaXMtYXBpL21vZC50cwpmdW5jdGlvbiBnZXRDaHJvbWl1bVVzZXJzRG93bmxvYWRzKCkgewogIHJldHVybiBnZXRfY2hyb21pdW1fdXNlcnNfZG93bmxvYWRzKCk7Cn0KCi8vIG1haW4udHMKZnVuY3Rpb24gbWFpbigpIHsKICByZXR1cm4gZ2V0Q2hyb21pdW1Vc2Vyc0Rvd25sb2FkcygpOwp9Cm1haW4oKTsK";
        let mut output = output_options("runtime_test", "local", "./tmp", false);
        let script = JSScript {
            name: String::from("chromium_downloads"),
            script: test.to_string(),
        };
        execute_script(&mut output, &script).unwrap();
    }

    #[test]
    #[cfg(target_os = "macos")]
    fn test_get_chromium_downloads() {
        let test = "Ly8gaHR0cHM6Ly9yYXcuZ2l0aHVidXNlcmNvbnRlbnQuY29tL3B1ZmZ5Y2lkL2FydGVtaXMtYXBpL21hc3Rlci9zcmMvYXBwbGljYXRpb25zL2Nocm9taXVtLnRzCmZ1bmN0aW9uIGdldENocm9taXVtRG93bmxvYWRzKHBhdGgpIHsKICBjb25zdCBkYXRhID0gRGVuby5jb3JlLm9wcy5nZXRfY2hyb21pdW1fZG93bmxvYWRzKHBhdGgpOwogIGNvbnN0IGRvd25sb2FkcyA9IEpTT04ucGFyc2UoZGF0YSk7CiAgcmV0dXJuIGRvd25sb2FkczsKfQoKLy8gaHR0cHM6Ly9yYXcuZ2l0aHVidXNlcmNvbnRlbnQuY29tL3B1ZmZ5Y2lkL2FydGVtaXMtYXBpL21hc3Rlci9zcmMvZmlsZXN5c3RlbS9kaXJlY3RvcnkudHMKYXN5bmMgZnVuY3Rpb24gcmVhZERpcihwYXRoKSB7CiAgY29uc3QgZGF0YSA9IEpTT04ucGFyc2UoYXdhaXQgZnMucmVhZERpcihwYXRoKSk7CiAgcmV0dXJuIGRhdGE7Cn0KCi8vIG1haW4udHMKYXN5bmMgZnVuY3Rpb24gbWFpbigpIHsKICByZXR1cm4gYXdhaXQgcmVjdXJzZV9kaXIoIi9Vc2VycyIpOwp9CmFzeW5jIGZ1bmN0aW9uIHJlY3Vyc2VfZGlyKHN0YXJ0X3BhdGgpIHsKICBsZXQgcmVzdWx0cyA9IG51bGw7CiAgZm9yIChjb25zdCBlbnRyeSBvZiBhd2FpdCByZWFkRGlyKHN0YXJ0X3BhdGgpKSB7CiAgICBjb25zdCBwYXRoID0gYCR7c3RhcnRfcGF0aH0vJHtlbnRyeS5maWxlbmFtZX1gOwogICAgaWYgKHBhdGguaW5jbHVkZXMoInRlc3RfZGF0YSIpICYmIGVudHJ5LmZpbGVuYW1lID09ICJIaXN0b3J5IiAmJiBlbnRyeS5maWxlbmFtZSkgewogICAgICByZXN1bHRzID0gZ2V0Q2hyb21pdW1Eb3dubG9hZHMocGF0aCk7CiAgICAgIHJldHVybiByZXN1bHRzOwogICAgfQogICAgaWYgKGVudHJ5LmlzX2RpcmVjdG9yeSkgewogICAgICB0cnkgewogICAgICAgIHJlc3VsdHMgPSBhd2FpdCByZWN1cnNlX2RpcihwYXRoKTsKICAgICAgICBpZiAocmVzdWx0cyAhPSBudWxsKSB7CiAgICAgICAgICByZXR1cm4gcmVzdWx0czsKICAgICAgICB9CiAgICAgIH0gY2F0Y2ggKF9lKSB7CiAgICAgICAgY29udGludWU7CiAgICAgIH0KICAgIH0KICB9CiAgcmV0dXJuIHJlc3VsdHM7Cn0KbWFpbigpOwo=";
        let mut output = output_options("runtime_test", "local", "./tmp", false);
        let script = JSScript {
            name: String::from("chromium_path_downloads"),
            script: test.to_string(),
        };
        execute_script(&mut output, &script).unwrap();
    }

    #[test]
    #[cfg(target_os = "windows")]
    fn test_get_chromium_downloads() {
        let test = "Ly8gaHR0cHM6Ly9yYXcuZ2l0aHVidXNlcmNvbnRlbnQuY29tL3B1ZmZ5Y2lkL2FydGVtaXMtYXBpL21hc3Rlci9zcmMvYXBwbGljYXRpb25zL2Nocm9taXVtLnRzCmZ1bmN0aW9uIGdldENocm9taXVtRG93bmxvYWRzKHBhdGgpIHsKICBjb25zdCBkYXRhID0gRGVuby5jb3JlLm9wcy5nZXRfY2hyb21pdW1fZG93bmxvYWRzKHBhdGgpOwogIGNvbnN0IGRvd25sb2FkcyA9IEpTT04ucGFyc2UoZGF0YSk7CiAgcmV0dXJuIGRvd25sb2FkczsKfQoKLy8gaHR0cHM6Ly9yYXcuZ2l0aHVidXNlcmNvbnRlbnQuY29tL3B1ZmZ5Y2lkL2FydGVtaXMtYXBpL21hc3Rlci9zcmMvZmlsZXN5c3RlbS9kaXJlY3RvcnkudHMKYXN5bmMgZnVuY3Rpb24gcmVhZERpcihwYXRoKSB7CiAgY29uc3QgZGF0YSA9IEpTT04ucGFyc2UoYXdhaXQgZnMucmVhZERpcihwYXRoKSk7CiAgcmV0dXJuIGRhdGE7Cn0KCi8vIG1haW4udHMKYXN5bmMgZnVuY3Rpb24gbWFpbigpIHsKICByZXR1cm4gYXdhaXQgcmVjdXJzZV9kaXIoIkM6XFxVc2VycyIpOwp9CmFzeW5jIGZ1bmN0aW9uIHJlY3Vyc2VfZGlyKHN0YXJ0X3BhdGgpIHsKICBsZXQgcmVzdWx0cyA9IG51bGw7CiAgZm9yIChjb25zdCBlbnRyeSBvZiBhd2FpdCByZWFkRGlyKHN0YXJ0X3BhdGgpKSB7CiAgICBjb25zdCBwYXRoID0gYCR7c3RhcnRfcGF0aH0vJHtlbnRyeS5maWxlbmFtZX1gOwogICAgaWYgKHBhdGguaW5jbHVkZXMoInRlc3RfZGF0YSIpICYmIGVudHJ5LmZpbGVuYW1lID09ICJIaXN0b3J5IiAmJiBlbnRyeS5maWxlbmFtZSkgewogICAgICByZXN1bHRzID0gZ2V0Q2hyb21pdW1Eb3dubG9hZHMocGF0aCk7CiAgICAgIHJldHVybiByZXN1bHRzOwogICAgfQogICAgaWYgKGVudHJ5LmlzX2RpcmVjdG9yeSkgewogICAgICB0cnkgewogICAgICAgIHJlc3VsdHMgPSBhd2FpdCByZWN1cnNlX2RpcihwYXRoKTsKICAgICAgICBpZiAocmVzdWx0cyAhPSBudWxsKSB7CiAgICAgICAgICByZXR1cm4gcmVzdWx0czsKICAgICAgICB9CiAgICAgIH0gY2F0Y2ggKF9lKSB7CiAgICAgICAgY29udGludWU7CiAgICAgIH0KICAgIH0KICB9CiAgcmV0dXJuIHJlc3VsdHM7Cn0KbWFpbigpOwo=";
        let mut output = output_options("runtime_test", "local", "./tmp", false);
        let script = JSScript {
            name: String::from("chromium_path_downloads"),
            script: test.to_string(),
        };
        execute_script(&mut output, &script).unwrap();
    }
}
