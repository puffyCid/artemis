use crate::{
    artifacts::os::windows::amcache::parser::grab_amcache, runtime::error::RuntimeError,
    structs::artifacts::os::windows::AmcacheOptions,
};
use deno_core::{error::AnyError, op2};
use log::error;

#[op2]
#[string]
/// Expose parsing amcache located on systemdrive to `Deno`
pub(crate) fn get_amcache() -> Result<String, AnyError> {
    let options = AmcacheOptions { alt_file: None };
    let amcache = grab_amcache(&options)?;

    let results = serde_json::to_string(&amcache)?;
    Ok(results)
}

#[op2]
#[string]
/// Expose parsing amcache located on alt file to `Deno`
pub(crate) fn get_alt_amcache(#[string] file: String) -> Result<String, AnyError> {
    if file.is_empty() {
        error!("[runtime] Failed to parse alt amcache file");
        return Err(RuntimeError::ExecuteScript.into());
    }
    // Get the first char from string (the drive letter)
    let options = AmcacheOptions {
        alt_file: Some(file),
    };

    let amcache = grab_amcache(&options)?;
    let results = serde_json::to_string(&amcache)?;
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
    fn test_get_amcache() {
        let test = "Ly8gZGVuby1mbXQtaWdub3JlLWZpbGUKLy8gZGVuby1saW50LWlnbm9yZS1maWxlCi8vIFRoaXMgY29kZSB3YXMgYnVuZGxlZCB1c2luZyBgZGVubyBidW5kbGVgIGFuZCBpdCdzIG5vdCByZWNvbW1lbmRlZCB0byBlZGl0IGl0IG1hbnVhbGx5CgpmdW5jdGlvbiBnZXRfYW1jYWNoZSgpIHsKICAgIGNvbnN0IGRhdGEgPSBEZW5vLmNvcmUub3BzLmdldF9hbWNhY2hlKCk7CiAgICBjb25zdCBhbWNhY2hlX2FycmF5ID0gSlNPTi5wYXJzZShkYXRhKTsKICAgIHJldHVybiBhbWNhY2hlX2FycmF5Owp9CmZ1bmN0aW9uIGdldEFtY2FjaGUoKSB7CiAgICByZXR1cm4gZ2V0X2FtY2FjaGUoKTsKfQpmdW5jdGlvbiBtYWluKCkgewogICAgY29uc3QgY2FjaGUgPSBnZXRBbWNhY2hlKCk7CiAgICByZXR1cm4gY2FjaGU7Cn0KbWFpbigpOwoK";
        let mut output = output_options("runtime_test", "local", "./tmp", false);
        let script = JSScript {
            name: String::from("amcache"),
            script: test.to_string(),
        };
        execute_script(&mut output, &script).unwrap();
    }

    #[test]
    fn test_get_alt_amcache() {
        let test = "Ly8gaHR0cHM6Ly9yYXcuZ2l0aHVidXNlcmNvbnRlbnQuY29tL3B1ZmZ5Y2lkL2FydGVtaXMtYXBpL21hc3Rlci9zcmMvd2luZG93cy9hbWFjaGUudHMKZnVuY3Rpb24gZ2V0QWx0QW1jYWNoZShkcml2ZSkgewogIGNvbnN0IHJlc3VsdHMgPSBEZW5vLmNvcmUub3BzLmdldF9hbHRfYW1jYWNoZShkcml2ZSk7CiAgY29uc3QgZGF0YSA9IEpTT04ucGFyc2UocmVzdWx0cyk7CiAgcmV0dXJuIGRhdGE7Cn0KCi8vIG1haW4udHMKZnVuY3Rpb24gbWFpbigpIHsKICBjb25zdCBjYWNoZSA9IGdldEFsdEFtY2FjaGUoIkM6XFxXaW5kb3dzXFxhcHBjb21wYXRcXFByb2dyYW1zXFxBbWNhY2hlLmh2ZSIpOwogIHJldHVybiBjYWNoZTsKfQptYWluKCk7";
        let mut output = output_options("runtime_test", "local", "./tmp", false);
        let script = JSScript {
            name: String::from("amcache_alt"),
            script: test.to_string(),
        };
        execute_script(&mut output, &script).unwrap();
    }
}
