use crate::{
    artifacts::os::windows::shellbags::parser::grab_shellbags, runtime::error::RuntimeError,
    structs::artifacts::os::windows::ShellbagsOptions,
};
use deno_core::{error::AnyError, op2};
use log::error;

#[op2]
#[string]
/// Expose parsing shellbags located on systemdrive to `Deno`
pub(crate) fn get_shellbags(resolve: bool) -> Result<String, AnyError> {
    let options = ShellbagsOptions {
        alt_file: None,
        resolve_guids: resolve,
    };
    let bags = grab_shellbags(&options)?;

    let results = serde_json::to_string(&bags)?;
    Ok(results)
}

#[op2]
#[string]
/// Expose parsing shellbags located on alt file to `Deno`
pub(crate) fn get_alt_shellbags(resolve: bool, #[string] file: String) -> Result<String, AnyError> {
    if file.is_empty() {
        error!("[runtime] Failed to parse alt shellbags file");
        return Err(RuntimeError::ExecuteScript.into());
    }
    // Get the first char from string (the drive letter)
    let options = ShellbagsOptions {
        alt_file: Some(file),
        resolve_guids: resolve,
    };

    let bags = grab_shellbags(&options)?;

    let results = serde_json::to_string(&bags)?;
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
    fn test_get_shellbags() {
        let test = "Ly8gZGVuby1mbXQtaWdub3JlLWZpbGUKLy8gZGVuby1saW50LWlnbm9yZS1maWxlCi8vIFRoaXMgY29kZSB3YXMgYnVuZGxlZCB1c2luZyBgZGVubyBidW5kbGVgIGFuZCBpdCdzIG5vdCByZWNvbW1lbmRlZCB0byBlZGl0IGl0IG1hbnVhbGx5CgpmdW5jdGlvbiBnZXRfc2hlbGxiYWdzKHJlc29sdmVfZ3VpZHMpIHsKICAgIGNvbnN0IGRhdGEgPSBEZW5vLmNvcmUub3BzLmdldF9zaGVsbGJhZ3MocmVzb2x2ZV9ndWlkcyk7CiAgICBjb25zdCBiYWdzX2FycmF5ID0gSlNPTi5wYXJzZShkYXRhKTsKICAgIHJldHVybiBiYWdzX2FycmF5Owp9CmZ1bmN0aW9uIGdldFNoZWxsYmFncyhyZXNvbHZlX2d1aWRzKSB7CiAgICByZXR1cm4gZ2V0X3NoZWxsYmFncyhyZXNvbHZlX2d1aWRzKTsKfQpmdW5jdGlvbiBtYWluKCkgewogICAgY29uc3QgYmFncyA9IGdldFNoZWxsYmFncyh0cnVlKTsKICAgIHJldHVybiBiYWdzOwp9Cm1haW4oKTs=";
        let mut output = output_options("runtime_test", "local", "./tmp", false);
        let script = JSScript {
            name: String::from("shellbags"),
            script: test.to_string(),
        };
        execute_script(&mut output, &script).unwrap();
    }

    #[test]
    fn test_get_alt_shellbags() {
        let test = "Ly8gaHR0cHM6Ly9yYXcuZ2l0aHVidXNlcmNvbnRlbnQuY29tL3B1ZmZ5Y2lkL2FydGVtaXMtYXBpL21hc3Rlci9zcmMvZmlsZXN5c3RlbS9maWxlcy50cwpmdW5jdGlvbiBnbG9iKHBhdHRlcm4pIHsKICBjb25zdCByZXN1bHQgPSBmcy5nbG9iKHBhdHRlcm4pOwogIGlmIChyZXN1bHQgaW5zdGFuY2VvZiBFcnJvcikgewogICAgcmV0dXJuIHJlc3VsdDsKICB9CiAgY29uc3QgZGF0YSA9IEpTT04ucGFyc2UocmVzdWx0KTsKICByZXR1cm4gZGF0YTsKfQoKLy8gaHR0cHM6Ly9yYXcuZ2l0aHVidXNlcmNvbnRlbnQuY29tL3B1ZmZ5Y2lkL2FydGVtaXMtYXBpL21hc3Rlci9zcmMvd2luZG93cy9zaGVsbGJhZ3MudHMKZnVuY3Rpb24gZ2V0QWx0U2hlbGxiYWdzKHJlc29sdmVfZ3VpZHMsIGRyaXZlKSB7CiAgY29uc3QgZGF0YSA9IERlbm8uY29yZS5vcHMuZ2V0X2FsdF9zaGVsbGJhZ3MoCiAgICByZXNvbHZlX2d1aWRzLAogICAgZHJpdmUKICApOwogIGNvbnN0IHJlc3VsdCA9IEpTT04ucGFyc2UoZGF0YSk7CiAgcmV0dXJuIHJlc3VsdDsKfQoKLy8gbWFpbi50cwpmdW5jdGlvbiBtYWluKCkgewogIGNvbnN0IHBhdGhzID0gZ2xvYigiQzpcXFVzZXJzXFwqXFxOVFVTRVIuREFUIik7CiAgaWYgKHBhdGhzIGluc3RhbmNlb2YgRXJyb3IpIHsKICAgIHJldHVybiBbXTsKICB9CiAgZm9yIChjb25zdCBwYXRoIG9mIHBhdGhzKSB7CiAgICBjb25zdCByZXNvbHZlX2d1aWRzID0gdHJ1ZTsKICAgIGNvbnN0IGJhZ3MgPSBnZXRBbHRTaGVsbGJhZ3MocmVzb2x2ZV9ndWlkcywgcGF0aC5mdWxsX3BhdGgpOwogICAgcmV0dXJuIGJhZ3M7CiAgfQogIHJldHVybiBbXTsKfQptYWluKCk7";
        let mut output = output_options("runtime_test", "local", "./tmp", false);
        let script = JSScript {
            name: String::from("shellbags_alt"),
            script: test.to_string(),
        };
        execute_script(&mut output, &script).unwrap();
    }
}
