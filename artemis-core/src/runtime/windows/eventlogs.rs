use crate::{
    artifacts::os::windows::eventlogs::parser::parse_eventlogs, runtime::error::RuntimeError,
};
use deno_core::{error::AnyError, op};
use log::error;

#[op]
/// Expose parsing a single eventlog file (evtx) to `Deno`
fn get_eventlogs(path: String) -> Result<String, AnyError> {
    if path.is_empty() {
        error!("[runtime] Empty path to eventlog file");
        return Err(RuntimeError::ExecuteScript.into());
    }
    let logs_results = parse_eventlogs(&path);
    let logs = match logs_results {
        Ok(results) => results,
        Err(err) => {
            error!("[runtime] Failed to parse eventlogs: {err:?}");
            return Err(RuntimeError::ExecuteScript.into());
        }
    };

    let results = serde_json::to_string_pretty(&logs)?;
    Ok(results)
}

#[cfg(test)]
mod tests {
    use crate::{
        runtime::deno::execute_script, structs::artifacts::runtime::script::JSScript,
        utils::artemis_toml::Output,
    };

    fn output_options(name: &str, output: &str, directory: &str, compress: bool) -> Output {
        Output {
            name: name.to_string(),
            directory: directory.to_string(),
            format: String::from("json"),
            compress,
            url: Some(String::new()),
            port: Some(0),
            api_key: Some(String::new()),
            username: Some(String::new()),
            password: Some(String::new()),
            generic_keys: Some(Vec::new()),
            endpoint_id: String::from("abcd"),
            collection_id: 0,
            output: output.to_string(),
            filter_name: None,
            filter_script: None,
        }
    }

    #[test]
    fn test_get_eventlogs() {
        let test = "Ly8gZGVuby1mbXQtaWdub3JlLWZpbGUKLy8gZGVuby1saW50LWlnbm9yZS1maWxlCi8vIFRoaXMgY29kZSB3YXMgYnVuZGxlZCB1c2luZyBgZGVubyBidW5kbGVgIGFuZCBpdCdzIG5vdCByZWNvbW1lbmRlZCB0byBlZGl0IGl0IG1hbnVhbGx5CgpmdW5jdGlvbiBnZXRfZXZlbnRsb2dzKHBhdGgpIHsKICAgIGNvbnN0IGRhdGEgPSBEZW5vW0Rlbm8uaW50ZXJuYWxdLmNvcmUub3BzLmdldF9ldmVudGxvZ3MocGF0aCk7CiAgICBjb25zdCBsb2dfYXJyYXkgPSBKU09OLnBhcnNlKGRhdGEpOwogICAgcmV0dXJuIGxvZ19hcnJheTsKfQpmdW5jdGlvbiBnZXRFdmVudExvZ3MocGF0aCkgewogICAgcmV0dXJuIGdldF9ldmVudGxvZ3MocGF0aCk7Cn0KZnVuY3Rpb24gbWFpbigpIHsKICAgIGNvbnN0IHBhdGggPSAiQzpcXFdpbmRvd3NcXFN5c3RlbTMyXFx3aW5ldnRcXExvZ3NcXFN5c3RlbS5ldnR4IjsKICAgIGNvbnN0IHJlY29yZHMgPSBnZXRFdmVudExvZ3MocGF0aCk7CiAgICBjb25zdCBzZXJ2aWNlX2luc3RhbGxzID0gW107CiAgICBmb3IgKGNvbnN0IHJlY29yZCBvZiByZWNvcmRzKXsKICAgICAgICBpZiAocmVjb3JkLmRhdGFbIkV2ZW50Il1bIlN5c3RlbSJdWyJFdmVudElEIl0gIT0gNzA0NSkgewogICAgICAgICAgICBjb250aW51ZTsKICAgICAgICB9CiAgICAgICAgc2VydmljZV9pbnN0YWxscy5wdXNoKHJlY29yZCk7CiAgICB9CiAgICByZXR1cm4gc2VydmljZV9pbnN0YWxsczsKfQptYWluKCk7Cgo=";
        let mut output = output_options("runtime_test", "local", "./tmp", false);
        let script = JSScript {
            name: String::from("service_installs"),
            script: test.to_string(),
        };
        execute_script(&mut output, &script).unwrap();
    }
}
