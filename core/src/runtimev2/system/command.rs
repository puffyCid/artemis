use crate::{
    runtimev2::helper::{string_arg, value_arg},
    utils::strings::extract_utf8_string,
};
use boa_engine::{js_string, Context, JsError, JsResult, JsValue};
use log::warn;
use serde::Serialize;
use std::process::Command;

#[derive(Serialize)]
pub(crate) struct CommandResult {
    success: bool,
    stdout: String,
    stderr: String,
}

/// Expose command execution to the JS Runtime
pub(crate) fn js_command(
    _this: &JsValue,
    args: &[JsValue],
    context: &mut Context,
) -> JsResult<JsValue> {
    let command = string_arg(args, &0)?;
    let command_args = value_arg(args, &1, context)?;

    let mut comm_args = Vec::new();
    if let Some(arguements) = command_args.as_array() {
        for value in arguements {
            comm_args.push(value.as_str().unwrap_or_default());
        }
    }

    warn!("[runtime] Executing {command} with args: {comm_args:?}");

    let mut comm = Command::new(command);
    comm.args(comm_args);
    let out = match comm.output() {
        Ok(connect) => connect,
        Err(err) => {
            let issue = format!("Failed to execute command: {err:?}");
            return Err(JsError::from_opaque(js_string!(issue).into()));
        }
    };

    let comm_result = CommandResult {
        success: out.status.success(),
        stdout: extract_utf8_string(&out.stdout),
        stderr: extract_utf8_string(&out.stderr),
    };

    let results = serde_json::to_value(&comm_result).unwrap_or_default();
    let value = JsValue::from_json(&results, context)?;

    Ok(value)
}

#[cfg(test)]
mod tests {
    use crate::runtimev2::run::execute_script;
    use crate::{structs::artifacts::runtime::script::JSScript, structs::toml::Output};

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

    #[cfg(target_family = "unix")]
    #[test]
    fn test_js_command() {
        let test = "Ly8gLi4vLi4vYXJ0ZW1pcy1hcGkvc3JjL3N5c3RlbS9jb21tYW5kLnRzCmZ1bmN0aW9uIGV4ZWN1dGVDb21tYW5kKGNvbW1hbmQsIGFyZ3MgPSBbXSkgewogIGNvbnN0IGRhdGEgPSBqc19jb21tYW5kKGNvbW1hbmQsIGFyZ3MpOwogIGlmIChkYXRhIGluc3RhbmNlb2YgRXJyb3IpIHsKICAgIHJldHVybiBkYXRhOwogIH0KICByZXR1cm4gZGF0YTsKfQoKLy8gbWFpbi50cwpmdW5jdGlvbiBtYWluKCkgewogIGNvbnN0IGNvbW1hbmQgPSAibHMiOwogIGNvbnN0IGFyZ3MgPSBbIi1sIiwgIi1oIiwgIi1hIl07CiAgY29uc3QgcmVzdWx0cyA9IGV4ZWN1dGVDb21tYW5kKGNvbW1hbmQsIGFyZ3MpOwogIHJldHVybiByZXN1bHRzOwp9Cm1haW4oKTsK";
        let mut output = output_options("runtime_test", "local", "./tmp", false);
        let script = JSScript {
            name: String::from("command"),
            script: test.to_string(),
        };
        execute_script(&mut output, &script).unwrap();
    }

    #[cfg(target_os = "windows")]
    #[test]
    fn test_js_command() {
        let test = "Ly8gLi4vLi4vYXJ0ZW1pcy1hcGkvc3JjL3N5c3RlbS9jb21tYW5kLnRzCmZ1bmN0aW9uIGV4ZWN1dGVDb21tYW5kKGNvbW1hbmQsIGFyZ3MgPSBbXSkgewogIGNvbnN0IGRhdGEgPSBqc19jb21tYW5kKGNvbW1hbmQsIGFyZ3MpOwogIGlmIChkYXRhIGluc3RhbmNlb2YgRXJyb3IpIHsKICAgIHJldHVybiBkYXRhOwogIH0KICByZXR1cm4gZGF0YTsKfQoKLy8gbWFpbi50cwpmdW5jdGlvbiBtYWluKCkgewogIGNvbnN0IGNvbW1hbmQgPSAiZGlyIjsKICBjb25zdCBhcmdzID0gW107CiAgY29uc3QgcmVzdWx0cyA9IGV4ZWN1dGVDb21tYW5kKGNvbW1hbmQsIGFyZ3MpOwogIHJldHVybiByZXN1bHRzOwp9Cm1haW4oKTsK";
        let mut output = output_options("runtime_test", "local", "./tmp", false);
        let script = JSScript {
            name: String::from("command"),
            script: test.to_string(),
        };
        execute_script(&mut output, &script).unwrap();
    }
}
