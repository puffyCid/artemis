use crate::{
    artifacts::applications::safari::{
        downloads::{downloads_query, get_safari_downloads},
        history::{get_safari_history, history_query},
    },
    runtime::helper::string_arg,
};
use boa_engine::{Context, JsError, JsResult, JsValue, js_string};

/// Get `Safari` history for all users
pub(crate) fn js_safari_users_history(
    _this: &JsValue,
    _args: &[JsValue],
    context: &mut Context,
) -> JsResult<JsValue> {
    let history = match get_safari_history() {
        Ok(result) => result,
        Err(err) => {
            let issue = format!("Failed to get safari history: {err:?}");
            return Err(JsError::from_opaque(js_string!(issue).into()));
        }
    };

    let results = serde_json::to_value(&history).unwrap_or_default();
    let value = JsValue::from_json(&results, context)?;

    Ok(value)
}

/// Get `Safari` history from provided path
pub(crate) fn js_safari_history(
    _this: &JsValue,
    args: &[JsValue],
    context: &mut Context,
) -> JsResult<JsValue> {
    let path = string_arg(args, 0)?;

    let history = match history_query(&path) {
        Ok(result) => result,
        Err(err) => {
            let issue = format!("Failed to get safari history: {err:?}");
            return Err(JsError::from_opaque(js_string!(issue).into()));
        }
    };

    let results = serde_json::to_value(&history).unwrap_or_default();
    let value = JsValue::from_json(&results, context)?;

    Ok(value)
}

/// Get `Safari` downloads for all users
pub(crate) fn js_safari_users_downloads(
    _this: &JsValue,
    _args: &[JsValue],
    context: &mut Context,
) -> JsResult<JsValue> {
    let downloads = match get_safari_downloads() {
        Ok(result) => result,
        Err(err) => {
            let issue = format!("Failed to get safari downloads: {err:?}");
            return Err(JsError::from_opaque(js_string!(issue).into()));
        }
    };

    let results = serde_json::to_value(&downloads).unwrap_or_default();
    let value = JsValue::from_json(&results, context)?;

    Ok(value)
}

/// Get `Safari` downloads from provided path
pub(crate) fn js_safari_downloads(
    _this: &JsValue,
    args: &[JsValue],
    context: &mut Context,
) -> JsResult<JsValue> {
    let path = string_arg(args, 0)?;

    let downloads = match downloads_query(&path) {
        Ok(result) => result,
        Err(err) => {
            let issue = format!("Failed to get safari downloads: {err:?}");
            return Err(JsError::from_opaque(js_string!(issue).into()));
        }
    };

    let results = serde_json::to_value(&downloads).unwrap_or_default();
    let value = JsValue::from_json(&results, context)?;

    Ok(value)
}

#[cfg(test)]
mod tests {
    use crate::{
        runtime::run::execute_script,
        structs::{artifacts::runtime::script::JSScript, toml::Output},
    };

