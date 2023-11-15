use crate::{
    artifacts::os::windows::accounts::parser::grab_users,
    structs::artifacts::os::windows::UserOptions,
};
use deno_core::{error::AnyError, op2};

#[op2]
#[string]
/// Expose parsing user info to `Deno`
pub(crate) fn get_users() -> Result<String, AnyError> {
    let options = UserOptions { alt_drive: None };

    let users = grab_users(&options)?;
    let results = serde_json::to_string(&users)?;
    Ok(results)
}

#[op2]
#[string]
/// Expose parsing user info on alt drive to `Deno`
pub(crate) fn get_alt_users(#[string] drive: String) -> Result<String, AnyError> {
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
        let test = "Ly8gLi4vLi4vYXJ0ZW1pcy1hcGkvc3JjL3dpbmRvd3MvdXNlcnMudHMKZnVuY3Rpb24gZ2V0X3VzZXJzX3dpbigpIHsKICBjb25zdCBkYXRhID0gRGVuby5jb3JlLm9wcy5nZXRfdXNlcnMoKTsKICBjb25zdCB1c2VyX2FycmF5ID0gSlNPTi5wYXJzZShkYXRhKTsKICByZXR1cm4gdXNlcl9hcnJheTsKfQoKLy8gLi4vLi4vYXJ0ZW1pcy1hcGkvbW9kLnRzCmZ1bmN0aW9uIGdldFVzZXJzV2luKCkgewogIHJldHVybiBnZXRfdXNlcnNfd2luKCk7Cn0KCi8vIG1haW4udHMKZnVuY3Rpb24gbWFpbigpIHsKICBjb25zdCB1c2VycyA9IGdldFVzZXJzV2luKCk7CiAgcmV0dXJuIHVzZXJzOwp9Cm1haW4oKTsK";
        let mut output = output_options("runtime_test", "local", "./tmp", false);
        let script = JSScript {
            name: String::from("users"),
            script: test.to_string(),
        };
        execute_script(&mut output, &script).unwrap();
    }

    #[test]
    fn test_get_alt_users() {
        let test = "Ly8gaHR0cHM6Ly9yYXcuZ2l0aHVidXNlcmNvbnRlbnQuY29tL3B1ZmZ5Y2lkL2FydGVtaXMtYXBpL21hc3Rlci9zcmMvd2luZG93cy91c2Vycy50cwpmdW5jdGlvbiBnZXRBbHRVc2Vyc1dpbihkcml2ZSkgewogIGNvbnN0IGRhdGEgPSBEZW5vLmNvcmUub3BzLmdldF9hbHRfdXNlcnMoZHJpdmUpOwogIGNvbnN0IHJlc3VsdHMgPSBKU09OLnBhcnNlKGRhdGEpOwogIHJldHVybiByZXN1bHRzOwp9CgovLyBtYWluLnRzCmZ1bmN0aW9uIG1haW4oKSB7CiAgY29uc3QgdXNlcnMgPSBnZXRBbHRVc2Vyc1dpbigiQyIpOwogIHJldHVybiB1c2VyczsKfQptYWluKCk7Cg==";
        let mut output = output_options("runtime_test", "local", "./tmp", false);
        let script = JSScript {
            name: String::from("users_alt"),
            script: test.to_string(),
        };
        execute_script(&mut output, &script).unwrap();
    }
}
