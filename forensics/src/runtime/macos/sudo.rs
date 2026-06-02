use crate::{
    artifacts::os::macos::sudo::logs::grab_sudo_logs, runtime::helper::string_arg,
    structs::artifacts::os::macos::MacosSudoOptions,
};
use boa_engine::{Context, JsArgs, JsError, JsResult, JsValue, js_string};

/// Get sudo log data
pub(crate) fn js_sudologs_macos(
    _this: &JsValue,
    args: &[JsValue],
    context: &mut Context,
) -> JsResult<JsValue> {
    let path = if args.get_or_undefined(0).is_undefined() {
        None
    } else {
        Some(string_arg(args, 0)?)
    };
    let options = MacosSudoOptions {
        logarchive_path: path,
    };
    let sudo_results = grab_sudo_logs(&options);
    let sudo = match sudo_results {
        Ok(result) => result,
        Err(err) => {
            let issue = format!("Failed to get sudo logs: {err:?}");
            return Err(JsError::from_opaque(js_string!(issue).into()));
        }
    };
    let results = serde_json::to_value(&sudo).unwrap_or_default();
    let value = JsValue::from_json(&results, context)?;

    Ok(value)
}

#[cfg(test)]
mod tests {
    use crate::{
        output2::{
            config::{OutputConfig, OutputDestination, OutputFormat},
            manager::OutputManager,
        },
        runtime::run::execute_script,
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
    fn test_js_sudologs() {
        let test = "Ly8gaHR0cHM6Ly9yYXcuZ2l0aHVidXNlcmNvbnRlbnQuY29tL3B1ZmZ5Y2lkL2FydGVtaXMtYXBpL21hc3Rlci9zcmMvdW5peC9zdWRvbG9ncy50cwpmdW5jdGlvbiBnZXRNYWNvc1N1ZG9Mb2dzKCkgewogIGNvbnN0IGRhdGEgPSBqc19zdWRvbG9nc19tYWNvcygiZmFrZSIpOwogIHJldHVybiBkYXRhOwp9CgovLyBtYWluLnRzCmZ1bmN0aW9uIG1haW4oKSB7CiAgY29uc3QgZGF0YSA9IGdldE1hY29zU3Vkb0xvZ3MoKTsKICByZXR1cm4gZGF0YTsKfQptYWluKCk7Cg==";
        let mut output = output_options("runtime_test", "./tmp", false);
        let script = JSScript {
            name: String::from("sudo_script"),
            script: test.to_string(),
        };
        execute_script(&mut output, &script).unwrap();
    }
}
