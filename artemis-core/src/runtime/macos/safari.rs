use crate::artifacts::applications::safari;
use deno_core::{error::AnyError, op2};

#[op2]
#[string]
/// Get `Safari` history for all users
pub(crate) fn get_safari_users_history() -> Result<String, AnyError> {
    let history = safari::history::get_safari_history()?;
    let results = serde_json::to_string(&history)?;
    Ok(results)
}

#[op2]
#[string]
/// Get `Safari` history from provided path
pub(crate) fn get_safari_history(#[string] path: String) -> Result<String, AnyError> {
    let history = safari::history::history_query(&path)?;
    let results = serde_json::to_string(&history)?;
    Ok(results)
}

#[op2]
#[string]
/// Get `Safari` downloads for all users
pub(crate) fn get_safari_users_downloads() -> Result<String, AnyError> {
    let downloads = safari::downloads::get_safari_downloads()?;
    let results = serde_json::to_string(&downloads)?;
    Ok(results)
}

#[op2]
#[string]
/// Get `Safari` downloads from provided path
pub(crate) fn get_safari_downloads(#[string] path: String) -> Result<String, AnyError> {
    let downloads = safari::downloads::downloads_query(&path)?;
    let results = serde_json::to_string(&downloads)?;
    Ok(results)
}

#[cfg(test)]
#[cfg(target_os = "macos")]
mod tests {
    use crate::{
        runtime::deno::execute_script, structs::artifacts::runtime::script::JSScript,
        structs::toml::Output,
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
        let test = "Ly8gLi4vLi4vYXJ0ZW1pcy1hcGkvc3JjL2FwcGxpY2F0aW9ucy9zYWZhcmkudHMKZnVuY3Rpb24gZ2V0X3NhZmFyaV91c2Vyc19oaXN0b3J5KCkgewogIGNvbnN0IGRhdGEgPSBEZW5vLmNvcmUub3BzLmdldF9zYWZhcmlfdXNlcnNfaGlzdG9yeSgpOwogIGNvbnN0IGhpc3RvcnkgPSBKU09OLnBhcnNlKGRhdGEpOwogIHJldHVybiBoaXN0b3J5Owp9CgovLyAuLi8uLi9hcnRlbWlzLWFwaS9tb2QudHMKZnVuY3Rpb24gZ2V0U2FmYXJpVXNlcnNIaXN0b3J5KCkgewogIHJldHVybiBnZXRfc2FmYXJpX3VzZXJzX2hpc3RvcnkoKTsKfQoKLy8gbWFpbi50cwpmdW5jdGlvbiBtYWluKCkgewogIHJldHVybiBnZXRTYWZhcmlVc2Vyc0hpc3RvcnkoKTsKfQptYWluKCk7Cg==";
        let mut output = output_options("runtime_test", "local", "./tmp", false);
        let script = JSScript {
            name: String::from("safari_history"),
            script: test.to_string(),
        };
        execute_script(&mut output, &script).unwrap();
    }

