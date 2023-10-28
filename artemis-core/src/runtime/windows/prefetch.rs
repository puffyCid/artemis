use crate::{
    artifacts::os::windows::prefetch::parser::{custom_prefetch_path, grab_prefetch},
    runtime::error::RuntimeError,
    structs::artifacts::os::windows::PrefetchOptions,
};
use deno_core::{error::AnyError, op2};
use log::error;

#[op2]
#[string]
/// Get Prefetch files at using default drive (typically C)
pub(crate) fn get_prefetch() -> Result<String, AnyError> {
    let options = PrefetchOptions { alt_drive: None };
    let pf_result = grab_prefetch(&options);
    let pf = match pf_result {
        Ok(results) => results,
        Err(err) => {
            error!("[runtime] Failed to parse prefetch at default path: {err:?}");
            return Err(RuntimeError::ExecuteScript.into());
        }
    };

    let results = serde_json::to_string(&pf)?;
    Ok(results)
}

#[op2]
#[string]
/// Get Prefetch files using alternative drive
pub(crate) fn get_alt_prefetch(#[string] drive: String) -> Result<String, AnyError> {
    if drive.is_empty() {
        error!("[runtime] Failed to parse alt prefetch drive. Need drive letter");
        return Err(RuntimeError::ExecuteScript.into());
    }
    // Get the first char from string (the drive letter)
    let drive_char = drive.chars().next().unwrap();
    let options = PrefetchOptions {
        alt_drive: Some(drive_char),
    };

    let pf_result = grab_prefetch(&options);
    let pf = match pf_result {
        Ok(results) => results,
        Err(err) => {
            error!("[runtime] Failed to parse prefetch at alt drive {drive}: {err:?}");
            return Err(RuntimeError::ExecuteScript.into());
        }
    };

    let results = serde_json::to_string(&pf)?;
    Ok(results)
}

#[op2]
#[string]
/// Parse Prefetch files at provided directory
pub(crate) fn get_prefetch_path(#[string] path: String) -> Result<String, AnyError> {
    if path.is_empty() {
        error!("[runtime] Got empty prefetch path.");
        return Err(RuntimeError::ExecuteScript.into());
    }

    let pf_result = custom_prefetch_path(&path);
    let pf = match pf_result {
        Ok(results) => results,
        Err(err) => {
            error!("[runtime] Failed to parse prefetch at path {path}: {err:?}");
            return Err(RuntimeError::ExecuteScript.into());
        }
    };

    let results = serde_json::to_string(&pf)?;
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
    fn test_get_prefetch() {
        let test = "Ly8gZGVuby1mbXQtaWdub3JlLWZpbGUKLy8gZGVuby1saW50LWlnbm9yZS1maWxlCi8vIFRoaXMgY29kZSB3YXMgYnVuZGxlZCB1c2luZyBgZGVubyBidW5kbGVgIGFuZCBpdCdzIG5vdCByZWNvbW1lbmRlZCB0byBlZGl0IGl0IG1hbnVhbGx5CgpmdW5jdGlvbiBnZXRfcHJlZmV0Y2goKSB7CiAgICBjb25zdCBkYXRhID0gRGVuby5jb3JlLm9wcy5nZXRfcHJlZmV0Y2goKTsKICAgIGNvbnN0IHBmID0gSlNPTi5wYXJzZShkYXRhKTsKICAgIHJldHVybiBwZjsKfQpmdW5jdGlvbiBnZXRQcmVmZXRjaCgpIHsKICAgIHJldHVybiBnZXRfcHJlZmV0Y2goKTsKfQpmdW5jdGlvbiBtYWluKCkgewogICAgY29uc3QgcGYgPSBnZXRQcmVmZXRjaCgpOwogICAgcmV0dXJuIHBmOwp9Cm1haW4oKTsKCg==";
        let mut output = output_options("runtime_test", "local", "./tmp", false);
        let script = JSScript {
            name: String::from("pf_default"),
            script: test.to_string(),
        };
        execute_script(&mut output, &script).unwrap();
    }

    #[test]
    fn test_get_alt_prefetch() {
        let test = "Ly8gZGVuby1mbXQtaWdub3JlLWZpbGUKLy8gZGVuby1saW50LWlnbm9yZS1maWxlCi8vIFRoaXMgY29kZSB3YXMgYnVuZGxlZCB1c2luZyBgZGVubyBidW5kbGVgIGFuZCBpdCdzIG5vdCByZWNvbW1lbmRlZCB0byBlZGl0IGl0IG1hbnVhbGx5CgpmdW5jdGlvbiBnZXRfYWx0X3ByZWZldGNoKHBhdGgpIHsKICAgIGNvbnN0IGRhdGEgPSBEZW5vLmNvcmUub3BzLmdldF9hbHRfcHJlZmV0Y2gocGF0aCk7CiAgICBjb25zdCBwZiA9IEpTT04ucGFyc2UoZGF0YSk7CiAgICByZXR1cm4gcGY7Cn0KZnVuY3Rpb24gZ2V0QWx0UHJlZmV0Y2gocGF0aCkgewogICAgcmV0dXJuIGdldF9hbHRfcHJlZmV0Y2gocGF0aCk7Cn0KZnVuY3Rpb24gbWFpbigpIHsKICAgIGNvbnN0IHBmID0gZ2V0QWx0UHJlZmV0Y2goIkM6XFxXaW5kb3dzXFxQcmVmZXRjaCIpOwogICAgcmV0dXJuIHBmOwp9Cm1haW4oKTsKCg==";
        let mut output = output_options("runtime_test", "local", "./tmp", false);
        let script = JSScript {
            name: String::from("pf_alt"),
            script: test.to_string(),
        };
        execute_script(&mut output, &script).unwrap();
    }

    #[test]
    fn test_get_prefetch_path() {
        let test = "Ly8gZGVuby1mbXQtaWdub3JlLWZpbGUKLy8gZGVuby1saW50LWlnbm9yZS1maWxlCi8vIFRoaXMgY29kZSB3YXMgYnVuZGxlZCB1c2luZyBgZGVubyBidW5kbGVgIGFuZCBpdCdzIG5vdCByZWNvbW1lbmRlZCB0byBlZGl0IGl0IG1hbnVhbGx5CgpmdW5jdGlvbiBnZXRfcHJlZmV0Y2hfcGF0aChwYXRoKSB7CiAgICBjb25zdCBkYXRhID0gRGVuby5jb3JlLm9wcy5nZXRfcHJlZmV0Y2hfcGF0aChwYXRoKTsKICAgIGNvbnN0IHBmID0gSlNPTi5wYXJzZShkYXRhKTsKICAgIHJldHVybiBwZjsKfQpmdW5jdGlvbiBnZXRQcmVmZXRjaFBhdGgocGF0aCkgewogICAgcmV0dXJuIGdldF9wcmVmZXRjaF9wYXRoKHBhdGgpOwp9CmZ1bmN0aW9uIG1haW4oKSB7CiAgICBjb25zdCBwZiA9IGdldFByZWZldGNoUGF0aCgiQzpcXFdpbmRvd3NcXFByZWZldGNoIik7CiAgICByZXR1cm4gcGY7Cn0KbWFpbigpOwoK";
        let mut output = output_options("runtime_test", "local", "./tmp", false);
        let script = JSScript {
            name: String::from("pf_path"),
            script: test.to_string(),
        };
        execute_script(&mut output, &script).unwrap();
    }
}
