use crate::{
    artifacts::os::windows::services::parser::{grab_service_file, grab_services},
    runtime::error::RuntimeError,
    structs::artifacts::os::windows::ServicesOptions,
};
use deno_core::{error::AnyError, op};
use log::error;

#[op]
/// Expose parsing Services at default systemdrive to Deno
fn get_services() -> Result<String, AnyError> {
    let options = ServicesOptions { alt_drive: None };
    let service_result = grab_services(&options);
    let service = match service_result {
        Ok(results) => results,
        Err(err) => {
            error!("[runtime] Failed to parse services at default path: {err:?}");
            return Err(RuntimeError::ExecuteScript.into());
        }
    };

    let results = serde_json::to_string(&service)?;
    Ok(results)
}

#[op]
/// Expose parsing Services at alternative drive to Deno
fn get_alt_services(drive: String) -> Result<String, AnyError> {
    if drive.is_empty() {
        error!("[runtime] Failed to parse alt services drive. Need drive letter");
        return Err(RuntimeError::ExecuteScript.into());
    }
    // Get the first char from string (the drive letter)
    let drive_char = drive.chars().next().unwrap();
    let options = ServicesOptions {
        alt_drive: Some(drive_char),
    };

    let service_result = grab_services(&options);
    let service = match service_result {
        Ok(results) => results,
        Err(err) => {
            error!("[runtime] Failed to parse services at alt drive {drive}: {err:?}");
            return Err(RuntimeError::ExecuteScript.into());
        }
    };

    let results = serde_json::to_string(&service)?;
    Ok(results)
}

#[op]
/// Expose parsing Services file to Deno
fn get_service_file(path: String) -> Result<String, AnyError> {
    if path.is_empty() {
        error!("[runtime] Got empty service file arguement.");
        return Err(RuntimeError::ExecuteScript.into());
    }

    let service_result = grab_service_file(&path);
    let service = match service_result {
        Ok(results) => results,
        Err(err) => {
            error!("[runtime] Failed to parse service file at path {path}: {err:?}");
            return Err(RuntimeError::ExecuteScript.into());
        }
    };

    let results = serde_json::to_string(&service)?;

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
    fn test_get_services() {
        let test = "Ly8gaHR0cHM6Ly9yYXcuZ2l0aHVidXNlcmNvbnRlbnQuY29tL3B1ZmZ5Y2lkL2FydGVtaXMtYXBpL21hc3Rlci9zcmMvd2luZG93cy9zZXJ2aWNlcy50cwpmdW5jdGlvbiBnZXRTZXJ2aWNlcygpIHsKICBjb25zdCBkYXRhID0gRGVuby5jb3JlLm9wcy5nZXRfc2VydmljZXMoKTsKICBjb25zdCBzZXJ2aWNlcyA9IEpTT04ucGFyc2UoZGF0YSk7CiAgcmV0dXJuIHNlcnZpY2VzOwp9CgovLyBtYWluLnRzCmZ1bmN0aW9uIG1haW4oKSB7CiAgY29uc3QgZGF0YSA9IGdldFNlcnZpY2VzKCk7CiAgcmV0dXJuIGRhdGE7Cn0KbWFpbigpOwo=";
        let mut output = output_options("runtime_test", "local", "./tmp", false);
        let script = JSScript {
            name: String::from("service_default"),
            script: test.to_string(),
        };
        execute_script(&mut output, &script).unwrap();
    }

    #[test]
    fn test_get_alt_services() {
        let test = "Ly8gaHR0cHM6Ly9yYXcuZ2l0aHVidXNlcmNvbnRlbnQuY29tL3B1ZmZ5Y2lkL2FydGVtaXMtYXBpL21hc3Rlci9zcmMvd2luZG93cy9zZXJ2aWNlcy50cwpmdW5jdGlvbiBnZXRBbHRTZXJ2aWNlcyhhbHQpIHsKICBjb25zdCBkYXRhID0gRGVuby5jb3JlLm9wcy5nZXRfYWx0X3NlcnZpY2VzKGFsdCk7CiAgY29uc3Qgc2VydmljZXMgPSBKU09OLnBhcnNlKGRhdGEpOwogIHJldHVybiBzZXJ2aWNlczsKfQoKLy8gbWFpbi50cwpmdW5jdGlvbiBtYWluKCkgewogIGNvbnN0IGRhdGEgPSBnZXRBbHRTZXJ2aWNlcygnQycpOwogIHJldHVybiBkYXRhOwp9Cm1haW4oKTsK";
        let mut output = output_options("runtime_test", "local", "./tmp", false);
        let script = JSScript {
            name: String::from("service_alt"),
            script: test.to_string(),
        };
        execute_script(&mut output, &script).unwrap();
    }

    #[test]
    fn test_get_service_file() {
        let test = "Ly8gaHR0cHM6Ly9yYXcuZ2l0aHVidXNlcmNvbnRlbnQuY29tL3B1ZmZ5Y2lkL2FydGVtaXMtYXBpL21hc3Rlci9zcmMvd2luZG93cy9zZXJ2aWNlcy50cwpmdW5jdGlvbiBnZXRTZXJ2aWNlRmlsZShwYXRoKSB7CiAgY29uc3QgZGF0YSA9IERlbm8uY29yZS5vcHMuZ2V0X2FsdF9zZXJ2aWNlcyhwYXRoKTsKICBjb25zdCBzZXJ2aWNlcyA9IEpTT04ucGFyc2UoZGF0YSk7CiAgcmV0dXJuIHNlcnZpY2VzOwp9CgovLyBtYWluLnRzCmZ1bmN0aW9uIG1haW4oKSB7CiAgY29uc3QgZGF0YSA9IGdldFNlcnZpY2VGaWxlKCJDOlxcV2luZG93c1xcU3lzdGVtMzJcXGNvbmZpZ1xcU1lTVEVNIik7CiAgcmV0dXJuIGRhdGE7Cn0KbWFpbigpOwo=";
        let mut output = output_options("runtime_test", "local", "./tmp", false);
        let script = JSScript {
            name: String::from("service_path"),
            script: test.to_string(),
        };
        execute_script(&mut output, &script).unwrap();
    }
}
