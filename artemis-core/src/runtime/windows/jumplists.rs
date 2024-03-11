use crate::{
    artifacts::os::windows::jumplists::parser::{grab_jumplist_file, grab_jumplists},
    runtime::error::RuntimeError,
    structs::artifacts::os::windows::JumplistsOptions,
};
use deno_core::{error::AnyError, op2};
use log::error;

#[op2]
#[string]
/// Expose parsing Jumplists at default systemdrive to Deno
pub(crate) fn get_jumplists() -> Result<String, AnyError> {
    let options = JumplistsOptions { alt_file: None };
    let jumplist = grab_jumplists(&options)?;

    let results = serde_json::to_string(&jumplist)?;
    Ok(results)
}

#[op2]
#[string]
/// Expose parsing Jumplist file to Deno
pub(crate) fn get_jumplist_file(#[string] path: String) -> Result<String, AnyError> {
    if path.is_empty() {
        error!("[runtime] Got empty jumplist file arguement.");
        return Err(RuntimeError::ExecuteScript.into());
    }

    let jumplist_result = grab_jumplist_file(&path);
    let jumplist = match jumplist_result {
        Ok(results) => results,
        Err(err) => {
            error!("[runtime] Failed to parse jumplist file at path {path}: {err:?}");
            return Err(RuntimeError::ExecuteScript.into());
        }
    };

    let results = serde_json::to_string(&jumplist)?;
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
    fn test_get_jumplists() {
        let test = "Ly8gaHR0cHM6Ly9yYXcuZ2l0aHVidXNlcmNvbnRlbnQuY29tL3B1ZmZ5Y2lkL2FydGVtaXMtYXBpL21hc3Rlci9zcmMvd2luZG93cy9qdW1wbGlzdHMudHMKZnVuY3Rpb24gZ2V0SnVtcGxpc3RzKCkgewogIGNvbnN0IGRhdGEgPSBEZW5vLmNvcmUub3BzLmdldF9qdW1wbGlzdHMoKTsKICBjb25zdCBqdW1wID0gSlNPTi5wYXJzZShkYXRhKTsKICByZXR1cm4ganVtcDsKfQoKLy8gbWFpbi50cwpmdW5jdGlvbiBtYWluKCkgewogIGNvbnN0IGp1bXAgPSBnZXRKdW1wbGlzdHMoKTsKICByZXR1cm4ganVtcDsKfQptYWluKCk7Cg==";
        let mut output = output_options("runtime_test", "local", "./tmp", false);
        let script = JSScript {
            name: String::from("jumplist_default"),
            script: test.to_string(),
        };
        execute_script(&mut output, &script).unwrap();
    }

    #[test]
    fn test_get_jumplist_file() {
        let test = "Ly8gaHR0cHM6Ly9yYXcuZ2l0aHVidXNlcmNvbnRlbnQuY29tL3B1ZmZ5Y2lkL2FydGVtaXMtYXBpL21hc3Rlci9zcmMvZmlsZXN5c3RlbS9maWxlcy50cwpmdW5jdGlvbiBnbG9iKHBhdHRlcm4pIHsKICBjb25zdCBkYXRhID0gZnMuZ2xvYihwYXR0ZXJuKTsKICBjb25zdCByZXN1bHQgPSBKU09OLnBhcnNlKGRhdGEpOwogIHJldHVybiByZXN1bHQ7Cn0KCi8vIGh0dHBzOi8vcmF3LmdpdGh1YnVzZXJjb250ZW50LmNvbS9wdWZmeWNpZC9hcnRlbWlzLWFwaS9tYXN0ZXIvc3JjL3dpbmRvd3MvanVtcGxpc3RzLnRzCmZ1bmN0aW9uIGdldEp1bXBsaXN0UGF0aChwYXRoKSB7CiAgY29uc3QgZGF0YSA9IERlbm8uY29yZS5vcHMuZ2V0X2p1bXBsaXN0X2ZpbGUocGF0aCk7CiAgY29uc3QganVtcCA9IEpTT04ucGFyc2UoZGF0YSk7CiAgcmV0dXJuIGp1bXA7Cn0KCi8vIG1haW4udHMKZnVuY3Rpb24gbWFpbigpIHsKICBjb25zdCBwYXRocyA9IGdsb2IoIkM6XFxVc2Vyc1xcKlxcQXBwRGF0YVxcUm9hbWluZ1xcTWljcm9zb2Z0XFxXaW5kb3dzXFxSZWNlbnRcXCpEZXN0aW5hdGlvbnNcXCoiKTsKICBpZiAocGF0aHMgaW5zdGFuY2VvZiBFcnJvcikgewogICAgY29uc29sZS5lcnJvcigiRXJyb3Igd2l0aCBKdW1wbGlzdHMgZ2xvYiIpOwogICAgcmV0dXJuOwogIH0KICBmb3IgKGNvbnN0IHBhdGggb2YgcGF0aHMpIHsKICAgIGNvbnN0IGp1bXAgPSBnZXRKdW1wbGlzdFBhdGgocGF0aC5mdWxsX3BhdGgpOwogICAgcmV0dXJuIGp1bXA7CiAgfQp9Cm1haW4oKTsK";
        let mut output = output_options("runtime_test", "local", "./tmp", false);
        let script = JSScript {
            name: String::from("jumplist_path"),
            script: test.to_string(),
        };
        execute_script(&mut output, &script).unwrap();
    }
}
