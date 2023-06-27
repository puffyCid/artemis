use crate::{
    artifacts::os::windows::shimdb::parser::{custom_shimdb_path, grab_shimdb},
    runtime::error::RuntimeError,
    structs::artifacts::os::windows::ShimdbOptions,
};
use deno_core::{error::AnyError, op};
use log::error;

#[op]
/// Expose parsing shimdb located on systemdrive to `Deno`
fn get_shimdb() -> Result<String, AnyError> {
    let options = ShimdbOptions { alt_drive: None };
    let shimdb_result = grab_shimdb(&options);

    let shimdb = match shimdb_result {
        Ok(results) => results,
        Err(err) => {
            error!("[runtime] Failed to parse shimdb: {err:?}");
            return Err(RuntimeError::ExecuteScript.into());
        }
    };

    let results = serde_json::to_string_pretty(&shimdb)?;
    Ok(results)
}

#[op]
/// Expose parsing shimdb located on alt drive to `Deno`
fn get_alt_shimdb(drive: String) -> Result<String, AnyError> {
    if drive.is_empty() {
        error!("[runtime] Failed to parse alt shimdb drive. Need drive letter");
        return Err(RuntimeError::ExecuteScript.into());
    }
    // Get the first char from string (the drive letter)
    let drive_char = drive.chars().next().unwrap();
    let options = ShimdbOptions {
        alt_drive: Some(drive_char),
    };

    let shimdb_result = grab_shimdb(&options);
    let shimdb = match shimdb_result {
        Ok(results) => results,
        Err(err) => {
            error!("[runtime] Failed to parse alt shimdb: {err:?}");
            return Err(RuntimeError::ExecuteScript.into());
        }
    };

    let results = serde_json::to_string_pretty(&shimdb)?;
    Ok(results)
}

