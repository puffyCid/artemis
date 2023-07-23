use crate::{
    artifacts::applications::firefox::{downloads::FirefoxDownloads, history::FirefoxHistory},
    runtime::error::RuntimeError,
};
use deno_core::{error::AnyError, op};
use log::error;

#[op]
/// Get `Firefox` history for all users
fn get_firefox_users_history() -> Result<String, AnyError> {
    let history_results = FirefoxHistory::get_history();
    let history = match history_results {
        Ok(results) => results,
        Err(err) => {
            error!("[runtime] Failed to get firefox history: {err:?}");
            return Err(RuntimeError::ExecuteScript.into());
        }
    };
    let results = serde_json::to_string(&history)?;
    Ok(results)
}

#[op]
/// Get `Firefox` history from provided path
fn get_firefox_history(path: String) -> Result<String, AnyError> {
    let history_results = FirefoxHistory::history_query(&path);
    let history = match history_results {
        Ok(results) => results,
        Err(err) => {
            error!("[runtime] Failed to get firefox history at {path}: {err:?}");
            return Err(RuntimeError::ExecuteScript.into());
        }
    };
    let results = serde_json::to_string(&history)?;
    Ok(results)
}

#[op]
/// Get `Firefox` downloads for all users
fn get_firefox_users_downloads() -> Result<String, AnyError> {
    let downloads_results = FirefoxDownloads::get_downloads();
    let downloads = match downloads_results {
        Ok(results) => results,
        Err(err) => {
            error!("[runtime] Failed to get firefox downloads: {err:?}");
            return Err(RuntimeError::ExecuteScript.into());
        }
    };
    let results = serde_json::to_string(&downloads)?;
    Ok(results)
}

