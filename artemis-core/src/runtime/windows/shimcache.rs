use crate::{
    artifacts::os::windows::shimcache::parser::grab_shimcache, runtime::error::RuntimeError,
    structs::artifacts::os::windows::ShimcacheOptions,
};
use deno_core::{error::AnyError, op2};
use log::error;

#[op2]
#[string]
/// Expose parsing shimcache located on default drive to `Deno`
pub(crate) fn get_shimcache() -> Result<String, AnyError> {
    let options = ShimcacheOptions { alt_file: None };
    let shim = grab_shimcache(&options)?;

    let results = serde_json::to_string(&shim)?;
    Ok(results)
}

#[op2]
#[string]
/// Expose parsing alt shimcache location `Deno`
pub(crate) fn get_alt_shimcache(#[string] file: String) -> Result<String, AnyError> {
    if file.is_empty() {
        error!("[runtime] Failed to parse alt shimcache file");
        return Err(RuntimeError::ExecuteScript.into());
    }
    // Get the first char from string (the drive letter)
    let options = ShimcacheOptions {
        alt_file: Some(file),
    };

    let shim = grab_shimcache(&options)?;
    let results = serde_json::to_string(&shim)?;

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
    fn test_get_shimcache() {
        let test = "Ly8gZGVuby1mbXQtaWdub3JlLWZpbGUKLy8gZGVuby1saW50LWlnbm9yZS1maWxlCi8vIFRoaXMgY29kZSB3YXMgYnVuZGxlZCB1c2luZyBgZGVubyBidW5kbGVgIGFuZCBpdCdzIG5vdCByZWNvbW1lbmRlZCB0byBlZGl0IGl0IG1hbnVhbGx5CgpmdW5jdGlvbiBnZXRfc2hpbWNhY2hlKCkgewogICAgY29uc3QgZGF0YSA9IERlbm8uY29yZS5vcHMuZ2V0X3NoaW1jYWNoZSgpOwogICAgY29uc3Qgc2hpbV9hcnJheSA9IEpTT04ucGFyc2UoZGF0YSk7CiAgICByZXR1cm4gc2hpbV9hcnJheTsKfQpmdW5jdGlvbiBnZXRTaGltY2FjaGUoKSB7CiAgICByZXR1cm4gZ2V0X3NoaW1jYWNoZSgpOwp9CmZ1bmN0aW9uIG1haW4oKSB7CiAgICBjb25zdCBzaGltY2FjaGVfZW50cmllcyA9IGdldFNoaW1jYWNoZSgpOwogICAgcmV0dXJuIHNoaW1jYWNoZV9lbnRyaWVzOwp9Cm1haW4oKTs=";
        let mut output = output_options("runtime_test", "local", "./tmp", false);
        let script = JSScript {
            name: String::from("shimcache"),
            script: test.to_string(),
        };
        execute_script(&mut output, &script).unwrap();
    }

    #[test]
    fn test_get_alt_shimcache() {
        let test = "Ly8gaHR0cHM6Ly9yYXcuZ2l0aHVidXNlcmNvbnRlbnQuY29tL3B1ZmZ5Y2lkL2FydGVtaXMtYXBpL21hc3Rlci9zcmMvd2luZG93cy9zaGltY2FjaGUudHMKZnVuY3Rpb24gZ2V0QWx0U2hpbWNhY2hlKGRyaXZlKSB7CiAgY29uc3QgZGF0YSA9IERlbm8uY29yZS5vcHMuZ2V0X2FsdF9zaGltY2FjaGUoZHJpdmUpOwogIGNvbnN0IHJlc3VsdHMgPSBKU09OLnBhcnNlKGRhdGEpOwogIHJldHVybiByZXN1bHRzOwp9CgovLyBtYWluLnRzCmZ1bmN0aW9uIG1haW4oKSB7CiAgY29uc3QgZGF0YSA9IGdldEFsdFNoaW1jYWNoZSgiQzpcXFdpbmRvd3NcXFN5c3RlbTMyXFxjb25maWdcXFNZU1RFTSIpOwogIHJldHVybiBkYXRhOwp9Cm1haW4oKTs=";
        let mut output = output_options("runtime_test", "local", "./tmp", false);
        let script = JSScript {
            name: String::from("shimcache_alt"),
            script: test.to_string(),
        };
        execute_script(&mut output, &script).unwrap();
    }
}
