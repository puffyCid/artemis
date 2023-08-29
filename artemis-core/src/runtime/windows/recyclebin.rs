use crate::{
    artifacts::os::windows::recyclebin::parser::{grab_recycle_bin, grab_recycle_bin_path},
    runtime::error::RuntimeError,
    structs::artifacts::os::windows::RecycleBinOptions,
};
use deno_core::{error::AnyError, op};
use log::error;

#[op]
/// Expose parsing Recycle Bin at default systemdrive to Deno
fn get_recycle_bin() -> Result<String, AnyError> {
    let options = RecycleBinOptions { alt_drive: None };
    let bin_result = grab_recycle_bin(&options);
    let bin = match bin_result {
        Ok(results) => results,
        Err(err) => {
            error!("[runtime] Failed to parse recycle bin at default path: {err:?}");
            return Err(RuntimeError::ExecuteScript.into());
        }
    };

    let results = serde_json::to_string(&bin)?;
    Ok(results)
}

#[op]
/// Expose parsing Recycle Bin at alternative drive to Deno
fn get_alt_recycle_bin(drive: String) -> Result<String, AnyError> {
    if drive.is_empty() {
        error!("[runtime] Failed to parse alt recycle bin drive. Need drive letter");
        return Err(RuntimeError::ExecuteScript.into());
    }
    // Get the first char from string (the drive letter)
    let drive_char = drive.chars().next().unwrap();
    let options = RecycleBinOptions {
        alt_drive: Some(drive_char),
    };

    let bin_result = grab_recycle_bin(&options);
    let bin = match bin_result {
        Ok(results) => results,
        Err(err) => {
            error!("[runtime] Failed to parse recycle bin at alt drive {drive}: {err:?}");
            return Err(RuntimeError::ExecuteScript.into());
        }
    };

    let results = serde_json::to_string(&bin)?;
    Ok(results)
}

