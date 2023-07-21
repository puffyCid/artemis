use crate::{
    artifacts::applications::safari::{downloads::SafariDownloads, history::SafariHistory},
    runtime::error::RuntimeError,
};
use deno_core::{error::AnyError, op};
use log::error;

#[op]
/// Get `Safari` history for all users
fn get_safari_users_history() -> Result<String, AnyError> {
    let history_results = SafariHistory::get_history();
    let history = match history_results {
        Ok(results) => results,
        Err(err) => {
            error!("[runtime] Failed to get safari history: {err:?}");
            return Err(RuntimeError::ExecuteScript.into());
        }
    };
    let results = serde_json::to_string_pretty(&history)?;
    Ok(results)
}

#[op]
/// Get `Safari` history from provided path
fn get_safari_history(path: String) -> Result<String, AnyError> {
    let history_results = SafariHistory::history_query(&path);
    let history = match history_results {
        Ok(results) => results,
        Err(err) => {
            error!("[runtime] Failed to get safari history at {path}: {err:?}");
            return Err(RuntimeError::ExecuteScript.into());
        }
    };
    let results = serde_json::to_string_pretty(&history)?;
    Ok(results)
}

#[op]
/// Get `Safari` downloads for all users
fn get_safari_users_downloads() -> Result<String, AnyError> {
    let downloads_results = SafariDownloads::get_downloads();
    let downloads = match downloads_results {
        Ok(results) => results,
        Err(err) => {
            error!("[runtime] Failed to get safari downloads: {err:?}");
            return Err(RuntimeError::ExecuteScript.into());
        }
    };
    let results = serde_json::to_string_pretty(&downloads)?;
    Ok(results)
}

