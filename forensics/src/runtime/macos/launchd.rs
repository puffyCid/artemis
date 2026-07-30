use crate::{
    artifacts::os::macos::launchd::launchdaemon::grab_launchd,
    structs::artifacts::os::macos::LaunchdOptions,
};
use boa_engine::{Context, JsError, JsResult, JsValue, js_string};

/// Expose parsing launchd daemons to `BoaJS`
pub(crate) fn js_launchd(
    _this: &JsValue,
    _args: &[JsValue],
    context: &mut Context,
) -> JsResult<JsValue> {
    let launchd = match grab_launchd(&LaunchdOptions { alt_file: None }) {
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
    use crate::structs::toml::{OutputConfig, OutputDestination, OutputFormat};
    use crate::{
        output::manager::OutputManager, runtime::run::execute_script,
        structs::artifacts::runtime::script::JSScript,
    };
    use std::path::PathBuf;

    fn output_options(name: &str, directory: &str, compress: bool) -> OutputManager {
        let config = OutputConfig {
            name: name.to_string(),
            directory: PathBuf::from(directory),
            format: OutputFormat::Jsonl,
            compress,
            endpoint_id: String::from("abcd"),
            destination: OutputDestination::Local,
            ..Default::default()
        };
        OutputManager::new(config).unwrap()
    }

    #[test]
    fn test_js_launchd() {
        let test = "Ly8gaHR0cHM6Ly9yYXcuZ2l0aHVidXNlcmNvbnRlbnQuY29tL3B1ZmZ5Y2lkL2FydGVtaXMtYXBpL21hc3Rlci9zcmMvbWFjb3MvbGF1bmNoZC50cwpmdW5jdGlvbiBnZXRfbGF1bmNoZF9kYWVtb25zKCkgewogIGNvbnN0IGRhdGEgPSBqc19sYXVuY2hkKCk7CiAgcmV0dXJuIGRhdGE7Cn0KCi8vIG1haW4udHMKZnVuY3Rpb24gbWFpbigpIHsKICBjb25zdCBkYWVtb25zID0gZ2V0X2xhdW5jaGRfZGFlbW9ucygpOwogIHJldHVybiBkYWVtb25zOwp9Cm1haW4oKTsK";
        let mut output = output_options("runtime_test", "./tmp", false);
        let script = JSScript {
            name: String::from("launchd_daemons"),
            script: test.to_string(),
        };
        execute_script(&mut output, &script).unwrap();
    }
}
