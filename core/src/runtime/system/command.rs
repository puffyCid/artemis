use crate::utils::strings::extract_utf8_string;
use deno_core::{error::AnyError, op2};
use log::warn;
use serde::Serialize;
use std::process::Command;

#[derive(Serialize)]
pub(crate) struct CommandResult {
    success: bool,
    stdout: String,
    stderr: String,
}

#[op2]
#[string]
/// Expose command execution to the JS Runtime
pub(crate) fn js_command(
    #[string] command: String,
    #[serde] args: Vec<String>,
) -> Result<String, AnyError> {
    let mut comm_args = Vec::new();
    for value in args {
        comm_args.push(value);
    }

    warn!("[runtime] Executing {command} with args: {comm_args:?}");

    let mut comm = Command::new(command);
    comm.args(comm_args);
    let out = comm.output()?;

    let comm_result = CommandResult {
        success: out.status.success(),
        stdout: extract_utf8_string(&out.stdout),
        stderr: extract_utf8_string(&out.stderr),
    };

    let results = serde_json::to_string(&comm_result)?;
    Ok(results)
}

#[cfg(test)]
mod tests {
    use crate::{
        runtime::deno::execute_script, structs::artifacts::runtime::script::JSScript,
        structs::toml::Output,
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

    #[cfg(target_family = "unix")]
    #[test]
    fn test_js_command() {
        let test = "Ly8gLi4vLi4vYXJ0ZW1pcy1hcGkvc3JjL3N5c3RlbS9jb21tYW5kLnRzCmZ1bmN0aW9uIGV4ZWN1dGVDb21tYW5kKGNvbW1hbmQsIGFyZ3MgPSBbXSkgewogIGNvbnN0IGRhdGEgPSBzeXN0ZW0uZXhlY3V0ZShjb21tYW5kLCBhcmdzKTsKICBpZiAoZGF0YSBpbnN0YW5jZW9mIEVycm9yKSB7CiAgICByZXR1cm4gZGF0YTsKICB9CiAgY29uc3QgcmVzdWx0ID0gSlNPTi5wYXJzZShkYXRhKTsKICByZXR1cm4gcmVzdWx0Owp9CgovLyBtYWluLnRzCmZ1bmN0aW9uIG1haW4oKSB7CiAgY29uc3QgY29tbWFuZCA9ICJscyI7CiAgY29uc3QgYXJncyA9IFsiLWwiLCAiLWgiLCAiLWEiXTsKICBjb25zdCByZXN1bHRzID0gZXhlY3V0ZUNvbW1hbmQoY29tbWFuZCwgYXJncyk7CiAgcmV0dXJuIHJlc3VsdHM7Cn0KbWFpbigpOwo=";
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
        let test = "Ly8gLi4vLi4vYXJ0ZW1pcy1hcGkvc3JjL3N5c3RlbS9jb21tYW5kLnRzCmZ1bmN0aW9uIGV4ZWN1dGVDb21tYW5kKGNvbW1hbmQsIGFyZ3MgPSBbXSkgewogIGNvbnN0IGRhdGEgPSBzeXN0ZW0uZXhlY3V0ZShjb21tYW5kLCBhcmdzKTsKICBpZiAoZGF0YSBpbnN0YW5jZW9mIEVycm9yKSB7CiAgICByZXR1cm4gZGF0YTsKICB9CiAgY29uc3QgcmVzdWx0ID0gSlNPTi5wYXJzZShkYXRhKTsKICByZXR1cm4gcmVzdWx0Owp9CgovLyBtYWluLnRzCmZ1bmN0aW9uIG1haW4oKSB7CiAgY29uc3QgY29tbWFuZCA9ICJkaXIiOwogIGNvbnN0IGFyZ3MgPSBbXTsKICBjb25zdCByZXN1bHRzID0gZXhlY3V0ZUNvbW1hbmQoY29tbWFuZCwgYXJncyk7CiAgcmV0dXJuIHJlc3VsdHM7Cn0KbWFpbigpOwo=";
        let mut output = output_options("runtime_test", "local", "./tmp", false);
        let script = JSScript {
            name: String::from("command"),
            script: test.to_string(),
        };
        execute_script(&mut output, &script).unwrap();
    }
}
