use crate::{
    artifacts::os::windows::shortcuts::parser::grab_lnk_file, runtime::helper::string_arg,
};
use boa_engine::{js_string, Context, JsError, JsResult, JsValue};

pub(crate) fn js_lnk(
    _this: &JsValue,
    args: &[JsValue],
    context: &mut Context,
) -> JsResult<JsValue> {
    let path = string_arg(args, &0)?;

    let lnk = match grab_lnk_file(&path) {
        Ok(result) => result,
        Err(err) => {
            let issue = format!("Failed to get shortcut {path}: {err:?}");
            return Err(JsError::from_opaque(js_string!(issue).into()));
        }
    };

    let results = serde_json::to_value(&lnk).unwrap_or_default();
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
            filter_name: None,
            filter_script: None,
            logging: None,
        }
    }

    #[test]
    fn test_js_lnk() {
        let test = "Ly8gaHR0cHM6Ly9yYXcuZ2l0aHVidXNlcmNvbnRlbnQuY29tL3B1ZmZ5Y2lkL2FydGVtaXMtYXBpL21hc3Rlci9zcmMvd2luZG93cy9zaG9ydGN1dHMudHMKZnVuY3Rpb24gZ2V0TG5rRmlsZShwYXRoKSB7CiAgY29uc3QgZGF0YSA9IGpzX2xuayhwYXRoKTsKICByZXR1cm4gZGF0YTsKfQoKLy8gaHR0cHM6Ly9yYXcuZ2l0aHVidXNlcmNvbnRlbnQuY29tL3B1ZmZ5Y2lkL2FydGVtaXMtYXBpL21hc3Rlci9zcmMvZW52aXJvbm1lbnQvZW52LnRzCmZ1bmN0aW9uIGdldEVudlZhbHVlKGtleSkgewogIGNvbnN0IGRhdGEgPSBqc19lbnZfdmFsdWUoa2V5KTsKICByZXR1cm4gZGF0YTsKfQoKLy8gaHR0cHM6Ly9yYXcuZ2l0aHVidXNlcmNvbnRlbnQuY29tL3B1ZmZ5Y2lkL2FydGVtaXMtYXBpL21hc3Rlci9zcmMvZmlsZXN5c3RlbS9kaXJlY3RvcnkudHMKYXN5bmMgZnVuY3Rpb24gcmVhZERpcihwYXRoKSB7CiAgY29uc3QgZGF0YSA9IGF3YWl0IGpzX3JlYWRfZGlyKHBhdGgpOwogIHJldHVybiBkYXRhOwp9CgovLyBtYWluLnRzCmFzeW5jIGZ1bmN0aW9uIG1haW4oKSB7CiAgY29uc3QgZHJpdmUgPSBnZXRFbnZWYWx1ZSgiU3lzdGVtRHJpdmUiKTsKICBpZiAoZHJpdmUgPT09ICIiKSB7CiAgICByZXR1cm4gW107CiAgfQogIGNvbnN0IHVzZXJzID0gYCR7ZHJpdmV9XFxVc2Vyc2A7CiAgY29uc3QgcmVjZW50X2ZpbGVzID0gW107CiAgY29uc3QgcmVzdWx0cyA9IGF3YWl0IHJlYWREaXIodXNlcnMpCiAgZm9yIChjb25zdCBlbnRyeSBvZiByZXN1bHRzKSB7CiAgICB0cnkgewogICAgICBjb25zdCBwYXRoID0gYCR7dXNlcnN9XFwke2VudHJ5LmZpbGVuYW1lfVxcQXBwRGF0YVxcUm9hbWluZ1xcTWljcm9zb2Z0XFxXaW5kb3dzXFxSZWNlbnRgOwogICAgICBjb25zdCByZXN1bHRzMiA9IGF3YWl0IHJlYWREaXIocGF0aCk7CiAgICAgIGZvciAoY29uc3QgZW50cnkyIG9mIHJlc3VsdHMyKSB7CiAgICAgICAgaWYgKCFlbnRyeTIuZmlsZW5hbWUuZW5kc1dpdGgoImxuayIpKSB7CiAgICAgICAgICBjb250aW51ZTsKICAgICAgICB9CiAgICAgICAgY29uc3QgbG5rX2ZpbGUgPSBgJHtwYXRofVxcJHtlbnRyeTIuZmlsZW5hbWV9YDsKICAgICAgICBjb25zdCBsbmsgPSBnZXRMbmtGaWxlKGxua19maWxlKTsKICAgICAgICByZWNlbnRfZmlsZXMucHVzaChsbmspOwogICAgICB9CiAgICB9IGNhdGNoIChfZXJyb3IpIHsKICAgICAgY29udGludWU7CiAgICB9CiAgfQogIHJldHVybiByZWNlbnRfZmlsZXM7Cn0KbWFpbigpOwo=";
        let mut output = output_options("runtime_test", "local", "./tmp", false);
        let script = JSScript {
            name: String::from("recent_files"),
            script: test.to_string(),
        };
        execute_script(&mut output, &script).unwrap();
    }
}
