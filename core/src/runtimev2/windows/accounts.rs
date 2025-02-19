use crate::{
    artifacts::os::windows::accounts::parser::grab_users, runtimev2::helper::string_arg,
    structs::artifacts::os::windows::WindowsUserOptions,
};
use boa_engine::{js_string, Context, JsError, JsResult, JsValue};

/// Expose parsing user info to `BoaJS`
pub(crate) fn js_users_windows(
    _this: &JsValue,
    _args: &[JsValue],
    context: &mut Context,
) -> JsResult<JsValue> {
    let options = WindowsUserOptions { alt_file: None };

    let users = match grab_users(&options) {
        Ok(result) => result,
        Err(err) => {
            let issue = format!("Failed to get user info: {err:?}");
            return Err(JsError::from_opaque(js_string!(issue).into()));
        }
    };
    let results = serde_json::to_value(&users).unwrap();
    let value = JsValue::from_json(&results, context)?;

    Ok(value)
}

/// Expose parsing user info on alt drive to `BoaJS`
pub(crate) fn js_alt_users_windows(
    _this: &JsValue,
    args: &[JsValue],
    context: &mut Context,
) -> JsResult<JsValue> {
    let path = string_arg(args, &0)?;

    if path.is_empty() {
        let issue = String::from("Failed to parse user info. Need full path");
        return Err(JsError::from_opaque(js_string!(issue).into()));
    }
    // Get the first char from string (the drive letter)
    let options = WindowsUserOptions {
        alt_file: Some(path),
    };

    let users = match grab_users(&options) {
        Ok(result) => result,
        Err(err) => {
            let issue = format!("Failed to get user info at alt path: {err:?}");
            return Err(JsError::from_opaque(js_string!(issue).into()));
        }
    };
    let results = serde_json::to_value(&users).unwrap();
    let value = JsValue::from_json(&results, context)?;

    Ok(value)
}

#[cfg(test)]
mod tests {
    use crate::{
        runtimev2::run::execute_script,
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
    fn test_js_get_users() {
        let test = "Ly8gLi4vLi4vYXJ0ZW1pcy1hcGkvc3JjL3dpbmRvd3MvdXNlcnMudHMKZnVuY3Rpb24gZ2V0X3VzZXJzX3dpbigpIHsKICBjb25zdCBkYXRhID0ganNfdXNlcnNfd2luZG93cygpOwogIHJldHVybiBkYXRhOwp9CgovLyAuLi8uLi9hcnRlbWlzLWFwaS9tb2QudHMKZnVuY3Rpb24gZ2V0VXNlcnNXaW4oKSB7CiAgcmV0dXJuIGdldF91c2Vyc193aW4oKTsKfQoKLy8gbWFpbi50cwpmdW5jdGlvbiBtYWluKCkgewogIGNvbnN0IHVzZXJzID0gZ2V0VXNlcnNXaW4oKTsKICByZXR1cm4gdXNlcnM7Cn0KbWFpbigpOwo=";
        let mut output = output_options("runtime_test", "local", "./tmp", false);
        let script = JSScript {
            name: String::from("users"),
            script: test.to_string(),
        };
        execute_script(&mut output, &script).unwrap();
    }

    #[test]
    fn test_js_get_alt_users() {
        let test = "Ly8gaHR0cHM6Ly9yYXcuZ2l0aHVidXNlcmNvbnRlbnQuY29tL3B1ZmZ5Y2lkL2FydGVtaXMtYXBpL21hc3Rlci9zcmMvd2luZG93cy91c2Vycy50cwpmdW5jdGlvbiBnZXRBbHRVc2Vyc1dpbihkcml2ZSkgewogIGNvbnN0IGRhdGEgPSBqc19hbHRfdXNlcnNfd2luZG93cyhkcml2ZSk7CiAgcmV0dXJuIGRhdGE7Cn0KCi8vIG1haW4udHMKZnVuY3Rpb24gbWFpbigpIHsKICBjb25zdCB1c2VycyA9IGdldEFsdFVzZXJzV2luKCJDOlxcV2luZG93c1xcU3lzdGVtMzJcXGNvbmZpZ1xcU0FNIik7CiAgcmV0dXJuIHVzZXJzOwp9Cm1haW4oKTsK";
        let mut output = output_options("runtime_test", "local", "./tmp", false);
        let script = JSScript {
            name: String::from("users_alt"),
            script: test.to_string(),
        };
        execute_script(&mut output, &script).unwrap();
    }
}
