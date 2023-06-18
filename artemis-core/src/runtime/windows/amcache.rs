use crate::{
    artifacts::os::windows::amcache::parser::grab_amcache, runtime::error::RuntimeError,
    structs::artifacts::os::windows::AmcacheOptions,
};
use deno_core::{error::AnyError, op};
use log::error;

#[op]
/// Expose parsing amcache located on systemdrive to `Deno`
fn get_amcache() -> Result<String, AnyError> {
    let options = AmcacheOptions { alt_drive: None };
    let amcache_results = grab_amcache(&options);
    let amcache = match amcache_results {
        Ok(results) => results,
        Err(err) => {
            error!("[runtime] Failed to parse amcache: {err:?}");
            return Err(RuntimeError::ExecuteScript.into());
        }
    };

    let results = serde_json::to_string_pretty(&amcache)?;
    Ok(results)
}

#[op]
/// Expose parsing amcache located on alt drive to `Deno`
fn get_alt_amcache(drive: String) -> Result<String, AnyError> {
    if drive.is_empty() {
        error!("[runtime] Failed to parse alt amcache drive. Need drive letter");
        return Err(RuntimeError::ExecuteScript.into());
    }
    // Get the first char from string (the drive letter)
    let drive_char = &drive.chars().next().unwrap();
    let options = AmcacheOptions {
        alt_drive: Some(drive_char.to_owned()),
    };

    let amcache_results = grab_amcache(&options);
    let amcache = match amcache_results {
        Ok(results) => results,
        Err(err) => {
            error!("[runtime] Failed to parse alt amcache: {err:?}");
            return Err(RuntimeError::ExecuteScript.into());
        }
    };

    let results = serde_json::to_string_pretty(&amcache)?;
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
        }
    }

    #[test]
    fn test_get_amcache() {
        let test = "Ly8gZGVuby1mbXQtaWdub3JlLWZpbGUKLy8gZGVuby1saW50LWlnbm9yZS1maWxlCi8vIFRoaXMgY29kZSB3YXMgYnVuZGxlZCB1c2luZyBgZGVubyBidW5kbGVgIGFuZCBpdCdzIG5vdCByZWNvbW1lbmRlZCB0byBlZGl0IGl0IG1hbnVhbGx5CgpmdW5jdGlvbiBnZXRfYW1jYWNoZSgpIHsKICAgIGNvbnN0IGRhdGEgPSBEZW5vW0Rlbm8uaW50ZXJuYWxdLmNvcmUub3BzLmdldF9hbWNhY2hlKCk7CiAgICBjb25zdCBhbWNhY2hlX2FycmF5ID0gSlNPTi5wYXJzZShkYXRhKTsKICAgIHJldHVybiBhbWNhY2hlX2FycmF5Owp9CmZ1bmN0aW9uIGdldEFtY2FjaGUoKSB7CiAgICByZXR1cm4gZ2V0X2FtY2FjaGUoKTsKfQpmdW5jdGlvbiBtYWluKCkgewogICAgY29uc3QgY2FjaGUgPSBnZXRBbWNhY2hlKCk7CiAgICByZXR1cm4gY2FjaGU7Cn0KbWFpbigpOwoK";
        let mut output = output_options("runtime_test", "local", "./tmp", false);
        let script = JSScript {
            name: String::from("amcache"),
            script: test.to_string(),
        };
        execute_script(&mut output, &script).unwrap();
    }

    #[test]
    fn test_get_alt_amcache() {
        let test = "Ly8gZGVuby1mbXQtaWdub3JlLWZpbGUKLy8gZGVuby1saW50LWlnbm9yZS1maWxlCi8vIFRoaXMgY29kZSB3YXMgYnVuZGxlZCB1c2luZyBgZGVubyBidW5kbGVgIGFuZCBpdCdzIG5vdCByZWNvbW1lbmRlZCB0byBlZGl0IGl0IG1hbnVhbGx5CgpmdW5jdGlvbiBnZXRfYWx0X2FtY2FjaGUoZHJpdmUpIHsKICAgIGNvbnN0IGRhdGEgPSBEZW5vW0Rlbm8uaW50ZXJuYWxdLmNvcmUub3BzLmdldF9hbHRfYW1jYWNoZShkcml2ZSk7CiAgICBjb25zdCBhbWNhY2hlX2FycmF5ID0gSlNPTi5wYXJzZShkYXRhKTsKICAgIHJldHVybiBhbWNhY2hlX2FycmF5Owp9CmZ1bmN0aW9uIGdldEFsdEFtY2FjaGUoZHJpdmUpIHsKICAgIHJldHVybiBnZXRfYWx0X2FtY2FjaGUoZHJpdmUpOwp9CmZ1bmN0aW9uIG1haW4oKSB7CiAgICBjb25zdCBjYWNoZSA9IGdldEFsdEFtY2FjaGUoIkMiKTsKICAgIHJldHVybiBjYWNoZTsKfQptYWluKCk7Cgo=";
        let mut output = output_options("runtime_test", "local", "./tmp", false);
        let script = JSScript {
            name: String::from("amcache_alt"),
            script: test.to_string(),
        };
        execute_script(&mut output, &script).unwrap();
    }
}