#[op]
/// Get `Firefox` downloads from provided path
fn get_firefox_downloads(path: String) -> Result<String, AnyError> {
    let downloads_results = FirefoxDownloads::downloads_query(&path);
    let downloads = match downloads_results {
        Ok(results) => results,
        Err(err) => {
            error!("[runtime] Failed to get firefox downloads at {path}: {err:?}");
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
    fn test_get_firefox_users_history() {
        let test = "Ly8gLi4vLi4vYXJ0ZW1pcy1hcGkvc3JjL2FwcGxpY2F0aW9ucy9maXJlZm94LnRzCmZ1bmN0aW9uIGdldF9maXJlZm94X3VzZXJzX2hpc3RvcnkoKSB7CiAgY29uc3QgZGF0YSA9IERlbm8uY29yZS5vcHMuZ2V0X2ZpcmVmb3hfdXNlcnNfaGlzdG9yeSgpOwogIGNvbnN0IGhpc3RvcnkgPSBKU09OLnBhcnNlKGRhdGEpOwogIHJldHVybiBoaXN0b3J5Owp9CgovLyAuLi8uLi9hcnRlbWlzLWFwaS9tb2QudHMKZnVuY3Rpb24gZ2V0RmlyZWZveFVzZXJzSGlzdG9yeSgpIHsKICByZXR1cm4gZ2V0X2ZpcmVmb3hfdXNlcnNfaGlzdG9yeSgpOwp9CgovLyBtYWluLnRzCmZ1bmN0aW9uIG1haW4oKSB7CiAgY29uc3QgZGF0YSA9IGdldEZpcmVmb3hVc2Vyc0hpc3RvcnkoKTsKICByZXR1cm4gZGF0YTsKfQptYWluKCk7Cg==";
        let mut output = output_options("runtime_test", "local", "./tmp", false);
        let script = JSScript {
            name: String::from("firefox_history"),
            script: test.to_string(),
        };
        execute_script(&mut output, &script).unwrap();
    }

    #[test]
    fn test_get_firefox_users_downloads() {
        let test = "Ly8gLi4vLi4vYXJ0ZW1pcy1hcGkvc3JjL2FwcGxpY2F0aW9ucy9maXJlZm94LnRzCmZ1bmN0aW9uIGdldF9maXJlZm94X3VzZXJzX2Rvd25sb2FkcygpIHsKICBjb25zdCBkYXRhID0gRGVuby5jb3JlLm9wcy5nZXRfZmlyZWZveF91c2Vyc19kb3dubG9hZHMoKTsKICBjb25zdCBkb3dubG9hZHMgPSBKU09OLnBhcnNlKGRhdGEpOwogIHJldHVybiBkb3dubG9hZHM7Cn0KCi8vIC4uLy4uL2FydGVtaXMtYXBpL21vZC50cwpmdW5jdGlvbiBnZXRGaXJlZm94VXNlcnNEb3dubG9hZHMoKSB7CiAgcmV0dXJuIGdldF9maXJlZm94X3VzZXJzX2Rvd25sb2FkcygpOwp9CgovLyBtYWluLnRzCmZ1bmN0aW9uIG1haW4oKSB7CiAgcmV0dXJuIGdldEZpcmVmb3hVc2Vyc0Rvd25sb2FkcygpOwp9Cm1haW4oKTsK";
        let mut output = output_options("runtime_test", "local", "./tmp", false);
        let script = JSScript {
            name: String::from("firefox_downloads"),
            script: test.to_string(),
        };
        execute_script(&mut output, &script).unwrap();
    }

    #[test]
    #[cfg(target_os = "macos")]
    fn test_get_firefox_history() {
        let test = "Ly8gaHR0cHM6Ly9yYXcuZ2l0aHVidXNlcmNvbnRlbnQuY29tL3B1ZmZ5Y2lkL2FydGVtaXMtYXBpL21hc3Rlci9zcmMvYXBwbGljYXRpb25zL2ZpcmVmb3gudHMKZnVuY3Rpb24gZ2V0RmlyZWZveEhpc3RvcnkocGF0aCkgewogIGNvbnN0IGRhdGEgPSBEZW5vLmNvcmUub3BzLmdldF9maXJlZm94X2hpc3RvcnkocGF0aCk7CiAgY29uc3QgaGlzdG9yeSA9IEpTT04ucGFyc2UoZGF0YSk7CiAgcmV0dXJuIGhpc3Rvcnk7Cn0KCi8vIGh0dHBzOi8vcmF3LmdpdGh1YnVzZXJjb250ZW50LmNvbS9wdWZmeWNpZC9hcnRlbWlzLWFwaS9tYXN0ZXIvc3JjL2ZpbGVzeXN0ZW0vZGlyZWN0b3J5LnRzCmZ1bmN0aW9uIHJlYWREaXIocGF0aCkgewogIGNvbnN0IGRhdGEgPSBmcy5yZWFkRGlyKHBhdGgpOwogIHJldHVybiBkYXRhOwp9CgovLyBtYWluLnRzCmFzeW5jIGZ1bmN0aW9uIG1haW4oKSB7CiAgY29uc3QgYmluX3BhdGggPSAiL1VzZXJzIjsKICByZXR1cm4gYXdhaXQgcmVjdXJzZV9kaXIoYmluX3BhdGgpOwp9CmFzeW5jIGZ1bmN0aW9uIHJlY3Vyc2VfZGlyKHN0YXJ0X3BhdGgpIHsKICBsZXQgcmVzdWx0cyA9IG51bGw7CiAgZm9yIGF3YWl0IChjb25zdCBlbnRyeSBvZiBhd2FpdCByZWFkRGlyKHN0YXJ0X3BhdGgpKSB7CiAgICBjb25zdCBwYXRoID0gYCR7c3RhcnRfcGF0aH0vJHtlbnRyeS5maWxlbmFtZX1gOwogICAgaWYgKHBhdGguaW5jbHVkZXMoInRlc3RfZGF0YSIpICYmIGVudHJ5LmZpbGVuYW1lID09ICJwbGFjZXMuc3FsaXRlIiAmJiBlbnRyeS5pc19maWxlKSB7CiAgICAgIHJlc3VsdHMgPSBnZXRGaXJlZm94SGlzdG9yeShwYXRoKTsKICAgICAgcmV0dXJuIHJlc3VsdHM7CiAgICB9CiAgICBpZiAoZW50cnkuaXNfZGlyZWN0b3J5KSB7CiAgICAgIHRyeSB7CiAgICAgICAgcmVzdWx0cyA9IHJlY3Vyc2VfZGlyKHBhdGgpOwogICAgICAgIGlmIChyZXN1bHRzICE9IG51bGwpIHsKICAgICAgICAgIHJldHVybiByZXN1bHRzOwogICAgICAgIH0KICAgICAgfSBjYXRjaCAoX2UpIHsKICAgICAgICBjb250aW51ZTsKICAgICAgfQogICAgfQogIH0KICByZXR1cm4gcmVzdWx0czsKfQptYWluKCk7Cg==";
        let mut output = output_options("runtime_test", "local", "./tmp", false);
        let script = JSScript {
            name: String::from("firefox_history_path"),
            script: test.to_string(),
        };
        execute_script(&mut output, &script).unwrap();
    }

    #[test]
    #[cfg(target_os = "windows")]
    fn test_get_firefox_history() {
        let test = "Ly8gLi4vLi4vYXJ0ZW1pcy1hcGkvc3JjL2FwcGxpY2F0aW9ucy9maXJlZm94LnRzCmZ1bmN0aW9uIGdldF9maXJlZm94X2hpc3RvcnkocGF0aCkgewogIGNvbnN0IGRhdGEgPSBEZW5vW0Rlbm8uaW50ZXJuYWxdLmNvcmUub3BzLmdldF9maXJlZm94X2hpc3RvcnkocGF0aCk7CiAgY29uc3QgaGlzdG9yeSA9IEpTT04ucGFyc2UoZGF0YSk7CiAgcmV0dXJuIGhpc3Rvcnk7Cn0KCi8vIC4uLy4uL2FydGVtaXMtYXBpL21vZC50cwpmdW5jdGlvbiBnZXRGaXJlZm94SGlzdG9yeShwYXRoKSB7CiAgcmV0dXJuIGdldF9maXJlZm94X2hpc3RvcnkocGF0aCk7Cn0KCi8vIG1haW4udHMKZnVuY3Rpb24gbWFpbigpIHsKICBjb25zdCBiaW5fcGF0aCA9ICJDOlxcVXNlcnMiOwogIHJldHVybiByZWN1cnNlX2RpcihiaW5fcGF0aCk7Cn0KZnVuY3Rpb24gcmVjdXJzZV9kaXIoc3RhcnRfcGF0aCkgewogIGxldCByZXN1bHRzID0gbnVsbDsKICBmb3IgKGNvbnN0IGVudHJ5IG9mIERlbm8ucmVhZERpclN5bmMoc3RhcnRfcGF0aCkpIHsKICAgIGNvbnN0IHBhdGggPSBgJHtzdGFydF9wYXRofVxcJHtlbnRyeS5uYW1lfWA7CiAgICBpZiAocGF0aC5pbmNsdWRlcygidGVzdF9kYXRhIikgJiYgZW50cnkubmFtZSA9PSAicGxhY2VzLnNxbGl0ZSIgJiYgZW50cnkuaXNGaWxlKSB7CiAgICAgIHJlc3VsdHMgPSBnZXRGaXJlZm94SGlzdG9yeShwYXRoKTsKICAgICAgcmV0dXJuIHJlc3VsdHM7CiAgICB9CiAgICBpZiAoZW50cnkuaXNEaXJlY3RvcnkpIHsKICAgICAgdHJ5IHsKICAgICAgICByZXN1bHRzID0gcmVjdXJzZV9kaXIocGF0aCk7CiAgICAgICAgaWYgKHJlc3VsdHMgIT0gbnVsbCkgewogICAgICAgICAgcmV0dXJuIHJlc3VsdHM7CiAgICAgICAgfQogICAgICB9IGNhdGNoIChfZSkgewogICAgICAgIGNvbnRpbnVlOwogICAgICB9CiAgICB9CiAgfQogIHJldHVybiByZXN1bHRzOwp9Cm1haW4oKTsK";
        let mut output = output_options("runtime_test", "local", "./tmp", false);
        let script = JSScript {
            name: String::from("firefox_history_path"),
            script: test.to_string(),
        };
        execute_script(&mut output, &script).unwrap();
    }

    #[test]
    #[cfg(target_os = "macos")]
    fn test_get_firefox_downloads() {
        let test = "Ly8gaHR0cHM6Ly9yYXcuZ2l0aHVidXNlcmNvbnRlbnQuY29tL3B1ZmZ5Y2lkL2FydGVtaXMtYXBpL21hc3Rlci9zcmMvYXBwbGljYXRpb25zL2ZpcmVmb3gudHMKZnVuY3Rpb24gZ2V0RmlyZWZveERvd25sb2FkcyhwYXRoKSB7CiAgY29uc3QgZGF0YSA9IERlbm8uY29yZS5vcHMuZ2V0X2ZpcmVmb3hfZG93bmxvYWRzKHBhdGgpOwogIGNvbnN0IGRvd25sb2FkcyA9IEpTT04ucGFyc2UoZGF0YSk7CiAgcmV0dXJuIGRvd25sb2FkczsKfQoKLy8gaHR0cHM6Ly9yYXcuZ2l0aHVidXNlcmNvbnRlbnQuY29tL3B1ZmZ5Y2lkL2FydGVtaXMtYXBpL21hc3Rlci9zcmMvZmlsZXN5c3RlbS9kaXJlY3RvcnkudHMKZnVuY3Rpb24gcmVhZERpcihwYXRoKSB7CiAgY29uc3QgZGF0YSA9IGZzLnJlYWREaXIocGF0aCk7CiAgcmV0dXJuIGRhdGE7Cn0KCi8vIG1haW4udHMKYXN5bmMgZnVuY3Rpb24gbWFpbigpIHsKICByZXR1cm4gYXdhaXQgcmVjdXJzZV9kaXIoIi9Vc2VycyIpOwp9CmFzeW5jIGZ1bmN0aW9uIHJlY3Vyc2VfZGlyKHN0YXJ0X3BhdGgpIHsKICBsZXQgcmVzdWx0cyA9IG51bGw7CiAgZm9yIGF3YWl0IChjb25zdCBlbnRyeSBvZiByZWFkRGlyKHN0YXJ0X3BhdGgpKSB7CiAgICBjb25zdCBwYXRoID0gYCR7c3RhcnRfcGF0aH0vJHtlbnRyeS5maWxlbmFtZX1gOwogICAgaWYgKHBhdGguaW5jbHVkZXMoInRlc3RfZGF0YSIpICYmIGVudHJ5LmZpbGVuYW1lID09ICJwbGFjZXNfZG93bmxvYWRzLnNxbGl0ZSIgJiYgZW50cnkuaXNfZmlsZSkgewogICAgICByZXN1bHRzID0gZ2V0RmlyZWZveERvd25sb2FkcyhwYXRoKTsKICAgICAgcmV0dXJuIHJlc3VsdHM7CiAgICB9CiAgICBpZiAoZW50cnkuaXNfZGlyZWN0b3J5KSB7CiAgICAgIHRyeSB7CiAgICAgICAgcmVzdWx0cyA9IGF3YWl0IHJlY3Vyc2VfZGlyKHBhdGgpOwogICAgICAgIGlmIChyZXN1bHRzICE9IG51bGwpIHsKICAgICAgICAgIHJldHVybiByZXN1bHRzOwogICAgICAgIH0KICAgICAgfSBjYXRjaCAoX2UpIHsKICAgICAgICBjb250aW51ZTsKICAgICAgfQogICAgfQogIH0KICByZXR1cm4gcmVzdWx0czsKfQptYWluKCk7Cg==";
        let mut output = output_options("runtime_test", "local", "./tmp", false);
        let script = JSScript {
            name: String::from("firefox_downloads_path"),
            script: test.to_string(),
        };
        execute_script(&mut output, &script).unwrap();
    }

    #[test]
    #[cfg(target_os = "windows")]
    fn test_get_firefox_downloads() {
        let test = "Ly8gLi4vLi4vYXJ0ZW1pcy1hcGkvc3JjL2FwcGxpY2F0aW9ucy9maXJlZm94LnRzCmZ1bmN0aW9uIGdldF9maXJlZm94X2Rvd25sb2FkcyhwYXRoKSB7CiAgY29uc3QgZGF0YSA9IERlbm9bRGVuby5pbnRlcm5hbF0uY29yZS5vcHMuZ2V0X2ZpcmVmb3hfZG93bmxvYWRzKHBhdGgpOwogIGNvbnN0IGRvd25sb2FkcyA9IEpTT04ucGFyc2UoZGF0YSk7CiAgcmV0dXJuIGRvd25sb2FkczsKfQoKLy8gLi4vLi4vYXJ0ZW1pcy1hcGkvbW9kLnRzCmZ1bmN0aW9uIGdldEZpcmVmb3hEb3dubG9hZHMocGF0aCkgewogIHJldHVybiBnZXRfZmlyZWZveF9kb3dubG9hZHMocGF0aCk7Cn0KCi8vIG1haW4udHMKZnVuY3Rpb24gbWFpbigpIHsKICByZXR1cm4gcmVjdXJzZV9kaXIoIkM6XFxVc2VycyIpOwp9CmZ1bmN0aW9uIHJlY3Vyc2VfZGlyKHN0YXJ0X3BhdGgpIHsKICBsZXQgcmVzdWx0cyA9IG51bGw7CiAgZm9yIChjb25zdCBlbnRyeSBvZiBEZW5vLnJlYWREaXJTeW5jKHN0YXJ0X3BhdGgpKSB7CiAgICBjb25zdCBwYXRoID0gYCR7c3RhcnRfcGF0aH1cXCR7ZW50cnkubmFtZX1gOwogICAgaWYgKHBhdGguaW5jbHVkZXMoInRlc3RfZGF0YSIpICYmIGVudHJ5Lm5hbWUgPT0gInBsYWNlc19kb3dubG9hZHMuc3FsaXRlIiAmJiBlbnRyeS5pc0ZpbGUpIHsKICAgICAgcmVzdWx0cyA9IGdldEZpcmVmb3hEb3dubG9hZHMocGF0aCk7CiAgICAgIHJldHVybiByZXN1bHRzOwogICAgfQogICAgaWYgKGVudHJ5LmlzRGlyZWN0b3J5KSB7CiAgICAgIHRyeSB7CiAgICAgICAgcmVzdWx0cyA9IHJlY3Vyc2VfZGlyKHBhdGgpOwogICAgICAgIGlmIChyZXN1bHRzICE9IG51bGwpIHsKICAgICAgICAgIHJldHVybiByZXN1bHRzOwogICAgICAgIH0KICAgICAgfSBjYXRjaCAoX2UpIHsKICAgICAgICBjb250aW51ZTsKICAgICAgfQogICAgfQogIH0KICByZXR1cm4gcmVzdWx0czsKfQptYWluKCk7Cg==";
        let mut output = output_options("runtime_test", "local", "./tmp", false);
        let script = JSScript {
            name: String::from("firefox_downloads_path"),
            script: test.to_string(),
        };
        execute_script(&mut output, &script).unwrap();
    }
}
