use crate::{
    artifacts::os::macos::execpolicy::policy::grab_execpolicy, runtime::error::RuntimeError,
};
use deno_core::{error::AnyError, op2};
use log::error;

#[op2]
#[string]
/// Expose parsing ExecPolicy to `Deno`
pub(crate) fn get_execpolicy() -> Result<String, AnyError> {
    let policy_results = grab_execpolicy();
    let policy = match policy_results {
        Ok(results) => results,
        Err(err) => {
            error!("[runtime] Failed to parse execpolicy: {err:?}");
            return Err(RuntimeError::ExecuteScript.into());
        }
    };
    let results = serde_json::to_string(&policy)?;
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
    fn test_get_execpolicy() {
        let test = "Ly8gaHR0cHM6Ly9yYXcuZ2l0aHVidXNlcmNvbnRlbnQuY29tL3B1ZmZ5Y2lkL2FydGVtaXMtYXBpL21hc3Rlci9zcmMvbWFjb3MvZXhlY3BvbGljeS50cwpmdW5jdGlvbiBnZXRfZXhlY3BvbGljeSgpIHsKICBjb25zdCBkYXRhID0gRGVuby5jb3JlLm9wcy5nZXRfZXhlY3BvbGljeSgpOwogIGNvbnN0IHBvbGljeSA9IEpTT04ucGFyc2UoZGF0YSk7CiAgcmV0dXJuIHBvbGljeTsKfQoKLy8gaHR0cHM6Ly9yYXcuZ2l0aHVidXNlcmNvbnRlbnQuY29tL3B1ZmZ5Y2lkL2FydGVtaXMtYXBpL21hc3Rlci9tb2QudHMKZnVuY3Rpb24gZ2V0RXhlY1BvbGljeSgpIHsKICByZXR1cm4gZ2V0X2V4ZWNwb2xpY3koKTsKfQoKLy8gbWFpbi50cwpmdW5jdGlvbiBtYWluKCkgewogIGNvbnN0IGRhdGEgPSBnZXRFeGVjUG9saWN5KCk7CiAgcmV0dXJuIGRhdGE7Cn0KbWFpbigpOwo=";
        let mut output = output_options("runtime_test", "local", "./tmp", true);
        let script = JSScript {
            name: String::from("execpolicy"),
            script: test.to_string(),
        };
        execute_script(&mut output, &script).unwrap();
    }
}
