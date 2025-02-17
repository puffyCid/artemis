use crate::{
    artifacts::os::linux::sudo::logs::grab_sudo_logs, runtime::error::RuntimeError,
    structs::artifacts::os::linux::LinuxSudoOptions,
};
use deno_core::{error::AnyError, op2};
use log::error;

#[op2]
#[string]
/// Get `Sudo log` data
pub(crate) fn get_sudologs_linux(#[string] path: String) -> Result<String, AnyError> {
    let mut options = LinuxSudoOptions { alt_path: None };

    if !path.is_empty() {
        options.alt_path = Some(path);
    }

    let sudo_results = grab_sudo_logs(&options);
    let sudo = match sudo_results {
        Ok(results) => results,
        Err(err) => {
            error!("[runtime] Failed to get sudo log data: {err:?}");
            return Err(RuntimeError::ExecuteScript.into());
        }
    };
    let results = serde_json::to_string(&sudo)?;
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
    fn test_get_sudologs() {
        let test = "Ly8gaHR0cHM6Ly9yYXcuZ2l0aHVidXNlcmNvbnRlbnQuY29tL3B1ZmZ5Y2lkL2FydGVtaXMtYXBpL21hc3Rlci9zcmMvdW5peC9zdWRvbG9ncy50cwpmdW5jdGlvbiBnZXRNYWNvc1N1ZG9Mb2dzKCkgewogIGNvbnN0IGRhdGEgPSBEZW5vLmNvcmUub3BzLmdldF9zdWRvbG9nc19saW51eCgpOwogIGNvbnN0IGxvZ19kYXRhID0gSlNPTi5wYXJzZShkYXRhKTsKICByZXR1cm4gbG9nX2RhdGE7Cn0KCi8vIG1haW4udHMKZnVuY3Rpb24gbWFpbigpIHsKICBjb25zdCBkYXRhID0gZ2V0TWFjb3NTdWRvTG9ncygpOwogIHJldHVybiBkYXRhOwp9Cm1haW4oKTsK";
        let mut output = output_options("runtime_test", "local", "./tmp", false);
        let script = JSScript {
            name: String::from("sudo_script"),
            script: test.to_string(),
        };
        execute_script(&mut output, &script).unwrap();
    }
}
