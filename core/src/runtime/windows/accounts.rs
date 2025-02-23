use crate::{
    artifacts::os::windows::accounts::parser::grab_users, runtime::helper::string_arg,
    structs::artifacts::os::windows::WindowsUserOptions,
};
use boa_engine::{js_string, Context, JsArgs, JsError, JsResult, JsValue};

/// Expose parsing user info to `BoaJS`
pub(crate) fn js_users_windows(
    _this: &JsValue,
    args: &[JsValue],
    context: &mut Context,
) -> JsResult<JsValue> {
    let path = if args.get_or_undefined(0).is_undefined() {
        None
    } else {
        Some(string_arg(args, &0)?)
    };
    let options = WindowsUserOptions { alt_file: path };

    let users = match grab_users(&options) {
        Ok(result) => result,
        Err(err) => {
            let issue = format!("Failed to get user info: {err:?}");
            return Err(JsError::from_opaque(js_string!(issue).into()));
        }
    };
    let results = serde_json::to_value(&users).unwrap_or_default();
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
    fn test_js_users_windows() {
        let test = "Ly8gLi4vLi4vYXJ0ZW1pcy1hcGkvc3JjL3dpbmRvd3MvdXNlcnMudHMKZnVuY3Rpb24gZ2V0X3VzZXJzX3dpbigpIHsKICB0cnkgewogIGNvbnN0IGRhdGEgPSBqc191c2Vyc193aW5kb3dzKCk7CiAgcmV0dXJuIGRhdGE7Cn0gY2F0Y2goZXJyKXtyZXR1cm4gZXJyO30KfQoKLy8gLi4vLi4vYXJ0ZW1pcy1hcGkvbW9kLnRzCmZ1bmN0aW9uIGdldFVzZXJzV2luKCkgewogIHJldHVybiBnZXRfdXNlcnNfd2luKCk7Cn0KCi8vIG1haW4udHMKZnVuY3Rpb24gbWFpbigpIHsKICBjb25zdCB1c2VycyA9IGdldFVzZXJzV2luKCk7CiAgcmV0dXJuIHVzZXJzOwp9Cm1haW4oKTsK";
        let mut output = output_options("runtime_test", "local", "./tmp", false);
        let script = JSScript {
            name: String::from("users"),
            script: test.to_string(),
        };
        execute_script(&mut output, &script).unwrap();
    }
}
