use crate::{
    artifacts::os::windows::shimcache::parser::grab_shimcache, runtime::error::RuntimeError,
    structs::artifacts::os::windows::ShimcacheOptions,
};
use deno_core::{error::AnyError, op};
use log::error;

#[op]
/// Expose parsing shimcache located on default drive to `Deno`
fn get_shimcache() -> Result<String, AnyError> {
    let options = ShimcacheOptions { alt_drive: None };
    let shim_results = grab_shimcache(&options);
    let reg = match shim_results {
        Ok(results) => results,
        Err(err) => {
            error!("[runtime] Failed to parse shimcache: {err:?}");
            return Err(RuntimeError::ExecuteScript.into());
        }
    };

    let results = serde_json::to_string_pretty(&reg)?;
    Ok(results)
}

#[op]
/// Expose parsing shimcache located on alt drive to `Deno`
fn get_alt_shimcache(drive: String) -> Result<String, AnyError> {
    if drive.is_empty() {
        error!("[runtime] Failed to parse alt shimcache drive. Need drive letter");
        return Err(RuntimeError::ExecuteScript.into());
    }
    // Get the first char from string (the drive letter)
    let drive_char = drive.chars().next().unwrap();
    let options = ShimcacheOptions {
        alt_drive: Some(drive_char),
    };

    let shim_results = grab_shimcache(&options);
    let reg = match shim_results {
        Ok(results) => results,
        Err(err) => {
            error!("[runtime] Failed to parse shimcache: {err:?}");
            return Err(RuntimeError::ExecuteScript.into());
        }
    };

    let results = serde_json::to_string_pretty(&reg)?;
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
            port: Some(0),
            api_key: Some(String::new()),
            username: Some(String::new()),
            password: Some(String::new()),
            generic_keys: Some(Vec::new()),
            endpoint_id: String::from("abcd"),
            collection_id: 0,
            output: output.to_string(),
            filter_name: None,
            filter_script: None,
        }
    }

    #[test]
    fn test_get_shimcache() {
        let test = "Ly8gZGVuby1mbXQtaWdub3JlLWZpbGUKLy8gZGVuby1saW50LWlnbm9yZS1maWxlCi8vIFRoaXMgY29kZSB3YXMgYnVuZGxlZCB1c2luZyBgZGVubyBidW5kbGVgIGFuZCBpdCdzIG5vdCByZWNvbW1lbmRlZCB0byBlZGl0IGl0IG1hbnVhbGx5CgpmdW5jdGlvbiBnZXRfc2hpbWNhY2hlKCkgewogICAgY29uc3QgZGF0YSA9IERlbm9bRGVuby5pbnRlcm5hbF0uY29yZS5vcHMuZ2V0X3NoaW1jYWNoZSgpOwogICAgY29uc3Qgc2hpbV9hcnJheSA9IEpTT04ucGFyc2UoZGF0YSk7CiAgICByZXR1cm4gc2hpbV9hcnJheTsKfQpmdW5jdGlvbiBnZXRTaGltY2FjaGUoKSB7CiAgICByZXR1cm4gZ2V0X3NoaW1jYWNoZSgpOwp9CmZ1bmN0aW9uIG1haW4oKSB7CiAgICBjb25zdCBzaGltY2FjaGVfZW50cmllcyA9IGdldFNoaW1jYWNoZSgpOwogICAgcmV0dXJuIHNoaW1jYWNoZV9lbnRyaWVzOwp9Cm1haW4oKTs=";
        let mut output = output_options("runtime_test", "local", "./tmp", false);
        let script = JSScript {
            name: String::from("shimcache"),
            script: test.to_string(),
        };
        execute_script(&mut output, &script).unwrap();
    }

    #[test]
    fn test_get_alt_shimcache() {
        let test = "Ly8gZGVuby1mbXQtaWdub3JlLWZpbGUKLy8gZGVuby1saW50LWlnbm9yZS1maWxlCi8vIFRoaXMgY29kZSB3YXMgYnVuZGxlZCB1c2luZyBgZGVubyBidW5kbGVgIGFuZCBpdCdzIG5vdCByZWNvbW1lbmRlZCB0byBlZGl0IGl0IG1hbnVhbGx5CgpmdW5jdGlvbiBnZXRfYWx0X3NoaW1jYWNoZShkcml2ZSkgewogICAgY29uc3QgZGF0YSA9IERlbm9bRGVuby5pbnRlcm5hbF0uY29yZS5vcHMuZ2V0X2FsdF9zaGltY2FjaGUoZHJpdmUpOwogICAgY29uc3Qgc2hpbV9hcnJheSA9IEpTT04ucGFyc2UoZGF0YSk7CiAgICByZXR1cm4gc2hpbV9hcnJheTsKfQpmdW5jdGlvbiBnZXRBbHRTaGltY2FjaGUoZHJpdmUpIHsKICAgIHJldHVybiBnZXRfYWx0X3NoaW1jYWNoZShkcml2ZSk7Cn0KZnVuY3Rpb24gbWFpbigpIHsKICAgIGNvbnN0IHNoaW1jYWNoZV9lbnRyaWVzID0gZ2V0QWx0U2hpbWNhY2hlKCJDIik7CiAgICByZXR1cm4gc2hpbWNhY2hlX2VudHJpZXM7Cn0KbWFpbigpOw==";
        let mut output = output_options("runtime_test", "local", "./tmp", false);
        let script = JSScript {
            name: String::from("shimcache_alt"),
            script: test.to_string(),
        };
        execute_script(&mut output, &script).unwrap();
    }
}
