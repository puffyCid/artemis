use crate::{
    artifacts::os::windows::eventlogs::parser::parse_eventlogs, runtime::error::RuntimeError,
};
use deno_core::{error::AnyError, op2};
use log::error;

#[op2]
#[string]
/// Expose parsing a single eventlog file (evtx) to `Deno`
pub(crate) fn get_eventlogs(
    #[string] path: String,
    offset: u32,
    limit: u32,
    include_templates: bool,
    #[string] template_file: String,
) -> Result<String, AnyError> {
    if path.is_empty() {
        error!("[runtime] Empty path to eventlog file");
        return Err(RuntimeError::ExecuteScript.into());
    }

    let temp_option = if template_file.is_empty() {
        None
    } else {
        Some(template_file)
    };

    let logs = parse_eventlogs(
        &path,
        &(offset as usize),
        &(limit as usize),
        &include_templates,
        &temp_option,
    )?;

    let results = serde_json::to_string(&logs)?;
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

    #[test]
    fn test_get_eventlogs() {
        let test = "Ly8gLi4vLi4vUHJvamVjdHMvYXJ0ZW1pcy1hcGkvc3JjL3V0aWxzL2Vycm9yLnRzCnZhciBFcnJvckJhc2UgPSBjbGFzcyBleHRlbmRzIEVycm9yIHsKICBjb25zdHJ1Y3RvcihuYW1lLCBtZXNzYWdlKSB7CiAgICBzdXBlcigpOwogICAgdGhpcy5uYW1lID0gbmFtZTsKICAgIHRoaXMubWVzc2FnZSA9IG1lc3NhZ2U7CiAgfQp9OwoKLy8gLi4vLi4vUHJvamVjdHMvYXJ0ZW1pcy1hcGkvc3JjL3dpbmRvd3MvZXJyb3JzLnRzCnZhciBXaW5kb3dzRXJyb3IgPSBjbGFzcyBleHRlbmRzIEVycm9yQmFzZSB7Cn07CgovLyAuLi8uLi9Qcm9qZWN0cy9hcnRlbWlzLWFwaS9zcmMvc3lzdGVtL3N5c3RlbWluZm8udHMKZnVuY3Rpb24gcGxhdGZvcm0oKSB7CiAgY29uc3QgZGF0YSA9IHN5c3RlbS5wbGF0Zm9ybSgpOwogIHJldHVybiBkYXRhOwp9CgovLyAuLi8uLi9Qcm9qZWN0cy9hcnRlbWlzLWFwaS9zcmMvd2luZG93cy9ldmVudGxvZ3MudHMKZnVuY3Rpb24gZ2V0RXZlbnRsb2dzKHBhdGgsIG9mZnNldCwgbGltaXQsIGluY2x1ZGVfdGVtcGxhdGVzID0gZmFsc2UsIHRlbXBsYXRlX2ZpbGUgPSAiIikgewogIGlmIChpbmNsdWRlX3RlbXBsYXRlcyAmJiBwbGF0Zm9ybSgpICE9ICJXaW5kb3dzIiAmJiB0ZW1wbGF0ZV9maWxlID09ICIiKSB7CiAgICByZXR1cm4gbmV3IFdpbmRvd3NFcnJvcigKICAgICAgIkVWRU5UTE9HIiwKICAgICAgYGNhbm5vdCBpbmNsdWRlIHRlbXBsYXRlIHN0cmluZ3Mgb24gbm9uLVdpbmRvd3MgcGxhdGZvcm0gd2l0aG91dCBhIHRlbXBsYXRlIGZpbGVgCiAgICApOwogIH0KICB0cnkgewogICAgY29uc3QgcmVzdWx0cyA9IERlbm8uY29yZS5vcHMuZ2V0X2V2ZW50bG9ncygKICAgICAgcGF0aCwKICAgICAgb2Zmc2V0LAogICAgICBsaW1pdCwKICAgICAgaW5jbHVkZV90ZW1wbGF0ZXMsCiAgICAgIHRlbXBsYXRlX2ZpbGUKICAgICk7CiAgICBjb25zdCBkYXRhID0gSlNPTi5wYXJzZShyZXN1bHRzKTsKICAgIHJldHVybiBkYXRhOwogIH0gY2F0Y2ggKGVycikgewogICAgcmV0dXJuIG5ldyBXaW5kb3dzRXJyb3IoCiAgICAgICJFVkVOVExPRyIsCiAgICAgIGBmYWlsZWQgdG8gcGFyc2UgZXZlbnRsb2cgJHtwYXRofTogJHtlcnJ9YAogICAgKTsKICB9Cn0KCi8vIG1haW4udHMKZnVuY3Rpb24gbWFpbigpIHsKICBjb25zdCBwYXRoID0gIkM6XFxXaW5kb3dzXFxTeXN0ZW0zMlxcd2luZXZ0XFxMb2dzXFxBcHBsaWNhdGlvbi5ldnR4IjsKICBjb25zdCBsb2dzID0gZ2V0RXZlbnRsb2dzKAogICAgcGF0aCwKICAgIDEwLAogICAgMTAsCiAgICB0cnVlCiAgKTsKICBpZiAobG9ncyBpbnN0YW5jZW9mIFdpbmRvd3NFcnJvcikgewogICAgY29uc29sZS5lcnJvcihsb2dzKTsKICAgIHJldHVybjsKICB9CiAgY29uc3QgbWVzc2FnZXMgPSBsb2dzWzBdOwogIGNvbnN0IHJhdyA9IGxvZ3NbMV07CiAgY29uc29sZS5sb2cobWVzc2FnZXMpOwogIGNvbnNvbGUubG9nKHJhdyk7Cn0KbWFpbigpOwo=";
        let mut output = output_options("runtime_test", "local", "./tmp", true);
        let script = JSScript {
            name: String::from("service_installs"),
            script: test.to_string(),
        };
        execute_script(&mut output, &script).unwrap();
    }
}
