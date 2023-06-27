use crate::{
    artifacts::os::windows::usnjrnl::parser::grab_usnjrnl, runtime::error::RuntimeError,
    structs::artifacts::os::windows::UsnJrnlOptions,
};
use deno_core::{error::AnyError, op};
use log::error;

#[op]
/// Expose parsing usnjrnl located on systemdrive to `Deno`
fn get_usnjrnl() -> Result<String, AnyError> {
    let options = UsnJrnlOptions { alt_drive: None };
    let jrnl_results = grab_usnjrnl(&options);
    let jrnl = match jrnl_results {
        Ok(results) => results,
        Err(err) => {
            error!("[runtime] Failed to parse usnjrnl: {err:?}");
            return Err(RuntimeError::ExecuteScript.into());
        }
    };

    let results = serde_json::to_string_pretty(&jrnl)?;
    Ok(results)
}

#[op]
/// Expose parsing usnjrnl located on alt drive to `Deno`
fn get_alt_usnjrnl(drive: String) -> Result<String, AnyError> {
    if drive.is_empty() {
        error!("[runtime] Failed to parse alt usnjrnl drive. Need drive letter");
        return Err(RuntimeError::ExecuteScript.into());
    }
    // Get the first char from string (the drive letter)
    let drive_char = &drive.chars().next().unwrap();
    let options = UsnJrnlOptions {
        alt_drive: Some(drive_char.to_owned()),
    };

    let jrnl_results = grab_usnjrnl(&options);
    let jrnl = match jrnl_results {
        Ok(results) => results,
        Err(err) => {
            error!("[runtime] Failed to parse usnjrnl: {err:?}");
            return Err(RuntimeError::ExecuteScript.into());
        }
    };

    let results = serde_json::to_string_pretty(&jrnl)?;
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
    #[ignore = "Parses the whole USNJrnl for rs files"]
    fn test_get_usnjrnl_rs_files() {
        let test = "Ly8gZGVuby1mbXQtaWdub3JlLWZpbGUKLy8gZGVuby1saW50LWlnbm9yZS1maWxlCi8vIFRoaXMgY29kZSB3YXMgYnVuZGxlZCB1c2luZyBgZGVubyBidW5kbGVgIGFuZCBpdCdzIG5vdCByZWNvbW1lbmRlZCB0byBlZGl0IGl0IG1hbnVhbGx5CgpmdW5jdGlvbiBnZXRfdXNuanJubCgpIHsKICAgIGNvbnN0IGRhdGEgPSBEZW5vW0Rlbm8uaW50ZXJuYWxdLmNvcmUub3BzLmdldF91c25qcm5sKCk7CiAgICBjb25zdCBqcm5sX2FycmF5ID0gSlNPTi5wYXJzZShkYXRhKTsKICAgIHJldHVybiBqcm5sX2FycmF5Owp9CmZ1bmN0aW9uIGdldFVzbkpybmwoKSB7CiAgICByZXR1cm4gZ2V0X3VzbmpybmwoKTsKfQpmdW5jdGlvbiBtYWluKCkgewogICAgY29uc3QganJubF9lbnRyaWVzID0gZ2V0VXNuSnJubCgpOwogICAgY29uc3QgcnNfZW50cmllcyA9IFtdOwogICAgZm9yKGxldCBlbnRyeSA9IDA7IGVudHJ5IDwganJubF9lbnRyaWVzLmxlbmd0aDsgZW50cnkrKyl7CiAgICAgICAgaWYgKGpybmxfZW50cmllc1tlbnRyeV0uZXh0ZW5zaW9uID09PSAicnMiKSB7CiAgICAgICAgICAgIHJzX2VudHJpZXMucHVzaChqcm5sX2VudHJpZXNbZW50cnldKTsKICAgICAgICB9CiAgICB9CiAgICByZXR1cm4gcnNfZW50cmllczsKfQptYWluKCk7Cgo=";
        let mut output = output_options("runtime_test", "local", "./tmp", false);
        let script = JSScript {
            name: String::from("usnjnl_rs_files"),
            script: test.to_string(),
        };
        execute_script(&mut output, &script).unwrap();
    }

    #[test]
    #[ignore = "Parses the whole USNJrnl"]
    fn test_get_alt_usnjrnl() {
        let test = "Ly8gZGVuby1mbXQtaWdub3JlLWZpbGUKLy8gZGVuby1saW50LWlnbm9yZS1maWxlCi8vIFRoaXMgY29kZSB3YXMgYnVuZGxlZCB1c2luZyBgZGVubyBidW5kbGVgIGFuZCBpdCdzIG5vdCByZWNvbW1lbmRlZCB0byBlZGl0IGl0IG1hbnVhbGx5CgpmdW5jdGlvbiBnZXRfYWx0X3VzbmpybmwoZHJpdmUpIHsKICAgIGNvbnN0IGRhdGEgPSBEZW5vW0Rlbm8uaW50ZXJuYWxdLmNvcmUub3BzLmdldF9hbHRfdXNuanJubChkcml2ZSk7CiAgICBjb25zdCBqcm5sX2FycmF5ID0gSlNPTi5wYXJzZShkYXRhKTsKICAgIHJldHVybiBqcm5sX2FycmF5Owp9CmZ1bmN0aW9uIGdldEFsdFVzbkpybmwoZHJpdmUpIHsKICAgIHJldHVybiBnZXRfYWx0X3VzbmpybmwoZHJpdmUpOwp9CmZ1bmN0aW9uIG1haW4oKSB7CiAgICBjb25zdCBqcm5sX2VudHJpZXMgPSBnZXRBbHRVc25Kcm5sKCJDIik7CiAgICByZXR1cm4ganJubF9lbnRyaWVzIDsKfQptYWluKCk7";
        let mut output = output_options("runtime_test", "local", "./tmp", false);
        let script = JSScript {
            name: String::from("usnjrnl_alt"),
            script: test.to_string(),
        };
        execute_script(&mut output, &script).unwrap();
    }
}