#[op]
/// Expose parsing Recycle Bin file to Deno
fn get_recycle_bin_file(path: String) -> Result<String, AnyError> {
    if path.is_empty() {
        error!("[runtime] Got empty recycle bin file arguement.");
        return Err(RuntimeError::ExecuteScript.into());
    }

    let bin_result = grab_recycle_bin_path(&path);
    let bin = match bin_result {
        Ok(results) => results,
        Err(err) => {
            error!("[runtime] Failed to parse recycle bin file at path {path}: {err:?}");
            return Err(RuntimeError::ExecuteScript.into());
        }
    };

    let results = serde_json::to_string(&bin)?;

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
    fn test_get_recycle_bin() {
        let test = "Ly8gaHR0cHM6Ly9yYXcuZ2l0aHVidXNlcmNvbnRlbnQuY29tL3B1ZmZ5Y2lkL2FydGVtaXMtYXBpL21hc3Rlci9zcmMvd2luZG93cy9yZWN5Y2xlYmluLnRzCmZ1bmN0aW9uIGdldFJlY3ljbGVCaW4oZHJpdmUpIHsKICBpZiAoZHJpdmUgPT09IHZvaWQgMCkgewogICAgY29uc3QgZGF0YTIgPSBEZW5vLmNvcmUub3BzLmdldF9yZWN5Y2xlX2JpbigpOwogICAgY29uc3QgYmluMiA9IEpTT04ucGFyc2UoZGF0YTIpOwogICAgcmV0dXJuIGJpbjI7CiAgfQogIGNvbnN0IGRhdGEgPSBEZW5vLmNvcmUub3BzLmdldF9hbHRfcmVjeWNsZV9iaW4oZHJpdmUpOwogIGNvbnN0IGJpbiA9IEpTT04ucGFyc2UoZGF0YSk7CiAgcmV0dXJuIGJpbjsKfQoKLy8gbWFpbi50cwpmdW5jdGlvbiBtYWluKCkgewogIGNvbnN0IGJpbiA9IGdldFJlY3ljbGVCaW4oKTsKICByZXR1cm4gYmluOwp9Cm1haW4oKTs=";
        let mut output = output_options("runtime_test", "local", "./tmp", false);
        let script = JSScript {
            name: String::from("recycle_bin_default"),
            script: test.to_string(),
        };
        execute_script(&mut output, &script).unwrap();
    }

    #[test]
    fn test_get_alt_recycle_bin() {
        let test = "Ly8gaHR0cHM6Ly9yYXcuZ2l0aHVidXNlcmNvbnRlbnQuY29tL3B1ZmZ5Y2lkL2FydGVtaXMtYXBpL21hc3Rlci9zcmMvd2luZG93cy9yZWN5Y2xlYmluLnRzCmZ1bmN0aW9uIGdldFJlY3ljbGVCaW4oZHJpdmUpIHsKICBpZiAoZHJpdmUgPT09IHZvaWQgMCkgewogICAgY29uc3QgZGF0YTIgPSBEZW5vLmNvcmUub3BzLmdldF9yZWN5Y2xlX2JpbigpOwogICAgY29uc3QgYmluMiA9IEpTT04ucGFyc2UoZGF0YTIpOwogICAgcmV0dXJuIGJpbjI7CiAgfQogIGNvbnN0IGRhdGEgPSBEZW5vLmNvcmUub3BzLmdldF9hbHRfcmVjeWNsZV9iaW4oZHJpdmUpOwogIGNvbnN0IGJpbiA9IEpTT04ucGFyc2UoZGF0YSk7CiAgcmV0dXJuIGJpbjsKfQoKLy8gbWFpbi50cwpmdW5jdGlvbiBtYWluKCkgewogIGNvbnN0IGJpbiA9IGdldFJlY3ljbGVCaW4oIkMiKTsKICByZXR1cm4gYmluOwp9Cm1haW4oKTs=";
        let mut output = output_options("runtime_test", "local", "./tmp", false);
        let script = JSScript {
            name: String::from("recycle_bin_alt"),
            script: test.to_string(),
        };
        execute_script(&mut output, &script).unwrap();
    }

    #[test]
    fn test_get_recycle_bin_file() {
        let test = "Ly8gaHR0cHM6Ly9yYXcuZ2l0aHVidXNlcmNvbnRlbnQuY29tL3B1ZmZ5Y2lkL2FydGVtaXMtYXBpL21haW4vc3JjL2ZpbGVzeXN0ZW0vZmlsZXMudHMKZnVuY3Rpb24gZ2xvYihwYXR0ZXJuKSB7CiAgY29uc3QgZGF0YSA9IGZzLmdsb2IocGF0dGVybik7CiAgY29uc3QgcmVzdWx0ID0gSlNPTi5wYXJzZShkYXRhKTsKICByZXR1cm4gcmVzdWx0Owp9CgovLyBodHRwczovL3Jhdy5naXRodWJ1c2VyY29udGVudC5jb20vcHVmZnljaWQvYXJ0ZW1pcy1hcGkvbWFzdGVyL3NyYy93aW5kb3dzL3JlY3ljbGViaW4udHMKZnVuY3Rpb24gZ2V0UmVjeWNsZUJpbkZpbGUocGF0aCkgewogIGNvbnN0IGRhdGEgPSBEZW5vLmNvcmUub3BzLmdldF9yZWN5Y2xlX2Jpbl9maWxlKHBhdGgpOwogIGNvbnN0IGJpbiA9IEpTT04ucGFyc2UoZGF0YSk7CiAgcmV0dXJuIGJpbjsKfQoKLy8gbWFpbi50cwpmdW5jdGlvbiBtYWluKCkgewogIGNvbnN0IHBhdGhzID0gZ2xvYigiQzpcXCRSRUNZQ0xFLkJJTlxcKlxcJEkqIik7CiAgaWYgKHBhdGhzIGluc3RhbmNlb2YgRXJyb3IpIHsKICAgIHJldHVybjsKICB9CiAgZm9yIChjb25zdCBwYXRoIG9mIHBhdGhzKSB7CiAgICBjb25zdCBkYXRhID0gZ2V0UmVjeWNsZUJpbkZpbGUocGF0aC5mdWxsX3BhdGgpOwogICAgcmV0dXJuIGRhdGE7CiAgfQp9Cm1haW4oKTs=";
        let mut output = output_options("runtime_test", "local", "./tmp", false);
        let script = JSScript {
            name: String::from("recycle_bin_path"),
            script: test.to_string(),
        };
        execute_script(&mut output, &script).unwrap();
    }
}