    #[test]
    fn test_get_safari_history() {
        let test = "Ly8gaHR0cHM6Ly9yYXcuZ2l0aHVidXNlcmNvbnRlbnQuY29tL3B1ZmZ5Y2lkL2FydGVtaXMtYXBpL21hc3Rlci9zcmMvbWFjb3Mvc2FmYXJpLnRzCmZ1bmN0aW9uIGdldFNhZmFyaUhpc3RvcnkocGF0aCkgewogIGNvbnN0IGRhdGEgPSBEZW5vLmNvcmUub3BzLmdldF9zYWZhcmlfaGlzdG9yeShwYXRoKTsKICBjb25zdCBoaXN0b3J5ID0gSlNPTi5wYXJzZShkYXRhKTsKICByZXR1cm4gaGlzdG9yeTsKfQoKLy8gaHR0cHM6Ly9yYXcuZ2l0aHVidXNlcmNvbnRlbnQuY29tL3B1ZmZ5Y2lkL2FydGVtaXMtYXBpL21hc3Rlci9zcmMvZmlsZXN5c3RlbS9kaXJlY3RvcnkudHMKYXN5bmMgZnVuY3Rpb24gcmVhZERpcihwYXRoKSB7CiAgY29uc3QgZGF0YSA9IEpTT04ucGFyc2UoYXdhaXQgZnMucmVhZERpcihwYXRoKSk7CiAgcmV0dXJuIGRhdGE7Cn0KCi8vIG1haW4udHMKYXN5bmMgZnVuY3Rpb24gbWFpbigpIHsKICBjb25zdCBiaW5fcGF0aCA9ICIvVXNlcnMiOwogIHJldHVybiBhd2FpdCByZWN1cnNlX2RpcihiaW5fcGF0aCk7Cn0KYXN5bmMgZnVuY3Rpb24gcmVjdXJzZV9kaXIoc3RhcnRfcGF0aCkgewogIGxldCByZXN1bHRzID0gbnVsbDsKICBmb3IgKGNvbnN0IGVudHJ5IG9mIGF3YWl0IHJlYWREaXIoc3RhcnRfcGF0aCkpIHsKICAgIGNvbnN0IHBhdGggPSBgJHtzdGFydF9wYXRofS8ke2VudHJ5LmZpbGVuYW1lfWA7CiAgICBpZiAocGF0aC5pbmNsdWRlcygidGVzdF9kYXRhIikgJiYgZW50cnkuZmlsZW5hbWUgPT0gIkhpc3RvcnkuZGIiICYmIGVudHJ5LmlzX2ZpbGUpIHsKICAgICAgcmVzdWx0cyA9IGdldFNhZmFyaUhpc3RvcnkocGF0aCk7CiAgICAgIHJldHVybiByZXN1bHRzOwogICAgfQogICAgaWYgKGVudHJ5LmlzX2RpcmVjdG9yeSkgewogICAgICB0cnkgewogICAgICAgIHJlc3VsdHMgPSBhd2FpdCByZWN1cnNlX2RpcihwYXRoKTsKICAgICAgICBpZiAocmVzdWx0cyAhPSBudWxsKSB7CiAgICAgICAgICByZXR1cm4gcmVzdWx0czsKICAgICAgICB9CiAgICAgIH0gY2F0Y2ggKF9lKSB7CiAgICAgICAgY29udGludWU7CiAgICAgIH0KICAgIH0KICB9CiAgcmV0dXJuIHJlc3VsdHM7Cn0KbWFpbigpOwo=";
        let mut output = output_options("runtime_test", "local", "./tmp", false);
        let script = JSScript {
            name: String::from("safari_history_path"),
            script: test.to_string(),
        };
        execute_script(&mut output, &script).unwrap();
    }

    #[test]
    fn test_get_safari_users_downloads() {
        let test = "Ly8gLi4vLi4vYXJ0ZW1pcy1hcGkvc3JjL2FwcGxpY2F0aW9ucy9zYWZhcmkudHMKZnVuY3Rpb24gZ2V0X3NhZmFyaV91c2Vyc19kb3dubG9hZHMoKSB7CiAgY29uc3QgZGF0YSA9IERlbm8uY29yZS5vcHMuZ2V0X3NhZmFyaV91c2Vyc19kb3dubG9hZHMoKTsKICBjb25zdCBkb3dubG9hZHMgPSBKU09OLnBhcnNlKGRhdGEpOwogIHJldHVybiBkb3dubG9hZHM7Cn0KCi8vIC4uLy4uL2FydGVtaXMtYXBpL21vZC50cwpmdW5jdGlvbiBnZXRTYWZhclVzZXJzRG93bmxvYWRzKCkgewogIHJldHVybiBnZXRfc2FmYXJpX3VzZXJzX2Rvd25sb2FkcygpOwp9CgovLyBtYWluLnRzCmZ1bmN0aW9uIG1haW4oKSB7CiAgcmV0dXJuIGdldFNhZmFyVXNlcnNEb3dubG9hZHMoKTsKfQptYWluKCk7Cg==";
        let mut output = output_options("runtime_test", "local", "./tmp", false);
        let script = JSScript {
            name: String::from("safari_downloads"),
            script: test.to_string(),
        };
        execute_script(&mut output, &script).unwrap();
    }

