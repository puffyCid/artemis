use crate::{
    artifacts::os::macos::launchd::launchdaemon::{grab_launchd_agents, grab_launchd_daemons},
    runtime::error::RuntimeError,
};
use deno_core::{error::AnyError, op};
use log::error;

#[op]
/// Expose parsing launchd daemons to `Deno`
fn get_launchd_daemons() -> Result<String, AnyError> {
    let launchd_results = grab_launchd_daemons();
    let launchd = match launchd_results {
        Ok(results) => results,
        Err(err) => {
            error!("[runtime] Failed to get launchd daemons: {err:?}");
            return Err(RuntimeError::ExecuteScript.into());
        }
    };
    let results = serde_json::to_string_pretty(&launchd)?;
    Ok(results)
}

#[op]
/// Expose parsing launchd agents to `Deno`
fn get_launchd_agents() -> Result<String, AnyError> {
    let launchd_results = grab_launchd_agents();
    let launchd = match launchd_results {
        Ok(results) => results,
        Err(err) => {
            error!("[runtime] Failed to get launchd agents: {err:?}");
            return Err(RuntimeError::ExecuteScript.into());
        }
    };
    let results = serde_json::to_string_pretty(&launchd)?;
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
            format: String::from("jsonl"),
            compress,
            // url: Some(String::new()),
            // port: Some(0),
            // api_key: Some(String::new()),
            // username: Some(String::new()),
            // password: Some(String::new()),
            // generic_keys: Some(Vec::new()),
            endpoint_id: String::from("abcd"),
            collection_id: 0,
            output: output.to_string(),
            filter_name: Some(String::new()),
            filter_script: Some(String::new()),
        }
    }

    #[test]
    fn test_get_launchd_daemons() {
        let test = "Y29uc29sZS5sb2coRGVub1tEZW5vLmludGVybmFsXS5jb3JlLm9wcy5nZXRfbGF1bmNoZF9kYWVtb25zKCkpOwo=";
        let mut output = output_options("runtime_test", "local", "./tmp", false);
        let script = JSScript {
            name: String::from("launchd_daemons"),
            script: test.to_string(),
        };
        execute_script(&mut output, &script).unwrap();
    }

    #[test]
    fn test_get_launchd_agents() {
        let test =
            "Y29uc29sZS5sb2coRGVub1tEZW5vLmludGVybmFsXS5jb3JlLm9wcy5nZXRfbGF1bmNoZF9hZ2VudHMoKSk=";
        let mut output = output_options("runtime_test", "local", "./tmp", false);
        let script = JSScript {
            name: String::from("launchd_agents"),
            script: test.to_string(),
        };
        execute_script(&mut output, &script).unwrap();
    }

    #[test]
    fn test_get_launchd_daemons_in_opt() {
        let test = "ZnVuY3Rpb24gbWFpbigpIHsKICBjb25zdCBkYXRhID0gRGVub1tEZW5vLmludGVybmFsXS5jb3JlLm9wcy5nZXRfbGF1bmNoZF9kYWVtb25zKCk7CiAgLy8gSXRzIGFuIGFycmF5IG9mIGpzb24gb2JqZWN0cwogIGNvbnN0IGxhdW5jaGRfYXJyYXkgPSBKU09OLnBhcnNlKGRhdGEpOwoKICBjb25zdCBsYXVuY2hkX29wdCA9IFtdOwogIGZvciAoY29uc3QgZW50cnkgb2YgbGF1bmNoZF9hcnJheSkgewogICAgaWYgKGVudHJ5WyJsYXVuY2hkX2RhdGEiXVsiUHJvZ3JhbUFyZ3VtZW50cyJdID09PSB1bmRlZmluZWQpIHsKICAgICAgY29udGludWU7CiAgICB9CgogICAgZm9yIChjb25zdCBrZXkgaW4gZW50cnlbImxhdW5jaGRfZGF0YSJdWyJQcm9ncmFtQXJndW1lbnRzIl0pIHsKICAgICAgaWYgKGVudHJ5WyJsYXVuY2hkX2RhdGEiXVsiUHJvZ3JhbUFyZ3VtZW50cyJdW2tleV0uaW5jbHVkZXMoIm9wdCIpKSB7CiAgICAgICAgbGF1bmNoZF9vcHQucHVzaChlbnRyeSk7CiAgICAgICAgYnJlYWs7CiAgICAgIH0KICAgIH0KICB9CiAgcmV0dXJuIGxhdW5jaGRfb3B0Owp9CgptYWluKCk7Cg==";
        let mut output = output_options("runtime_test", "local", "./tmp", false);
        let script = JSScript {
            name: String::from("launchd"),
            script: test.to_string(),
        };
        execute_script(&mut output, &script).unwrap();
    }
}