#[op]
/// Expose parsing custom shimdb path to `Deno`
fn get_custom_shimdb(paths: String) -> Result<String, AnyError> {
    let shimdb_result = custom_shimdb_path(&paths);
    let shimdb = match shimdb_result {
        Ok(results) => results,
        Err(_err) => {
            // Parsing sdb files could fail for many reasons (ex: file is not a sdb file)
            // Instead of cancelling the whole script, return empty result
            return Ok(String::new());
        }
    };

    let results = serde_json::to_string_pretty(&shimdb)?;
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
    fn test_get_shimdb() {
        let test = "Ly8gZGVuby1mbXQtaWdub3JlLWZpbGUKLy8gZGVuby1saW50LWlnbm9yZS1maWxlCi8vIFRoaXMgY29kZSB3YXMgYnVuZGxlZCB1c2luZyBgZGVubyBidW5kbGVgIGFuZCBpdCdzIG5vdCByZWNvbW1lbmRlZCB0byBlZGl0IGl0IG1hbnVhbGx5CgpmdW5jdGlvbiBnZXRfc2hpbWRiKCkgewogICAgY29uc3QgZGF0YSA9IERlbm9bRGVuby5pbnRlcm5hbF0uY29yZS5vcHMuZ2V0X3NoaW1kYigpOwogICAgY29uc3Qgc2hpbV9hcnJheSA9IEpTT04ucGFyc2UoZGF0YSk7CiAgICByZXR1cm4gc2hpbV9hcnJheTsKfQpmdW5jdGlvbiBnZXRTaGltZGIoKSB7CiAgICByZXR1cm4gZ2V0X3NoaW1kYigpOwp9CmZ1bmN0aW9uIG1haW4oKSB7CiAgICBjb25zdCBzZGIgPSBnZXRTaGltZGIoKTsKICAgIHJldHVybiBzZGI7Cn0KbWFpbigpOwoK";
        let mut output = output_options("runtime_test", "local", "./tmp", false);
        let script = JSScript {
            name: String::from("shimdb"),
            script: test.to_string(),
        };
        execute_script(&mut output, &script).unwrap();
    }

    #[test]
    fn test_get_alt_shimdb() {
        let test = "Ly8gZGVuby1mbXQtaWdub3JlLWZpbGUKLy8gZGVuby1saW50LWlnbm9yZS1maWxlCi8vIFRoaXMgY29kZSB3YXMgYnVuZGxlZCB1c2luZyBgZGVubyBidW5kbGVgIGFuZCBpdCdzIG5vdCByZWNvbW1lbmRlZCB0byBlZGl0IGl0IG1hbnVhbGx5CgpmdW5jdGlvbiBnZXRfYWx0X3NoaW1kYihkcml2ZSkgewogICAgY29uc3QgZGF0YSA9IERlbm9bRGVuby5pbnRlcm5hbF0uY29yZS5vcHMuZ2V0X2FsdF9zaGltZGIoZHJpdmUpOwogICAgY29uc3Qgc2hpbV9hcnJheSA9IEpTT04ucGFyc2UoZGF0YSk7CiAgICByZXR1cm4gc2hpbV9hcnJheTsKfQpmdW5jdGlvbiBnZXRBbHRTaGltZGIoZHJpdmUpIHsKICAgIHJldHVybiBnZXRfYWx0X3NoaW1kYihkcml2ZSk7Cn0KZnVuY3Rpb24gbWFpbigpIHsKICAgIGNvbnN0IHNkYiA9IGdldEFsdFNoaW1kYigiQyIpOwogICAgcmV0dXJuIHNkYjsKfQptYWluKCk7Cgo=";
        let mut output = output_options("runtime_test", "local", "./tmp", false);
        let script = JSScript {
            name: String::from("shimdb_alt"),
            script: test.to_string(),
        };
        execute_script(&mut output, &script).unwrap();
    }

    #[test]
    #[ignore = "Searches all files under Users"]
    fn test_get_custom_shimdb() {
        let test = "Ly8gZGVuby1mbXQtaWdub3JlLWZpbGUKLy8gZGVuby1saW50LWlnbm9yZS1maWxlCi8vIFRoaXMgY29kZSB3YXMgYnVuZGxlZCB1c2luZyBgZGVubyBidW5kbGVgIGFuZCBpdCdzIG5vdCByZWNvbW1lbmRlZCB0byBlZGl0IGl0IG1hbnVhbGx5CgpmdW5jdGlvbiBnZXRfY3VzdG9tX3NoaW1kYihwYXRoKSB7CiAgICBjb25zdCBkYXRhID0gRGVub1tEZW5vLmludGVybmFsXS5jb3JlLm9wcy5nZXRfY3VzdG9tX3NoaW1kYihwYXRoKTsKICAgIGlmIChkYXRhID09PSAiIikgewogICAgICAgIHJldHVybiBudWxsOwogICAgfQogICAgY29uc3Qgc2hpbV9hcnJheSA9IEpTT04ucGFyc2UoZGF0YSk7CiAgICByZXR1cm4gc2hpbV9hcnJheTsKfQpmdW5jdGlvbiBnZXRDdXN0b21TaGltZGIocGF0aCkgewogICAgcmV0dXJuIGdldF9jdXN0b21fc2hpbWRiKHBhdGgpOwp9CmZ1bmN0aW9uIG1haW4oKSB7CiAgICBjb25zdCBkcml2ZSA9IERlbm8uZW52LmdldCgiU3lzdGVtRHJpdmUiKTsKICAgIGlmIChkcml2ZSA9PT0gdW5kZWZpbmVkKSB7CiAgICAgICAgcmV0dXJuIFtdOwogICAgfQogICAgY29uc3QgdXNlcnMgPSBgJHtkcml2ZX1cXFVzZXJzYDsKICAgIGNvbnN0IGN1c3RvbV9zZGIgPSBbXTsKICAgIHJlY3Vyc2VfZGlyKGN1c3RvbV9zZGIsIHVzZXJzKTsKICAgIHJldHVybiBjdXN0b21fc2RiOwp9CmZ1bmN0aW9uIHJlY3Vyc2VfZGlyKHNkYnMsIHN0YXJ0X3BhdGgpIHsKICAgIGZvciAoY29uc3QgZW50cnkgb2YgRGVuby5yZWFkRGlyU3luYyhzdGFydF9wYXRoKSl7CiAgICAgICAgY29uc3Qgc2RiX3BhdGggPSBgJHtzdGFydF9wYXRofVxcJHtlbnRyeS5uYW1lfWA7CiAgICAgICAgaWYgKGVudHJ5LmlzRmlsZSkgewogICAgICAgICAgICBjb25zdCBkYXRhID0gZ2V0Q3VzdG9tU2hpbWRiKHNkYl9wYXRoKTsKICAgICAgICAgICAgaWYgKGRhdGEgPT09IG51bGwpIHsKICAgICAgICAgICAgICAgIGNvbnRpbnVlOwogICAgICAgICAgICB9CiAgICAgICAgICAgIHNkYnMucHVzaChkYXRhKTsKICAgICAgICB9CiAgICAgICAgaWYgKGVudHJ5LmlzRGlyZWN0b3J5KSB7CiAgICAgICAgICAgIHJlY3Vyc2VfZGlyKHNkYnMsIHNkYl9wYXRoKTsKICAgICAgICB9CiAgICB9Cn0KbWFpbigpOwoK";
        let mut output = output_options("runtime_test", "local", "./tmp", false);
        let script = JSScript {
            name: String::from("custom_sdb_files"),
            script: test.to_string(),
        };
        execute_script(&mut output, &script).unwrap();
    }
}
