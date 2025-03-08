use crate::{
    artifacts::os::macos::plist::property_list::{parse_plist_data, parse_plist_file},
    runtime::helper::{bytes_arg, string_arg},
};
use boa_engine::{Context, JsError, JsResult, JsValue, js_string};

/// Expose parsing plist file to `BoaJS`
pub(crate) fn js_plist(
    _this: &JsValue,
    args: &[JsValue],
    context: &mut Context,
) -> JsResult<JsValue> {
    let path = string_arg(args, &0)?;

    let plist = match parse_plist_file(&path) {
        Ok(result) => result,
        Err(err) => {
            let issue = format!("Failed to get plist: {err:?}");
            return Err(JsError::from_opaque(js_string!(issue).into()));
        }
    };
    let results = serde_json::to_value(&plist).unwrap_or_default();
    let value = JsValue::from_json(&results, context)?;

    Ok(value)
}

/// Expose parsing plist file  to `BoaJS`
pub(crate) fn js_plist_data(
    _this: &JsValue,
    args: &[JsValue],
    context: &mut Context,
) -> JsResult<JsValue> {
    let bytes = bytes_arg(args, &0, context)?;
    let plist_results = parse_plist_data(&bytes);
    let plist = match plist_results {
        Ok(result) => result,
        Err(err) => {
            let issue = format!("Failed to get plist: {err:?}");
            return Err(JsError::from_opaque(js_string!(issue).into()));
        }
    };
    let results = serde_json::to_value(&plist).unwrap_or_default();
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
            format: String::from("json"),
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
    fn test_js_plist() {
        // Grabs and attempts to parse all plist files under /User/*, will recurse all directories
        let test = "Ly8gaHR0cHM6Ly9yYXcuZ2l0aHVidXNlcmNvbnRlbnQuY29tL3B1ZmZ5Y2lkL2FydGVtaXMtYXBpL21hc3Rlci9zcmMvbWFjb3MvcGxpc3QudHMKZnVuY3Rpb24gZ2V0UGxpc3QocGF0aCkgewogIGlmIChwYXRoIGluc3RhbmNlb2YgVWludDhBcnJheSkgewogICAgY29uc3QgZGF0YTIgPSBqc19wbGlzdF9kYXRhKHBhdGgpOwogICAgcmV0dXJuIGRhdGEyOwogIH0KICBjb25zdCBkYXRhID0ganNfcGxpc3QocGF0aCk7CiAgcmV0dXJuIGRhdGE7Cn0KCi8vIGh0dHBzOi8vcmF3LmdpdGh1YnVzZXJjb250ZW50LmNvbS9wdWZmeWNpZC9hcnRlbWlzLWFwaS9tYXN0ZXIvc3JjL3N5c3RlbS9vdXRwdXQudHMKZnVuY3Rpb24gb3V0cHV0UmVzdWx0cyhkYXRhLCBkYXRhX25hbWUsIG91dHB1dCkgewogIGNvbnN0IG91dHB1dF9zdHJpbmcgPSBvdXRwdXQ7CiAgY29uc3Qgc3RhdHVzID0ganNfb3V0cHV0X3Jlc3VsdHMoCiAgICBkYXRhLAogICAgZGF0YV9uYW1lLAogICAgb3V0cHV0X3N0cmluZwogICk7CiAgcmV0dXJuIHN0YXR1czsKfQoKLy8gaHR0cHM6Ly9yYXcuZ2l0aHVidXNlcmNvbnRlbnQuY29tL3B1ZmZ5Y2lkL2FydGVtaXMtYXBpL21hc3Rlci9zcmMvZmlsZXN5c3RlbS9kaXJlY3RvcnkudHMKYXN5bmMgZnVuY3Rpb24gcmVhZERpcihwYXRoKSB7CiAgY29uc3QgcmVzdWx0ID0gYXdhaXQganNfcmVhZF9kaXIocGF0aCk7CiAgcmV0dXJuIHJlc3VsdDsKfQoKLy8gbWFpbi50cwphc3luYyBmdW5jdGlvbiBtYWluKCkgewogIGNvbnN0IHN0YXJ0X3BhdGggPSAiL1VzZXJzIjsKICBjb25zdCBwbGlzdF9maWxlcyA9IFtdOwogIGF3YWl0IHJlY3Vyc2VfZGlyKHBsaXN0X2ZpbGVzLCBzdGFydF9wYXRoKTsKICByZXR1cm4gcGxpc3RfZmlsZXM7Cn0KYXN5bmMgZnVuY3Rpb24gcmVjdXJzZV9kaXIocGxpc3RfZmlsZXMsIHN0YXJ0X3BhdGgpIHsKICBpZiAocGxpc3RfZmlsZXMubGVuZ3RoID4gMjApIHsKICAgIGNvbnN0IG91dCA9IHsKICAgICAgbmFtZTogImFydGVtaXNfcGxpc3QiLAogICAgICBkaXJlY3Rvcnk6ICIuL3RtcCIsCiAgICAgIGZvcm1hdDogImpzb24iIC8qIEpTT04gKi8sCiAgICAgIGNvbXByZXNzOiBmYWxzZSwKICAgICAgZW5kcG9pbnRfaWQ6ICJhbnl0aGluZy1pLXdhbnQiLAogICAgICBjb2xsZWN0aW9uX2lkOiAxLAogICAgICBvdXRwdXQ6ICJsb2NhbCIgLyogTE9DQUwgKi8KICAgIH07CiAgICBjb25zdCBzdGF0dXMgPSBvdXRwdXRSZXN1bHRzKAogICAgICBwbGlzdF9maWxlcywKICAgICAgImFydGVtaXNfaW5mbyIsCiAgICAgIG91dAogICAgKTsKICAgIGlmICghc3RhdHVzKSB7CiAgICAgIGNvbnNvbGUubG9nKCJDb3VsZCBub3Qgb3V0cHV0IHRvIGxvY2FsIGRpcmVjdG9yeSIpOwogICAgfQogICAgcGxpc3RfZmlsZXMgPSBbXTsKICB9CiAgY29uc3QgcmVzdWx0ID0gYXdhaXQgcmVhZERpcihzdGFydF9wYXRoKTsKICBpZiAocmVzdWx0IGluc3RhbmNlb2YgRXJyb3IpIHsKICAgIHJldHVybjsKICB9CiAgZm9yIChjb25zdCBlbnRyeSBvZiByZXN1bHQpIHsKICAgIGNvbnN0IHBsaXN0X3BhdGggPSBgJHtzdGFydF9wYXRofS8ke2VudHJ5LmZpbGVuYW1lfWA7CiAgICBpZiAoZW50cnkuaXNfZmlsZSAmJiBlbnRyeS5maWxlbmFtZS5lbmRzV2l0aCgicGxpc3QiKSkgewogICAgICB0cnkgewogICAgICAgIGNvbnN0IGRhdGEgPSBnZXRQbGlzdChwbGlzdF9wYXRoKTsKICAgICAgICBpZiAoZGF0YSBpbnN0YW5jZW9mIEVycm9yKSB7CiAgICAgICAgICBjb250aW51ZTsKICAgICAgICB9CiAgICAgICAgY29uc3QgcGxpc3RfaW5mbyA9IHsKICAgICAgICAgIHBsaXN0X2NvbnRlbnQ6IGRhdGEsCiAgICAgICAgICBmaWxlOiBwbGlzdF9wYXRoCiAgICAgICAgfTsKICAgICAgICBwbGlzdF9maWxlcy5wdXNoKHBsaXN0X2luZm8pOwogICAgICB9IGNhdGNoIChfZXJyKSB7CiAgICAgICAgY29udGludWU7CiAgICAgIH0KICAgICAgY29udGludWU7CiAgICB9CiAgICBpZiAoZW50cnkuaXNfZGlyZWN0b3J5KSB7CiAgICAgIGF3YWl0IHJlY3Vyc2VfZGlyKHBsaXN0X2ZpbGVzLCBwbGlzdF9wYXRoKTsKICAgIH0KICB9Cn0KbWFpbigpOwo=";
        let mut output = output_options("runtime_test", "local", "./tmp", false);

        let script = JSScript {
            name: String::from("plist_files"),
            script: test.to_string(),
        };
        execute_script(&mut output, &script).unwrap();
    }

    #[test]
    fn test_js_plist_data() {
        let test = "Ly8gaHR0cHM6Ly9yYXcuZ2l0aHVidXNlcmNvbnRlbnQuY29tL3B1ZmZ5Y2lkL2FydGVtaXMtYXBpL21hc3Rlci9zcmMvbWFjb3MvcGxpc3QudHMKZnVuY3Rpb24gZ2V0UGxpc3QocGF0aCkgewogIGlmIChwYXRoIGluc3RhbmNlb2YgVWludDhBcnJheSkgewogICAgY29uc3QgZGF0YTIgPSBqc19wbGlzdF9kYXRhKHBhdGgpOwogICAgcmV0dXJuIGRhdGEyOwogIH0KICBjb25zdCBkYXRhID0ganNfcGxpc3QocGF0aCk7CiAgcmV0dXJuIGRhdGE7Cn0KCi8vIGh0dHBzOi8vcmF3LmdpdGh1YnVzZXJjb250ZW50LmNvbS9wdWZmeWNpZC9hcnRlbWlzLWFwaS9tYXN0ZXIvc3JjL2VuY29kaW5nL2Jhc2U2NC50cwpmdW5jdGlvbiBkZWNvZGUoYjY0KSB7CiAgY29uc3QgYnl0ZXMgPSBqc19iYXNlNjRfZGVjb2RlKGI2NCk7CiAgcmV0dXJuIGJ5dGVzOwp9CgovLyBtYWluLnRzCmZ1bmN0aW9uIG1haW4oKSB7CiAgY29uc3QgZGF0YSA9ICJBQUFBQUFGdUFBSUFBQXhOWVdOcGJuUnZjMmdnU0VRQUFBQUFBQUFBQUFBQUFBQUFBQUFBQUFBQVFrUUFBZi8vLy84S2JYVnNkR2x3WVhOelpBQUFBQUFBQUFBQUFBQUFBQUFBQUFBQUFBQUFBQUFBQUFBQUFBQUFBQUFBQUFBQUFBQUFBQUFBQUFBQUFBQUFBQUFBQUFBQS8vLy8vd0FBQUFBQUFBQUFBQUFBQVAvLy8vOEFBQW9nWTNVQUFBQUFBQUFBQUFBQUFBQUFBMkpwYmdBQUFnQkVMenBNYVdKeVlYSjVPa0Z3Y0d4cFkyRjBhVzl1SUZOMWNIQnZjblE2WTI5dExtTmhibTl1YVdOaGJDNXRkV3gwYVhCaGMzTTZZbWx1T20xMWJIUnBjR0Z6YzJRQURnQVdBQW9BYlFCMUFHd0FkQUJwQUhBQVlRQnpBSE1BWkFBUEFCb0FEQUJOQUdFQVl3QnBBRzRBZEFCdkFITUFhQUFnQUVnQVJBQVNBRUpNYVdKeVlYSjVMMEZ3Y0d4cFkyRjBhVzl1SUZOMWNIQnZjblF2WTI5dExtTmhibTl1YVdOaGJDNXRkV3gwYVhCaGMzTXZZbWx1TDIxMWJIUnBjR0Z6YzJRQUV3QUJMd0QvL3dBQSI7CiAgY29uc3QgcmF3X3BsaXN0ID0gZGVjb2RlKGRhdGEpOwogIHRyeSB7CiAgIGNvbnN0IF9yZXN1bHRzID0gZ2V0UGxpc3QocmF3X3BsaXN0KTsKICB9IGNhdGNoKF9lcnIpIHt9CiAgCn0KbWFpbigpOwo=";
        let mut output = output_options("runtime_test", "local", "./tmp", false);

        let script = JSScript {
            name: String::from("plist_raw"),
            script: test.to_string(),
        };
        execute_script(&mut output, &script).unwrap();
    }
}
