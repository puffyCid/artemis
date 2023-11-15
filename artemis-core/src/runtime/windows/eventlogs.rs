use crate::{
    artifacts::os::windows::eventlogs::parser::parse_eventlogs, runtime::error::RuntimeError,
};
use deno_core::{error::AnyError, op2};
use log::error;

#[op2]
#[string]
/// Expose parsing a single eventlog file (evtx) to `Deno`
pub(crate) fn get_eventlogs(#[string] path: String) -> Result<String, AnyError> {
    if path.is_empty() {
        error!("[runtime] Empty path to eventlog file");
        return Err(RuntimeError::ExecuteScript.into());
    }
    let logs = parse_eventlogs(&path)?;

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
        let test = "Ly8gZGVuby1mbXQtaWdub3JlLWZpbGUKLy8gZGVuby1saW50LWlnbm9yZS1maWxlCi8vIFRoaXMgY29kZSB3YXMgYnVuZGxlZCB1c2luZyBgZGVubyBidW5kbGVgIGFuZCBpdCdzIG5vdCByZWNvbW1lbmRlZCB0byBlZGl0IGl0IG1hbnVhbGx5CgpmdW5jdGlvbiBnZXRfZXZlbnRsb2dzKHBhdGgpIHsKICAgIGNvbnN0IGRhdGEgPSBEZW5vLmNvcmUub3BzLmdldF9ldmVudGxvZ3MocGF0aCk7CiAgICBjb25zdCBsb2dfYXJyYXkgPSBKU09OLnBhcnNlKGRhdGEpOwogICAgcmV0dXJuIGxvZ19hcnJheTsKfQpmdW5jdGlvbiBnZXRFdmVudExvZ3MocGF0aCkgewogICAgcmV0dXJuIGdldF9ldmVudGxvZ3MocGF0aCk7Cn0KZnVuY3Rpb24gbWFpbigpIHsKICAgIGNvbnN0IHBhdGggPSAiQzpcXFdpbmRvd3NcXFN5c3RlbTMyXFx3aW5ldnRcXExvZ3NcXFN5c3RlbS5ldnR4IjsKICAgIGNvbnN0IHJlY29yZHMgPSBnZXRFdmVudExvZ3MocGF0aCk7CiAgICBjb25zdCBzZXJ2aWNlX2luc3RhbGxzID0gW107CiAgICBmb3IgKGNvbnN0IHJlY29yZCBvZiByZWNvcmRzKXsKICAgICAgICBpZiAocmVjb3JkLmRhdGFbIkV2ZW50Il1bIlN5c3RlbSJdWyJFdmVudElEIl0gIT0gNzA0NSkgewogICAgICAgICAgICBjb250aW51ZTsKICAgICAgICB9CiAgICAgICAgc2VydmljZV9pbnN0YWxscy5wdXNoKHJlY29yZCk7CiAgICB9CiAgICByZXR1cm4gc2VydmljZV9pbnN0YWxsczsKfQptYWluKCk7Cgo=";
        let mut output = output_options("runtime_test", "local", "./tmp", true);
        let script = JSScript {
            name: String::from("service_installs"),
            script: test.to_string(),
        };
        execute_script(&mut output, &script).unwrap();
    }
}
