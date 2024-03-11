use crate::{
    artifacts::os::windows::services::parser::{grab_service_file, grab_services},
    runtime::error::RuntimeError,
    structs::artifacts::os::windows::ServicesOptions,
};
use deno_core::{error::AnyError, op2};
use log::error;

#[op2]
#[string]
/// Expose parsing Services at default systemdrive to Deno
pub(crate) fn get_services() -> Result<String, AnyError> {
    let options = ServicesOptions { alt_file: None };
    let service = grab_services(&options)?;

    let results = serde_json::to_string(&service)?;
    Ok(results)
}

#[op2]
#[string]
/// Expose parsing Services file to Deno
pub(crate) fn get_service_file(#[string] path: String) -> Result<String, AnyError> {
    if path.is_empty() {
        error!("[runtime] Got empty service file arguement.");
        return Err(RuntimeError::ExecuteScript.into());
    }

    let service = grab_service_file(&path)?;
    let results = serde_json::to_string(&service)?;

    Ok(results)
}

#[cfg(test)]
#[cfg(target_os = "windows")]
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
    fn test_get_service_file() {
        let test = "Ly8gaHR0cHM6Ly9yYXcuZ2l0aHVidXNlcmNvbnRlbnQuY29tL3B1ZmZ5Y2lkL2FydGVtaXMtYXBpL21hc3Rlci9zcmMvd2luZG93cy9zZXJ2aWNlcy50cwpmdW5jdGlvbiBnZXRTZXJ2aWNlRmlsZShwYXRoKSB7CiAgY29uc3QgZGF0YSA9IERlbm8uY29yZS5vcHMuZ2V0X3NlcnZpY2VfZmlsZShwYXRoKTsKICBjb25zdCBzZXJ2aWNlcyA9IEpTT04ucGFyc2UoZGF0YSk7CiAgcmV0dXJuIHNlcnZpY2VzOwp9CgovLyBtYWluLnRzCmZ1bmN0aW9uIG1haW4oKSB7CiAgY29uc3QgZGF0YSA9IGdldFNlcnZpY2VGaWxlKCJDOlxcV2luZG93c1xcU3lzdGVtMzJcXGNvbmZpZ1xcU1lTVEVNIik7CiAgcmV0dXJuIGRhdGE7Cn0KbWFpbigpOwo=";
        let mut output = output_options("runtime_test", "local", "./tmp", false);
        let script = JSScript {
            name: String::from("service_path"),
            script: test.to_string(),
        };
        execute_script(&mut output, &script).unwrap();
    }
}
