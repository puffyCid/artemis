use crate::artifacts::os::macos::launchd::launchdaemon::{
    grab_launchd_agents, grab_launchd_daemons,
};
use boa_engine::{Context, JsError, JsResult, JsValue, js_string};

/// Expose parsing launchd daemons to `BoaJS`
pub(crate) fn js_launchd_daemons(
    _this: &JsValue,
    _args: &[JsValue],
    context: &mut Context,
) -> JsResult<JsValue> {
    let launchd = match grab_launchd_daemons() {
        Ok(result) => result,
        Err(err) => {
            let issue = format!("Failed to get launch daemons: {err:?}");
            return Err(JsError::from_opaque(js_string!(issue).into()));
        }
    };
    let results = serde_json::to_value(&launchd).unwrap_or_default();
    let value = JsValue::from_json(&results, context)?;

    Ok(value)
}

/// Expose parsing launchd agents to `BoaJS`
pub(crate) fn js_launchd_agents(
    _this: &JsValue,
    _args: &[JsValue],
    context: &mut Context,
) -> JsResult<JsValue> {
    let launchd = match grab_launchd_agents() {
        Ok(result) => result,
        Err(err) => {
            let issue = format!("Failed to get launch daemons: {err:?}");
            return Err(JsError::from_opaque(js_string!(issue).into()));
        }
    };
    let results = serde_json::to_value(&launchd).unwrap_or_default();
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
    fn test_js_launchd_daemons_agents() {
        let test = "Ly8gaHR0cHM6Ly9yYXcuZ2l0aHVidXNlcmNvbnRlbnQuY29tL3B1ZmZ5Y2lkL2FydGVtaXMtYXBpL21hc3Rlci9zcmMvbWFjb3MvbGF1bmNoZC50cwpmdW5jdGlvbiBnZXRfbGF1bmNoZF9kYWVtb25zKCkgewogIGNvbnN0IGRhdGEgPSBqc19sYXVuY2hkX2RhZW1vbnMoKTsKICByZXR1cm4gZGF0YTsKfQpmdW5jdGlvbiBnZXRfbGF1bmNoZF9hZ2VudHMoKSB7CiAgY29uc3QgZGF0YSA9IGpzX2xhdW5jaGRfYWdlbnRzKCk7CiAgcmV0dXJuIGRhdGE7Cn0KCi8vIGh0dHBzOi8vcmF3LmdpdGh1YnVzZXJjb250ZW50LmNvbS9wdWZmeWNpZC9hcnRlbWlzLWFwaS9tYXN0ZXIvbW9kLnRzCmZ1bmN0aW9uIGdldExhdW5jaGRBZ2VudHMoKSB7CiAgcmV0dXJuIGdldF9sYXVuY2hkX2FnZW50cygpOwp9CmZ1bmN0aW9uIGdldExhdW5jaGREYWVtb25zKCkgewogIHJldHVybiBnZXRfbGF1bmNoZF9kYWVtb25zKCk7Cn0KCi8vIG1haW4udHMKZnVuY3Rpb24gbWFpbigpIHsKICBjb25zdCBhZ2VudHMgPSBnZXRMYXVuY2hkQWdlbnRzKCk7CiAgY29uc3QgZGFlbW9ucyA9IGdldExhdW5jaGREYWVtb25zKCk7CiAgcmV0dXJuIGFnZW50cy5jb25jYXQoZGFlbW9ucyk7Cn0KbWFpbigpOwo=";
        let mut output = output_options("runtime_test", "local", "./tmp", false);
        let script = JSScript {
            name: String::from("launchd_daemons"),
            script: test.to_string(),
        };
        execute_script(&mut output, &script).unwrap();
    }
}
