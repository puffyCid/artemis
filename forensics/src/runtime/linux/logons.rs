use crate::{artifacts::os::linux::logons::parser::grab_logon_file, runtime::helper::string_arg};
use boa_engine::{Context, JsResult, JsValue};

/// Expose parsing logon file  to `BoaJS`
pub(crate) fn js_get_logon(
    _this: &JsValue,
    args: &[JsValue],
    context: &mut Context,
) -> JsResult<JsValue> {
    let path = string_arg(args, 0)?;

    let mut logons = Vec::new();
    grab_logon_file(&path, &mut logons);

    let results = serde_json::to_value(&logons).unwrap_or_default();
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
    fn test_js_get_logon() {
        let test = "Ly8gaHR0cHM6Ly9yYXcuZ2l0aHVidXNlcmNvbnRlbnQuY29tL3B1ZmZ5Y2lkL2FydGVtaXMtYXBpL21hc3Rlci9zcmMvbGludXgvbG9nb24udHMKZnVuY3Rpb24gZ2V0TG9nb24ocGF0aCkgewogIGlmIChwYXRoLmVuZHNXaXRoKCJidG1wIikgJiYgIXBhdGguZW5kc1dpdGgoInd0bXAiKSAmJiAhcGF0aC5lbmRzV2l0aCgidXRtcCIpKSB7CiAgICBjb25zb2xlLmVycm9yKGBQcm92aWRlZCBub24tbG9nb24gZmlsZSAke3BhdGh9YCk7CiAgICByZXR1cm4gW107CiAgfQogIGNvbnN0IGRhdGEgPSBqc19nZXRfbG9nb24ocGF0aCk7CiAgcmV0dXJuIGRhdGE7Cn0KCi8vIG1haW4udHMKZnVuY3Rpb24gbWFpbigpIHsKICBjb25zdCB3dG1wID0gIi92YXIvbG9nL3d0bXAiOwogIGNvbnN0IHJlc3VsdHMgPSBnZXRMb2dvbih3dG1wKTsKICByZXR1cm4gcmVzdWx0czsKfQptYWluKCk7Cg==";
        let mut output = output_options("runtime_test", "./tmp", false);

        let script = JSScript {
            name: String::from("logon"),
            script: test.to_string(),
        };
        execute_script(&mut output, &script).unwrap();
    }
}
