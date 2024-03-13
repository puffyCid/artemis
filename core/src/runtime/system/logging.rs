use deno_core::op2;
use log::{error, info, warn};

#[op2(fast)]
pub(crate) fn js_log(#[string] level: String, #[string] message: String) {
    match level.as_str() {
        "warn" => warn!("{message}"),
        "error" => error!("{message}"),
        "info" => info!("{message}"),
        _ => error!("unknown level {level}: {message}"),
    }
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

    #[test]
    fn test_js_command() {
        let test = "Ly8gLi4vLi4vYXJ0ZW1pcy1hcGkvc3JjL3N5c3RlbS9jb21tYW5kLnRzCmZ1bmN0aW9uIGV4ZWN1dGVDb21tYW5kKGNvbW1hbmQsIGFyZ3MgPSBbXSkgewogIGNvbnN0IGNvbW1fYXJncyA9IHt9OwogIGZvciAobGV0IGFyZyA9IDA7IGFyZyA8IGFyZ3MubGVuZ3RoOyBhcmcrKykgewogICAgY29tbV9hcmdzW2FyZ10gPSBhcmdzW2FyZ107CiAgfQogIGNvbnN0IGRhdGEgPSBzeXN0ZW0uZXhlY3V0ZShjb21tYW5kLCBjb21tX2FyZ3MpOwogIGlmIChkYXRhIGluc3RhbmNlb2YgRXJyb3IpIHsKICAgIHJldHVybiBkYXRhOwogIH0KICBjb25zdCByZXN1bHQgPSBKU09OLnBhcnNlKGRhdGEpOwogIHJldHVybiByZXN1bHQ7Cn0KCi8vIG1haW4udHMKZnVuY3Rpb24gbWFpbigpIHsKICBjb25zdCBjb21tYW5kID0gImxzIjsKICBjb25zdCBhcmdzID0gWyItbCIsICItaCIsICItYSJdOwogIGNvbnN0IHJlc3VsdHMgPSBleGVjdXRlQ29tbWFuZChjb21tYW5kLCBhcmdzKTsgCiAgY29uc29sZS53YXJuKCJoaSIpOwogIHJldHVybiByZXN1bHRzOwp9Cm1haW4oKTsK";
        let mut output = output_options("runtime_test", "local", "./tmp", false);
        let script = JSScript {
            name: String::from("logging"),
            script: test.to_string(),
        };
        execute_script(&mut output, &script).unwrap();
    }
}
