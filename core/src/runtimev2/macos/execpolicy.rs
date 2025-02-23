use crate::{
    artifacts::os::macos::execpolicy::policy::grab_execpolicy, runtimev2::helper::string_arg,
    structs::artifacts::os::macos::ExecPolicyOptions,
};
use boa_engine::{js_string, Context, JsArgs, JsError, JsResult, JsValue};

/// Expose parsing ExecPolicy to `BoaJS`
pub(crate) fn js_execpolicy(
    _this: &JsValue,
    args: &[JsValue],
    context: &mut Context,
) -> JsResult<JsValue> {
    let path = if args.get_or_undefined(0).is_undefined() {
        None
    } else {
        Some(string_arg(args, &0)?)
    };

    let options = ExecPolicyOptions { alt_file: path };

    let policy = match grab_execpolicy(&options) {
        Ok(result) => result,
        Err(err) => {
            let issue = format!("Failed to get execpolicy: {err:?}");
            return Err(JsError::from_opaque(js_string!(issue).into()));
        }
    };

    let results = serde_json::to_value(&policy).unwrap_or_default();
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
    fn test_js_execpolicy() {
        let test = "Ly8gaHR0cHM6Ly9yYXcuZ2l0aHVidXNlcmNvbnRlbnQuY29tL3B1ZmZ5Y2lkL2FydGVtaXMtYXBpL21hc3Rlci9zcmMvbWFjb3MvZXhlY3BvbGljeS50cwpmdW5jdGlvbiBnZXRfZXhlY3BvbGljeSgpIHsKICBjb25zdCBkYXRhID0ganNfZXhlY3BvbGljeSgpOwogIHJldHVybiBkYXRhOwp9CgovLyBodHRwczovL3Jhdy5naXRodWJ1c2VyY29udGVudC5jb20vcHVmZnljaWQvYXJ0ZW1pcy1hcGkvbWFzdGVyL21vZC50cwpmdW5jdGlvbiBnZXRFeGVjUG9saWN5KCkgewogIHJldHVybiBnZXRfZXhlY3BvbGljeSgpOwp9CgovLyBtYWluLnRzCmZ1bmN0aW9uIG1haW4oKSB7CiAgY29uc3QgZGF0YSA9IGdldEV4ZWNQb2xpY3koKTsKICByZXR1cm4gZGF0YTsKfQptYWluKCk7Cg==";
        let mut output = output_options("runtime_test", "local", "./tmp", true);
        let script = JSScript {
            name: String::from("execpolicy"),
            script: test.to_string(),
        };
        execute_script(&mut output, &script).unwrap();
    }
}