#[op]
/// Get `Safari` downloads from provided path
fn get_safari_downloads(path: String) -> Result<String, AnyError> {
    let downloads_results = SafariDownloads::downloads_query(&path);
    let downloads = match downloads_results {
        Ok(results) => results,
        Err(err) => {
            error!("[runtime] Failed to get safari downloads at {path}: {err:?}");
            return Err(RuntimeError::ExecuteScript.into());
        }
    };
    let results = serde_json::to_string_pretty(&downloads)?;
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
    fn test_get_safari_users_history() {
        let test = "Ly8gLi4vLi4vYXJ0ZW1pcy1hcGkvc3JjL2FwcGxpY2F0aW9ucy9zYWZhcmkudHMKZnVuY3Rpb24gZ2V0X3NhZmFyaV91c2Vyc19oaXN0b3J5KCkgewogIGNvbnN0IGRhdGEgPSBEZW5vW0Rlbm8uaW50ZXJuYWxdLmNvcmUub3BzLmdldF9zYWZhcmlfdXNlcnNfaGlzdG9yeSgpOwogIGNvbnN0IGhpc3RvcnkgPSBKU09OLnBhcnNlKGRhdGEpOwogIHJldHVybiBoaXN0b3J5Owp9CgovLyAuLi8uLi9hcnRlbWlzLWFwaS9tb2QudHMKZnVuY3Rpb24gZ2V0U2FmYXJpVXNlcnNIaXN0b3J5KCkgewogIHJldHVybiBnZXRfc2FmYXJpX3VzZXJzX2hpc3RvcnkoKTsKfQoKLy8gbWFpbi50cwpmdW5jdGlvbiBtYWluKCkgewogIHJldHVybiBnZXRTYWZhcmlVc2Vyc0hpc3RvcnkoKTsKfQptYWluKCk7Cg==";
        let mut output = output_options("runtime_test", "local", "./tmp", false);
        let script = JSScript {
            name: String::from("safari_history"),
            script: test.to_string(),
        };
        execute_script(&mut output, &script).unwrap();
    }

    #[test]
    fn test_get_safari_history() {
        let test = "Ly8gLi4vLi4vYXJ0ZW1pcy1hcGkvc3JjL2FwcGxpY2F0aW9ucy9zYWZhcmkudHMKZnVuY3Rpb24gZ2V0X3NhZmFyaV9oaXN0b3J5KHBhdGgpIHsKICBjb25zdCBkYXRhID0gRGVub1tEZW5vLmludGVybmFsXS5jb3JlLm9wcy5nZXRfc2FmYXJpX2hpc3RvcnkocGF0aCk7CiAgY29uc3QgaGlzdG9yeSA9IEpTT04ucGFyc2UoZGF0YSk7CiAgcmV0dXJuIGhpc3Rvcnk7Cn0KCi8vIC4uLy4uL2FydGVtaXMtYXBpL21vZC50cwpmdW5jdGlvbiBnZXRTYWZhcmlIaXN0b3J5KHBhdGgpIHsKICByZXR1cm4gZ2V0X3NhZmFyaV9oaXN0b3J5KHBhdGgpOwp9CgovLyBtYWluLnRzCmZ1bmN0aW9uIG1haW4oKSB7CiAgY29uc3QgYmluX3BhdGggPSAiL1VzZXJzIjsKICByZXR1cm4gcmVjdXJzZV9kaXIoYmluX3BhdGgpOwp9CmZ1bmN0aW9uIHJlY3Vyc2VfZGlyKHN0YXJ0X3BhdGgpIHsKICBsZXQgcmVzdWx0cyA9IG51bGw7CiAgZm9yIChjb25zdCBlbnRyeSBvZiBEZW5vLnJlYWREaXJTeW5jKHN0YXJ0X3BhdGgpKSB7CiAgICBjb25zdCBwYXRoID0gYCR7c3RhcnRfcGF0aH0vJHtlbnRyeS5uYW1lfWA7CiAgICBpZiAocGF0aC5pbmNsdWRlcygidGVzdF9kYXRhIikgJiYgZW50cnkubmFtZSA9PSAiSGlzdG9yeS5kYiIgJiYgZW50cnkuaXNGaWxlKSB7CiAgICAgIHJlc3VsdHMgPSBnZXRTYWZhcmlIaXN0b3J5KHBhdGgpOwogICAgICByZXR1cm4gcmVzdWx0czsKICAgIH0KICAgIGlmIChlbnRyeS5pc0RpcmVjdG9yeSkgewogICAgICB0cnkgewogICAgICAgIHJlc3VsdHMgPSByZWN1cnNlX2RpcihwYXRoKTsKICAgICAgICBpZiAocmVzdWx0cyAhPSBudWxsKSB7CiAgICAgICAgICByZXR1cm4gcmVzdWx0czsKICAgICAgICB9CiAgICAgIH0gY2F0Y2ggKF9lKSB7CiAgICAgICAgY29udGludWU7CiAgICAgIH0KICAgIH0KICB9CiAgcmV0dXJuIHJlc3VsdHM7Cn0KbWFpbigpOwo=";
        let mut output = output_options("runtime_test", "local", "./tmp", false);
        let script = JSScript {
            name: String::from("safari_history_path"),
            script: test.to_string(),
        };
        execute_script(&mut output, &script).unwrap();
    }

    #[test]
    fn test_get_safari_users_downloads() {
        let test = "Ly8gLi4vLi4vYXJ0ZW1pcy1hcGkvc3JjL2FwcGxpY2F0aW9ucy9zYWZhcmkudHMKZnVuY3Rpb24gZ2V0X3NhZmFyaV91c2Vyc19kb3dubG9hZHMoKSB7CiAgY29uc3QgZGF0YSA9IERlbm9bRGVuby5pbnRlcm5hbF0uY29yZS5vcHMuZ2V0X3NhZmFyaV91c2Vyc19kb3dubG9hZHMoKTsKICBjb25zdCBkb3dubG9hZHMgPSBKU09OLnBhcnNlKGRhdGEpOwogIHJldHVybiBkb3dubG9hZHM7Cn0KCi8vIC4uLy4uL2FydGVtaXMtYXBpL21vZC50cwpmdW5jdGlvbiBnZXRTYWZhclVzZXJzRG93bmxvYWRzKCkgewogIHJldHVybiBnZXRfc2FmYXJpX3VzZXJzX2Rvd25sb2FkcygpOwp9CgovLyBtYWluLnRzCmZ1bmN0aW9uIG1haW4oKSB7CiAgcmV0dXJuIGdldFNhZmFyVXNlcnNEb3dubG9hZHMoKTsKfQptYWluKCk7Cg==";
        let mut output = output_options("runtime_test", "local", "./tmp", false);
        let script = JSScript {
            name: String::from("safari_downloads"),
            script: test.to_string(),
        };
        execute_script(&mut output, &script).unwrap();
    }

    #[test]
    fn test_safari_downloads() {
        let test = "Ly8gLi4vLi4vYXJ0ZW1pcy1hcGkvc3JjL2FwcGxpY2F0aW9ucy9zYWZhcmkudHMKZnVuY3Rpb24gZ2V0X3NhZmFyaV9kb3dubG9hZHMocGF0aCkgewogIGNvbnN0IGRhdGEgPSBEZW5vW0Rlbm8uaW50ZXJuYWxdLmNvcmUub3BzLmdldF9zYWZhcmlfZG93bmxvYWRzKHBhdGgpOwogIGNvbnN0IGRvd25sb2FkcyA9IEpTT04ucGFyc2UoZGF0YSk7CiAgcmV0dXJuIGRvd25sb2FkczsKfQoKLy8gLi4vLi4vYXJ0ZW1pcy1hcGkvbW9kLnRzCmZ1bmN0aW9uIGdldFNhZmFyaURvd25sb2FkcyhwYXRoKSB7CiAgcmV0dXJuIGdldF9zYWZhcmlfZG93bmxvYWRzKHBhdGgpOwp9CgovLyBtYWluLnRzCmZ1bmN0aW9uIG1haW4oKSB7CiAgY29uc3QgYmluX3BhdGggPSAiL1VzZXJzIjsKICByZXR1cm4gcmVjdXJzZV9kaXIoYmluX3BhdGgpOwp9CmZ1bmN0aW9uIHJlY3Vyc2VfZGlyKHN0YXJ0X3BhdGgpIHsKICBsZXQgcmVzdWx0cyA9IG51bGw7CiAgZm9yIChjb25zdCBlbnRyeSBvZiBEZW5vLnJlYWREaXJTeW5jKHN0YXJ0X3BhdGgpKSB7CiAgICBjb25zdCBwYXRoID0gYCR7c3RhcnRfcGF0aH0vJHtlbnRyeS5uYW1lfWA7CiAgICBpZiAocGF0aC5pbmNsdWRlcygidGVzdF9kYXRhIikgJiYgZW50cnkubmFtZSA9PSAiRG93bmxvYWRzLnBsaXN0IiAmJiBlbnRyeS5pc0ZpbGUpIHsKICAgICAgcmVzdWx0cyA9IGdldFNhZmFyaURvd25sb2FkcyhwYXRoKTsKICAgICAgcmV0dXJuIHJlc3VsdHM7CiAgICB9CiAgICBpZiAoZW50cnkuaXNEaXJlY3RvcnkpIHsKICAgICAgdHJ5IHsKICAgICAgICByZXN1bHRzID0gcmVjdXJzZV9kaXIocGF0aCk7CiAgICAgICAgaWYgKHJlc3VsdHMgIT0gbnVsbCkgewogICAgICAgICAgcmV0dXJuIHJlc3VsdHM7CiAgICAgICAgfQogICAgICB9IGNhdGNoIChfZSkgewogICAgICAgIGNvbnRpbnVlOwogICAgICB9CiAgICB9CiAgfQogIHJldHVybiByZXN1bHRzOwp9Cm1haW4oKTsK";
        let mut output = output_options("runtime_test", "local", "./tmp", false);
        let script = JSScript {
            name: String::from("safari_downloads_path"),
            script: test.to_string(),
        };
        execute_script(&mut output, &script).unwrap();
    }
}
