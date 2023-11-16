use crate::{
    artifacts::os::windows::tasks::parser::{grab_task_job, grab_task_xml, grab_tasks},
    runtime::error::RuntimeError,
    structs::artifacts::os::windows::TasksOptions,
};
use deno_core::{error::AnyError, op2};
use log::error;

#[op2]
#[string]
/// Expose parsing Schedule Tasks at default systemdrive to Deno
pub(crate) fn get_tasks() -> Result<String, AnyError> {
    let options = TasksOptions { alt_drive: None };
    let task = grab_tasks(&options)?;

    let results = serde_json::to_string(&task)?;
    Ok(results)
}

#[op2]
#[string]
/// Expose parsing Schedule Tasks at alternative drive to Deno
pub(crate) fn get_alt_tasks(#[string] drive: String) -> Result<String, AnyError> {
    if drive.is_empty() {
        error!("[runtime] Failed to parse alt tasks drive. Need drive letter");
        return Err(RuntimeError::ExecuteScript.into());
    }
    // Get the first char from string (the drive letter)
    let drive_char = drive.chars().next().unwrap();
    let options = TasksOptions {
        alt_drive: Some(drive_char),
    };

    let task_result = grab_tasks(&options);
    let task = match task_result {
        Ok(results) => results,
        Err(err) => {
            error!("[runtime] Failed to parse tasks at alt drive {drive}: {err:?}");
            return Err(RuntimeError::ExecuteScript.into());
        }
    };

    let results = serde_json::to_string(&task)?;
    Ok(results)
}

