use crate::{
    artifacts::os::windows::wmi::parser::grab_wmi_persist, runtime::error::RuntimeError,
    structs::artifacts::os::windows::WmiPersistOptions,
};
use deno_core::{error::AnyError, op2};
use log::error;

#[op2]
#[string]
/// Expose parsing wmi persist to `Deno`
pub(crate) fn get_wmipersist() -> Result<String, AnyError> {
    let options = WmiPersistOptions { alt_dir: None };

    let assist = grab_wmi_persist(&options)?;
    let results = serde_json::to_string(&assist)?;

    Ok(results)
}

#[op2]
#[string]
/// Expose parsing wmi persist at path to `Deno`
pub(crate) fn get_alt_wmipersist(#[string] path: String) -> Result<String, AnyError> {
    if path.is_empty() {
        error!("[runtime] Failed to parse wmi path. Path is empty");
        return Err(RuntimeError::ExecuteScript.into());
    }
    // Get the first char from string (the drive letter)
    let options = WmiPersistOptions {
        alt_dir: Some(path),
    };

    let assist = grab_wmi_persist(&options)?;

    let results = serde_json::to_string(&assist)?;
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
    fn test_get_wmipersist() {
        let test = "Ly8gLi4vLi4vUHJvamVjdHMvYXJ0ZW1pcy1hcGkvc3JjL3V0aWxzL2Vycm9yLnRzCnZhciBFcnJvckJhc2UgPSBjbGFzcyBleHRlbmRzIEVycm9yIHsKICBjb25zdHJ1Y3RvcihuYW1lLCBtZXNzYWdlKSB7CiAgICBzdXBlcigpOwogICAgdGhpcy5uYW1lID0gbmFtZTsKICAgIHRoaXMubWVzc2FnZSA9IG1lc3NhZ2U7CiAgfQp9OwoKLy8gLi4vLi4vUHJvamVjdHMvYXJ0ZW1pcy1hcGkvc3JjL3dpbmRvd3MvZXJyb3JzLnRzCnZhciBXaW5kb3dzRXJyb3IgPSBjbGFzcyBleHRlbmRzIEVycm9yQmFzZSB7Cn07CgovLyAuLi8uLi9Qcm9qZWN0cy9hcnRlbWlzLWFwaS9zcmMvZW52aXJvbm1lbnQvZW52LnRzCmZ1bmN0aW9uIGdldEVudlZhbHVlKGtleSkgewogIGNvbnN0IGRhdGEgPSBlbnYuZW52aXJvbm1lbnRWYWx1ZShrZXkpOwogIHJldHVybiBkYXRhOwp9CgovLyAuLi8uLi9Qcm9qZWN0cy9hcnRlbWlzLWFwaS9zcmMvd2luZG93cy93bWkudHMKZnVuY3Rpb24gZ2V0V21pUGVyc2lzdCgpIHsKICB0cnkgewogICAgY29uc3QgZGF0YSA9IERlbm8uY29yZS5vcHMuZ2V0X3dtaXBlcnNpc3QoKTsKICAgIGNvbnN0IHJlc3VsdHMgPSBKU09OLnBhcnNlKGRhdGEpOwogICAgcmV0dXJuIHJlc3VsdHM7CiAgfSBjYXRjaCAoZXJyKSB7CiAgICByZXR1cm4gbmV3IFdpbmRvd3NFcnJvcigiV01JUEVSU0lTVCIsIGBmYWlsZWQgdG8gcGFyc2UgV01JIHJlcG86ICR7ZXJyfWApOwogIH0KfQoKLy8gbWFpbi50cwpmdW5jdGlvbiBtYWluKCkgewogIGNvbnN0IGRhdGEgPSBnZXRXbWlQZXJzaXN0KCk7CiAgcmV0dXJuIGRhdGE7Cn0KbWFpbigpOwo=";
        let mut output = output_options("runtime_test", "local", "./tmp", false);
        let script = JSScript {
            name: String::from("wmipersist"),
            script: test.to_string(),
        };
        execute_script(&mut output, &script).unwrap();
    }

    #[test]
    fn test_get_alt_wmipersist() {
        let test = "aW1wb3J0IHsgZ2V0V21pUGVyc2lzdFBhdGggfSBmcm9tICIuLi8uLi9Qcm9qZWN0cy9hcnRlbWlzLWFwaS9zcmMvd2luZG93cy93bWkudHMiOwoKZnVuY3Rpb24gbWFpbigpIHsKICBjb25zdCBkYXRhID0gZ2V0V21pUGVyc2lzdFBhdGgoIi4iKTsKICByZXR1cm4gZGF0YTsKfQoKbWFpbigpOwo=";
        let mut output = output_options("runtime_test", "local", "./tmp", false);
        let script = JSScript {
            name: String::from("wmipersist_alt"),
            script: test.to_string(),
        };
        execute_script(&mut output, &script).unwrap();
    }
}