    fn output_options(name: &str, output: &str, directory: &str, compress: bool) -> Output {
        Output {
            name: name.to_string(),
            directory: directory.to_string(),
            format: String::from("jsonl"),
            compress,
            timeline: false,
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

    #[tokio::test]
    async fn test_js_safari_users_history() {
        let test = "Ly8gLi4vLi4vYXJ0ZW1pcy1hcGkvc3JjL2FwcGxpY2F0aW9ucy9zYWZhcmkudHMKZnVuY3Rpb24gZ2V0X3NhZmFyaV91c2Vyc19oaXN0b3J5KCkgewogIGNvbnN0IGRhdGEgPSBqc19zYWZhcmlfdXNlcnNfaGlzdG9yeSgpOwogIHJldHVybiBkYXRhOwp9CgovLyAuLi8uLi9hcnRlbWlzLWFwaS9tb2QudHMKZnVuY3Rpb24gZ2V0U2FmYXJpVXNlcnNIaXN0b3J5KCkgewogIHJldHVybiBnZXRfc2FmYXJpX3VzZXJzX2hpc3RvcnkoKTsKfQoKLy8gbWFpbi50cwpmdW5jdGlvbiBtYWluKCkgewogIHJldHVybiBnZXRTYWZhcmlVc2Vyc0hpc3RvcnkoKTsKfQptYWluKCk7Cg==";
        let mut output = output_options("runtime_test", "local", "./tmp", false);
        let script = JSScript {
            name: String::from("safari_history"),
            script: test.to_string(),
        };
        execute_script(&mut output, &script).await.unwrap();
    }

    #[tokio::test]
    async fn test_js_safari_history() {
        let test = "Ly8gaHR0cHM6Ly9yYXcuZ2l0aHVidXNlcmNvbnRlbnQuY29tL3B1ZmZ5Y2lkL2FydGVtaXMtYXBpL21hc3Rlci9zcmMvbWFjb3Mvc2FmYXJpLnRzCmZ1bmN0aW9uIGdldFNhZmFyaUhpc3RvcnkocGF0aCkgewogIGNvbnN0IGRhdGEgPSBqc19zYWZhcmlfaGlzdG9yeShwYXRoKTsKICByZXR1cm4gZGF0YTsKfQoKLy8gaHR0cHM6Ly9yYXcuZ2l0aHVidXNlcmNvbnRlbnQuY29tL3B1ZmZ5Y2lkL2FydGVtaXMtYXBpL21hc3Rlci9zcmMvZmlsZXN5c3RlbS9kaXJlY3RvcnkudHMKYXN5bmMgZnVuY3Rpb24gcmVhZERpcihwYXRoKSB7CiAgY29uc3QgZGF0YSA9IGF3YWl0IGpzX3JlYWRfZGlyKHBhdGgpOwogIHJldHVybiBkYXRhOwp9CgovLyBtYWluLnRzCmFzeW5jIGZ1bmN0aW9uIG1haW4oKSB7CiAgY29uc3QgYmluX3BhdGggPSAiL1VzZXJzIjsKICByZXR1cm4gYXdhaXQgcmVjdXJzZV9kaXIoYmluX3BhdGgpOwp9CmFzeW5jIGZ1bmN0aW9uIHJlY3Vyc2VfZGlyKHN0YXJ0X3BhdGgpIHsKICBsZXQgcmVzdWx0cyA9IG51bGw7CiAgZm9yIChjb25zdCBlbnRyeSBvZiBhd2FpdCByZWFkRGlyKHN0YXJ0X3BhdGgpKSB7CiAgICBjb25zdCBwYXRoID0gYCR7c3RhcnRfcGF0aH0vJHtlbnRyeS5maWxlbmFtZX1gOwogICAgaWYgKHBhdGguaW5jbHVkZXMoInRlc3RfZGF0YSIpICYmIGVudHJ5LmZpbGVuYW1lID09ICJIaXN0b3J5LmRiIiAmJiBlbnRyeS5pc19maWxlKSB7CiAgICAgIHJlc3VsdHMgPSBnZXRTYWZhcmlIaXN0b3J5KHBhdGgpOwogICAgICByZXR1cm4gcmVzdWx0czsKICAgIH0KICAgIGlmIChlbnRyeS5pc19kaXJlY3RvcnkpIHsKICAgICAgdHJ5IHsKICAgICAgICByZXN1bHRzID0gYXdhaXQgcmVjdXJzZV9kaXIocGF0aCk7CiAgICAgICAgaWYgKHJlc3VsdHMgIT0gbnVsbCkgewogICAgICAgICAgcmV0dXJuIHJlc3VsdHM7CiAgICAgICAgfQogICAgICB9IGNhdGNoIChfZSkgewogICAgICAgIGNvbnRpbnVlOwogICAgICB9CiAgICB9CiAgfQogIHJldHVybiByZXN1bHRzOwp9Cm1haW4oKTsK";
        let mut output = output_options("runtime_test", "local", "./tmp", false);
        let script = JSScript {
            name: String::from("safari_history_path"),
            script: test.to_string(),
        };
        execute_script(&mut output, &script).await.unwrap();
    }

    #[tokio::test]
    async fn test_js_safari_users_downloads() {
        let test = "Ly8gLi4vLi4vYXJ0ZW1pcy1hcGkvc3JjL2FwcGxpY2F0aW9ucy9zYWZhcmkudHMKZnVuY3Rpb24gZ2V0X3NhZmFyaV91c2Vyc19kb3dubG9hZHMoKSB7CiAgY29uc3QgZGF0YSA9IGpzX3NhZmFyaV91c2Vyc19kb3dubG9hZHMoKTsKICByZXR1cm4gZGF0YTsKfQoKLy8gLi4vLi4vYXJ0ZW1pcy1hcGkvbW9kLnRzCmZ1bmN0aW9uIGdldFNhZmFyVXNlcnNEb3dubG9hZHMoKSB7CiAgcmV0dXJuIGdldF9zYWZhcmlfdXNlcnNfZG93bmxvYWRzKCk7Cn0KCi8vIG1haW4udHMKZnVuY3Rpb24gbWFpbigpIHsKICByZXR1cm4gZ2V0U2FmYXJVc2Vyc0Rvd25sb2FkcygpOwp9Cm1haW4oKTsK";
        let mut output = output_options("runtime_test", "local", "./tmp", false);
        let script = JSScript {
            name: String::from("safari_downloads"),
            script: test.to_string(),
        };
        execute_script(&mut output, &script).await.unwrap();
    }

    #[tokio::test]
    async fn test_js_safari_downloads() {
        let test = "Ly8gaHR0cHM6Ly9yYXcuZ2l0aHVidXNlcmNvbnRlbnQuY29tL3B1ZmZ5Y2lkL2FydGVtaXMtYXBpL21hc3Rlci9zcmMvbWFjb3Mvc2FmYXJpLnRzCmZ1bmN0aW9uIGdldFNhZmFyaURvd25sb2FkcyhwYXRoKSB7CiAgY29uc3QgZGF0YSA9IGpzX3NhZmFyaV9kb3dubG9hZHMocGF0aCk7CiAgcmV0dXJuIGRhdGE7Cn0KCi8vIGh0dHBzOi8vcmF3LmdpdGh1YnVzZXJjb250ZW50LmNvbS9wdWZmeWNpZC9hcnRlbWlzLWFwaS9tYXN0ZXIvc3JjL2ZpbGVzeXN0ZW0vZGlyZWN0b3J5LnRzCmFzeW5jIGZ1bmN0aW9uIHJlYWREaXIocGF0aCkgewogIGNvbnN0IGRhdGEgPSBhd2FpdCBqc19yZWFkX2RpcihwYXRoKTsKICByZXR1cm4gZGF0YTsKfQoKLy8gbWFpbi50cwphc3luYyBmdW5jdGlvbiBtYWluKCkgewogIGNvbnN0IGJpbl9wYXRoID0gIi9Vc2VycyI7CiAgcmV0dXJuIGF3YWl0IHJlY3Vyc2VfZGlyKGJpbl9wYXRoKTsKfQphc3luYyBmdW5jdGlvbiByZWN1cnNlX2RpcihzdGFydF9wYXRoKSB7CiAgbGV0IHJlc3VsdHMgPSBudWxsOwogIGZvciAoY29uc3QgZW50cnkgb2YgYXdhaXQgcmVhZERpcihzdGFydF9wYXRoKSkgewogICAgY29uc3QgcGF0aCA9IGAke3N0YXJ0X3BhdGh9LyR7ZW50cnkuZmlsZW5hbWV9YDsKICAgIGlmIChwYXRoLmluY2x1ZGVzKCJ0ZXN0X2RhdGEiKSAmJiBlbnRyeS5maWxlbmFtZSA9PSAiRG93bmxvYWRzLnBsaXN0IiAmJiBlbnRyeS5pc19maWxlKSB7CiAgICAgIHJlc3VsdHMgPSBnZXRTYWZhcmlEb3dubG9hZHMocGF0aCk7CiAgICAgIHJldHVybiByZXN1bHRzOwogICAgfQogICAgaWYgKGVudHJ5LmlzX2RpcmVjdG9yeSkgewogICAgICB0cnkgewogICAgICAgIHJlc3VsdHMgPSBhd2FpdCByZWN1cnNlX2RpcihwYXRoKTsKICAgICAgICBpZiAocmVzdWx0cyAhPSBudWxsKSB7CiAgICAgICAgICByZXR1cm4gcmVzdWx0czsKICAgICAgICB9CiAgICAgIH0gY2F0Y2ggKF9lKSB7CiAgICAgICAgY29udGludWU7CiAgICAgIH0KICAgIH0KICB9CiAgcmV0dXJuIHJlc3VsdHM7Cn0KbWFpbigpOwo=";
        let mut output = output_options("runtime_test", "local", "./tmp", false);
        let script = JSScript {
            name: String::from("safari_downloads_path"),
            script: test.to_string(),
        };
        execute_script(&mut output, &script).await.unwrap();
    }
}
