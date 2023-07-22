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
    fn test_get_launchd_daemons() {
        let test = "Ly8gaHR0cHM6Ly9yYXcuZ2l0aHVidXNlcmNvbnRlbnQuY29tL3B1ZmZ5Y2lkL2FydGVtaXMtYXBpL21hc3Rlci9zcmMvbWFjb3MvbGF1bmNoZC50cwpmdW5jdGlvbiBnZXRfbGF1bmNoZF9kYWVtb25zKCkgewogIGNvbnN0IGRhdGEgPSBEZW5vLmNvcmUub3BzLmdldF9sYXVuY2hkX2RhZW1vbnMoKTsKICBjb25zdCBsYXVuY2hkX2FycmF5ID0gSlNPTi5wYXJzZShkYXRhKTsKICByZXR1cm4gbGF1bmNoZF9hcnJheTsKfQpmdW5jdGlvbiBnZXRfbGF1bmNoZF9hZ2VudHMoKSB7CiAgY29uc3QgZGF0YSA9IERlbm8uY29yZS5vcHMuZ2V0X2xhdW5jaGRfYWdlbnRzKCk7CiAgY29uc3QgbGF1bmNoZF9hcnJheSA9IEpTT04ucGFyc2UoZGF0YSk7CiAgcmV0dXJuIGxhdW5jaGRfYXJyYXk7Cn0KCi8vIGh0dHBzOi8vcmF3LmdpdGh1YnVzZXJjb250ZW50LmNvbS9wdWZmeWNpZC9hcnRlbWlzLWFwaS9tYXN0ZXIvbW9kLnRzCmZ1bmN0aW9uIGdldExhdW5jaGRBZ2VudHMoKSB7CiAgcmV0dXJuIGdldF9sYXVuY2hkX2FnZW50cygpOwp9CmZ1bmN0aW9uIGdldExhdW5jaGREYWVtb25zKCkgewogIHJldHVybiBnZXRfbGF1bmNoZF9kYWVtb25zKCk7Cn0KCi8vIG1haW4udHMKZnVuY3Rpb24gbWFpbigpIHsKICBjb25zdCBhZ2VudHMgPSBnZXRMYXVuY2hkQWdlbnRzKCk7CiAgY29uc3QgZGFlbW9ucyA9IGdldExhdW5jaGREYWVtb25zKCk7CiAgcmV0dXJuIGFnZW50cy5jb25jYXQoZGFlbW9ucyk7Cn0KbWFpbigpOwo=";
        let mut output = output_options("runtime_test", "local", "./tmp", false);
        let script = JSScript {
            name: String::from("launchd_daemons"),
            script: test.to_string(),
        };
        execute_script(&mut output, &script).unwrap();
    }

    #[test]
    fn test_get_launchd_agents() {
        let test = "Y29uc29sZS5sb2coRGVuby5jb3JlLm9wcy5nZXRfbGF1bmNoZF9hZ2VudHMoKVswXSk=";
        let mut output = output_options("runtime_test", "local", "./tmp", false);
        let script = JSScript {
            name: String::from("launchd_agents"),
            script: test.to_string(),
        };
        execute_script(&mut output, &script).unwrap();
    }

    #[test]
    fn test_get_launchd_daemons_in_opt() {
        let test = "ZnVuY3Rpb24gbWFpbigpIHsKICBjb25zdCBkYXRhID0gRGVuby5jb3JlLm9wcy5nZXRfbGF1bmNoZF9kYWVtb25zKCk7CiAgLy8gSXRzIGFuIGFycmF5IG9mIGpzb24gb2JqZWN0cwogIGNvbnN0IGxhdW5jaGRfYXJyYXkgPSBKU09OLnBhcnNlKGRhdGEpOwoKICBjb25zdCBsYXVuY2hkX29wdCA9IFtdOwogIGZvciAoY29uc3QgZW50cnkgb2YgbGF1bmNoZF9hcnJheSkgewogICAgaWYgKGVudHJ5WyJsYXVuY2hkX2RhdGEiXVsiUHJvZ3JhbUFyZ3VtZW50cyJdID09PSB1bmRlZmluZWQpIHsKICAgICAgY29udGludWU7CiAgICB9CgogICAgZm9yIChjb25zdCBrZXkgaW4gZW50cnlbImxhdW5jaGRfZGF0YSJdWyJQcm9ncmFtQXJndW1lbnRzIl0pIHsKICAgICAgaWYgKGVudHJ5WyJsYXVuY2hkX2RhdGEiXVsiUHJvZ3JhbUFyZ3VtZW50cyJdW2tleV0uaW5jbHVkZXMoIm9wdCIpKSB7CiAgICAgICAgbGF1bmNoZF9vcHQucHVzaChlbnRyeSk7CiAgICAgICAgYnJlYWs7CiAgICAgIH0KICAgIH0KICB9CiAgcmV0dXJuIGxhdW5jaGRfb3B0Owp9CgptYWluKCk7Cg==";
        let mut output = output_options("runtime_test", "local", "./tmp", false);
        let script = JSScript {
            name: String::from("launchd"),
            script: test.to_string(),
        };
        execute_script(&mut output, &script).unwrap();
    }
}
