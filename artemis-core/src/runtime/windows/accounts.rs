use crate::{
    artifacts::os::windows::accounts::parser::grab_users, runtime::error::RuntimeError,
    structs::artifacts::os::windows::UserOptions,
};
use deno_core::{error::AnyError, op};
use log::error;

#[op]
/// Expose parsing user info to `Deno`
fn get_users() -> Result<String, AnyError> {
    let options = UserOptions { alt_drive: None };

    let users_results = grab_users(&options);
    let users = match users_results {
        Ok(results) => results,
        Err(err) => {
            error!("[runtime] Failed to parse users: {err:?}");
            return Err(RuntimeError::ExecuteScript.into());
        }
    };

    let results = serde_json::to_string_pretty(&users)?;
    Ok(results)
}

#[op]
/// Expose parsing user info on alt drive to `Deno`
fn get_alt_users(drive: String) -> Result<String, AnyError> {
    if drive.is_empty() {
        error!("[runtime] Failed to parse user info. Need drive letter");
        return Err(RuntimeError::ExecuteScript.into());
    }
    // Get the first char from string (the drive letter)
    let drive_char = &drive.chars().next().unwrap();
    let options = UserOptions {
        alt_drive: Some(drive_char.to_owned()),
    };

    let users_results = grab_users(&options);
    let users = match users_results {
        Ok(results) => results,
        Err(err) => {
            error!("[runtime] Failed to parse alt users: {err:?}");
            return Err(RuntimeError::ExecuteScript.into());
        }
    };

    let results = serde_json::to_string_pretty(&users)?;
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
        let test = "Ly8gLi4vLi4vYXJ0ZW1pcy1hcGkvc3JjL3dpbmRvd3MvdXNlcnMudHMKZnVuY3Rpb24gZ2V0X3VzZXJzX3dpbigpIHsKICBjb25zdCBkYXRhID0gRGVub1tEZW5vLmludGVybmFsXS5jb3JlLm9wcy5nZXRfdXNlcnMoKTsKICBjb25zdCB1c2VyX2FycmF5ID0gSlNPTi5wYXJzZShkYXRhKTsKICByZXR1cm4gdXNlcl9hcnJheTsKfQoKLy8gLi4vLi4vYXJ0ZW1pcy1hcGkvbW9kLnRzCmZ1bmN0aW9uIGdldFVzZXJzV2luKCkgewogIHJldHVybiBnZXRfdXNlcnNfd2luKCk7Cn0KCi8vIG1haW4udHMKZnVuY3Rpb24gbWFpbigpIHsKICBjb25zdCB1c2VycyA9IGdldFVzZXJzV2luKCk7CiAgcmV0dXJuIHVzZXJzOwp9Cm1haW4oKTsK";
        let mut output = output_options("runtime_test", "local", "./tmp", false);
        let script = JSScript {
            name: String::from("users"),
            script: test.to_string(),
        };
        execute_script(&mut output, &script).unwrap();
    }

    #[test]
    fn test_get_alt_users() {
        let test = "Ly8gLi4vLi4vYXJ0ZW1pcy1hcGkvc3JjL3dpbmRvd3MvdXNlcnMudHMKZnVuY3Rpb24gZ2V0X2FsdF91c2Vyc193aW4oZHJpdmUpIHsKICBjb25zdCBkYXRhID0gRGVub1tEZW5vLmludGVybmFsXS5jb3JlLm9wcy5nZXRfYWx0X3VzZXJzKGRyaXZlKTsKICBjb25zdCB1c2VyX2FycmF5ID0gSlNPTi5wYXJzZShkYXRhKTsKICByZXR1cm4gdXNlcl9hcnJheTsKfQoKLy8gLi4vLi4vYXJ0ZW1pcy1hcGkvbW9kLnRzCmZ1bmN0aW9uIGdldEFsdFVzZXJzV2luKGRyaXZlKSB7CiAgcmV0dXJuIGdldF9hbHRfdXNlcnNfd2luKGRyaXZlKTsKfQoKLy8gbWFpbi50cwpmdW5jdGlvbiBtYWluKCkgewogIGNvbnN0IGRyaXZlID0gRGVuby5lbnYuZ2V0KCJTeXN0ZW1Ecml2ZSIpOwogIGlmIChkcml2ZSA9PT0gdm9pZCAwKSB7CiAgICByZXR1cm4gW107CiAgfQogIGNvbnN0IHVzZXJzID0gZ2V0QWx0VXNlcnNXaW4oZHJpdmUpOwogIHJldHVybiB1c2VyczsKfQptYWluKCk7Cg==";
        let mut output = output_options("runtime_test", "local", "./tmp", false);
        let script = JSScript {
            name: String::from("users_alt"),
            script: test.to_string(),
        };
        execute_script(&mut output, &script).unwrap();
    }
}