    #[test]
    fn test_safari_downloads() {
        let test = "Ly8gaHR0cHM6Ly9yYXcuZ2l0aHVidXNlcmNvbnRlbnQuY29tL3B1ZmZ5Y2lkL2FydGVtaXMtYXBpL21hc3Rlci9zcmMvbWFjb3Mvc2FmYXJpLnRzCmZ1bmN0aW9uIGdldFNhZmFyaURvd25sb2FkcyhwYXRoKSB7CiAgY29uc3QgZGF0YSA9IERlbm8uY29yZS5vcHMuZ2V0X3NhZmFyaV9kb3dubG9hZHMocGF0aCk7CiAgY29uc3QgZG93bmxvYWRzID0gSlNPTi5wYXJzZShkYXRhKTsKICByZXR1cm4gZG93bmxvYWRzOwp9CgovLyBodHRwczovL3Jhdy5naXRodWJ1c2VyY29udGVudC5jb20vcHVmZnljaWQvYXJ0ZW1pcy1hcGkvbWFzdGVyL3NyYy9maWxlc3lzdGVtL2RpcmVjdG9yeS50cwphc3luYyBmdW5jdGlvbiByZWFkRGlyKHBhdGgpIHsKICBjb25zdCBkYXRhID0gSlNPTi5wYXJzZShhd2FpdCBmcy5yZWFkRGlyKHBhdGgpKTsKICByZXR1cm4gZGF0YTsKfQoKLy8gbWFpbi50cwphc3luYyBmdW5jdGlvbiBtYWluKCkgewogIGNvbnN0IGJpbl9wYXRoID0gIi9Vc2VycyI7CiAgcmV0dXJuIGF3YWl0IHJlY3Vyc2VfZGlyKGJpbl9wYXRoKTsKfQphc3luYyBmdW5jdGlvbiByZWN1cnNlX2RpcihzdGFydF9wYXRoKSB7CiAgbGV0IHJlc3VsdHMgPSBudWxsOwogIGZvciAoY29uc3QgZW50cnkgb2YgYXdhaXQgcmVhZERpcihzdGFydF9wYXRoKSkgewogICAgY29uc3QgcGF0aCA9IGAke3N0YXJ0X3BhdGh9LyR7ZW50cnkuZmlsZW5hbWV9YDsKICAgIGlmIChwYXRoLmluY2x1ZGVzKCJ0ZXN0X2RhdGEiKSAmJiBlbnRyeS5maWxlbmFtZSA9PSAiRG93bmxvYWRzLnBsaXN0IiAmJiBlbnRyeS5pc19maWxlKSB7CiAgICAgIHJlc3VsdHMgPSBnZXRTYWZhcmlEb3dubG9hZHMocGF0aCk7CiAgICAgIHJldHVybiByZXN1bHRzOwogICAgfQogICAgaWYgKGVudHJ5LmlzX2RpcmVjdG9yeSkgewogICAgICB0cnkgewogICAgICAgIHJlc3VsdHMgPSBhd2FpdCByZWN1cnNlX2RpcihwYXRoKTsKICAgICAgICBpZiAocmVzdWx0cyAhPSBudWxsKSB7CiAgICAgICAgICByZXR1cm4gcmVzdWx0czsKICAgICAgICB9CiAgICAgIH0gY2F0Y2ggKF9lKSB7CiAgICAgICAgY29udGludWU7CiAgICAgIH0KICAgIH0KICB9CiAgcmV0dXJuIHJlc3VsdHM7Cn0KbWFpbigpOwo=";
        let mut output = output_options("runtime_test", "local", "./tmp", false);
        let script = JSScript {
            name: String::from("safari_downloads_path"),
            script: test.to_string(),
        };
        execute_script(&mut output, &script).unwrap();
    }
}
