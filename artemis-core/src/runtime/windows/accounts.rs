use crate::{
    artifacts::os::windows::accounts::parser::grab_users, runtime::error::RuntimeError,
    structs::artifacts::os::windows::WindowsUserOptions,
};
use deno_core::{error::AnyError, op2};
use log::error;

#[op2]
#[string]
/// Expose parsing user info to `Deno`
pub(crate) fn get_users_windows() -> Result<String, AnyError> {
    let options = WindowsUserOptions { alt_file: None };

    let users = grab_users(&options)?;
    let results = serde_json::to_string(&users)?;
    Ok(results)
}

#[op2]
#[string]
/// Expose parsing user info on alt drive to `Deno`
pub(crate) fn get_alt_users_windows(#[string] file: String) -> Result<String, AnyError> {
    if file.is_empty() {
        error!("[runtime] Failed to parse user info. Need full path");
        return Err(RuntimeError::ExecuteScript.into());
    }
    // Get the first char from string (the drive letter)
    let options = WindowsUserOptions {
        alt_file: Some(file),
    };

    let users = grab_users(&options)?;
    let results = serde_json::to_string(&users)?;
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
    fn test_get_users() {
        let test = "Ly8gLi4vLi4vYXJ0ZW1pcy1hcGkvc3JjL3dpbmRvd3MvdXNlcnMudHMKZnVuY3Rpb24gZ2V0X3VzZXJzX3dpbigpIHsKICBjb25zdCBkYXRhID0gRGVuby5jb3JlLm9wcy5nZXRfdXNlcnNfd2luZG93cygpOwogIGNvbnN0IHVzZXJfYXJyYXkgPSBKU09OLnBhcnNlKGRhdGEpOwogIHJldHVybiB1c2VyX2FycmF5Owp9CgovLyAuLi8uLi9hcnRlbWlzLWFwaS9tb2QudHMKZnVuY3Rpb24gZ2V0VXNlcnNXaW4oKSB7CiAgcmV0dXJuIGdldF91c2Vyc193aW4oKTsKfQoKLy8gbWFpbi50cwpmdW5jdGlvbiBtYWluKCkgewogIGNvbnN0IHVzZXJzID0gZ2V0VXNlcnNXaW4oKTsKICByZXR1cm4gdXNlcnM7Cn0KbWFpbigpOwo=";
        let mut output = output_options("runtime_test", "local", "./tmp", false);
        let script = JSScript {
            name: String::from("users"),
            script: test.to_string(),
        };
        execute_script(&mut output, &script).unwrap();
    }

    #[test]
    fn test_get_alt_users() {
        let test = "Ly8gaHR0cHM6Ly9yYXcuZ2l0aHVidXNlcmNvbnRlbnQuY29tL3B1ZmZ5Y2lkL2FydGVtaXMtYXBpL21hc3Rlci9zcmMvd2luZG93cy91c2Vycy50cwpmdW5jdGlvbiBnZXRBbHRVc2Vyc1dpbihkcml2ZSkgewogIGNvbnN0IGRhdGEgPSBEZW5vLmNvcmUub3BzLmdldF9hbHRfdXNlcnNfd2luZG93cyhkcml2ZSk7CiAgY29uc3QgcmVzdWx0cyA9IEpTT04ucGFyc2UoZGF0YSk7CiAgcmV0dXJuIHJlc3VsdHM7Cn0KCi8vIG1haW4udHMKZnVuY3Rpb24gbWFpbigpIHsKICBjb25zdCB1c2VycyA9IGdldEFsdFVzZXJzV2luKCJDOlxcV2luZG93c1xcU3lzdGVtMzJcXGNvbmZpZ1xcU0FNIik7CiAgcmV0dXJuIHVzZXJzOwp9Cm1haW4oKTsK";
        let mut output = output_options("runtime_test", "local", "./tmp", false);
        let script = JSScript {
            name: String::from("users_alt"),
            script: test.to_string(),
        };
        execute_script(&mut output, &script).unwrap();
    }
}