#[op2]
#[string]
/// Expose parsing Schedule Task file to Deno
pub(crate) fn get_task_file(#[string] path: String) -> Result<String, AnyError> {
    if path.is_empty() {
        error!("[runtime] Got empty task file arguement.");
        return Err(RuntimeError::ExecuteScript.into());
    }

    let results = if path.ends_with(".job") {
        let task_result = grab_task_job(&path);
        let task = match task_result {
            Ok(results) => results,
            Err(err) => {
                error!("[runtime] Failed to parse task job at path {path}: {err:?}");
                return Err(RuntimeError::ExecuteScript.into());
            }
        };

        serde_json::to_string(&task)?
    } else {
        let task_result = grab_task_xml(&path);
        let task = match task_result {
            Ok(results) => results,
            Err(err) => {
                error!("[runtime] Failed to parse task xml at path {path}: {err:?}");
                return Err(RuntimeError::ExecuteScript.into());
            }
        };

        serde_json::to_string(&task)?
    };

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
    fn test_get_tasks() {
        let test = "Ly8gaHR0cHM6Ly9yYXcuZ2l0aHVidXNlcmNvbnRlbnQuY29tL3B1ZmZ5Y2lkL2FydGVtaXMtYXBpL21hc3Rlci9zcmMvd2luZG93cy90YXNrcy50cwpmdW5jdGlvbiBnZXRUYXNrcygpIHsKICBjb25zdCBkYXRhID0gRGVuby5jb3JlLm9wcy5nZXRfdGFza3MoKTsKICBjb25zdCB0YXNrcyA9IEpTT04ucGFyc2UoZGF0YSk7CiAgcmV0dXJuIHRhc2tzOwp9CgovLyBtYWluLnRzCmZ1bmN0aW9uIG1haW4oKSB7CiAgY29uc3QgdGFza3MgPSBnZXRUYXNrcygpOwogIGlmICh0YXNrcyBpbnN0YW5jZW9mIEVycm9yKSB7CiAgICBjb25zb2xlLmVycm9yKGBHb3QgdGFzayBwYXJzaW5nIGVycm9yISAke3Rhc2tzfWApOwogIH0KICByZXR1cm4gdGFza3M7Cn0KbWFpbigpOwo=";
        let mut output = output_options("runtime_test", "local", "./tmp", false);
        let script = JSScript {
            name: String::from("task_default"),
            script: test.to_string(),
        };
        execute_script(&mut output, &script).unwrap();
    }

    #[test]
    fn test_get_alt_tasks() {
        let test = "Ly8gaHR0cHM6Ly9yYXcuZ2l0aHVidXNlcmNvbnRlbnQuY29tL3B1ZmZ5Y2lkL2FydGVtaXMtYXBpL21hc3Rlci9zcmMvd2luZG93cy90YXNrcy50cwpmdW5jdGlvbiBnZXRBbHRUYXNrcyhkcml2ZSkgewogIGNvbnN0IGRhdGEgPSBEZW5vLmNvcmUub3BzLmdldF9hbHRfdGFza3MoZHJpdmUpOwogIGNvbnN0IHRhc2tzID0gSlNPTi5wYXJzZShkYXRhKTsKICByZXR1cm4gdGFza3M7Cn0KCi8vIG1haW4udHMKZnVuY3Rpb24gbWFpbigpIHsKICBjb25zdCB0YXNrcyA9IGdldEFsdFRhc2tzKCJDIik7CiAgaWYgKHRhc2tzIGluc3RhbmNlb2YgRXJyb3IpIHsKICAgIGNvbnNvbGUuZXJyb3IoYEdvdCB0YXNrIHBhcnNpbmcgZXJyb3IhICR7dGFza3N9YCk7CiAgfQogIHJldHVybiB0YXNrczsKfQptYWluKCk7Cg==";
        let mut output = output_options("runtime_test", "local", "./tmp", false);
        let script = JSScript {
            name: String::from("task_alt"),
            script: test.to_string(),
        };
        execute_script(&mut output, &script).unwrap();
    }

    #[test]
    fn test_get_task_file() {
        let test = "Ly8gaHR0cHM6Ly9yYXcuZ2l0aHVidXNlcmNvbnRlbnQuY29tL3B1ZmZ5Y2lkL2FydGVtaXMtYXBpL21hc3Rlci9zcmMvd2luZG93cy90YXNrcy50cwpmdW5jdGlvbiBnZXRUYXNrRmlsZShwYXRoKSB7CiAgY29uc3QgZGF0YSA9IERlbm8uY29yZS5vcHMuZ2V0X3Rhc2tfZmlsZShwYXRoKTsKICBjb25zdCB0YXNrcyA9IEpTT04ucGFyc2UoZGF0YSk7CiAgcmV0dXJuIHRhc2tzOwp9CgovLyBodHRwczovL3Jhdy5naXRodWJ1c2VyY29udGVudC5jb20vcHVmZnljaWQvYXJ0ZW1pcy1hcGkvbWFpbi9zcmMvZmlsZXN5c3RlbS9maWxlcy50cwpmdW5jdGlvbiBnbG9iKHBhdHRlcm4pIHsKICBjb25zdCBkYXRhID0gZnMuZ2xvYihwYXR0ZXJuKTsKICBjb25zdCByZXN1bHQgPSBKU09OLnBhcnNlKGRhdGEpOwogIHJldHVybiByZXN1bHQ7Cn0KCi8vIG1haW4udHMKZnVuY3Rpb24gbWFpbigpIHsKICBjb25zdCB4bWxfZmlsZXMgPSBnbG9iKCJDOlxcV2luZG93c1xcU3lzdGVtMzJcXFRhc2tzXFwqIik7CiAgaWYgKHhtbF9maWxlcyBpbnN0YW5jZW9mIEVycm9yKSB7CiAgICBjb25zb2xlLmVycm9yKGBHb3QgZ2xvYmJpbmcgZXJyb3IhICR7eG1sX2ZpbGVzfWApOwogICAgcmV0dXJuIHhtbF9maWxlczsKICB9CiAgZm9yIChjb25zdCBlbnRyeSBvZiB4bWxfZmlsZXMpIHsKICAgIGlmICghZW50cnkuaXNfZmlsZSkgewogICAgICBjb250aW51ZTsKICAgIH0KICAgIHJldHVybiBnZXRUYXNrRmlsZShlbnRyeS5mdWxsX3BhdGgpOwogIH0KfQptYWluKCk7Cg==";
        let mut output = output_options("runtime_test", "local", "./tmp", false);
        let script = JSScript {
            name: String::from("task_path"),
            script: test.to_string(),
        };
        execute_script(&mut output, &script).unwrap();
    }
}
