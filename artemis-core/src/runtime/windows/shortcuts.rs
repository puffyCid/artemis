use crate::{
    artifacts::os::windows::shortcuts::parser::grab_lnk_file, runtime::error::RuntimeError,
};
use deno_core::{error::AnyError, op};
use log::error;

#[op]
fn get_lnk_file(path: String) -> Result<String, AnyError> {
    let lnk_result = grab_lnk_file(&path);
    let lnk = match lnk_result {
        Ok(results) => results,
        Err(err) => {
            error!("[runtime] Failed to parse shortcut file {path}: {err:?}");
            return Err(RuntimeError::ExecuteScript.into());
        }
    };

    let results = serde_json::to_string(&lnk)?;
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
    fn test_get_lnk_file() {
        let test = "Ly8gaHR0cHM6Ly9yYXcuZ2l0aHVidXNlcmNvbnRlbnQuY29tL3B1ZmZ5Y2lkL2FydGVtaXMtYXBpL21hc3Rlci9zcmMvd2luZG93cy9zaG9ydGN1dHMudHMKZnVuY3Rpb24gZ2V0TG5rRmlsZShwYXRoKSB7CiAgY29uc3QgZGF0YSA9IERlbm8uY29yZS5vcHMuZ2V0X2xua19maWxlKHBhdGgpOwogIGNvbnN0IHJlc3VsdHMgPSBKU09OLnBhcnNlKGRhdGEpOwogIHJldHVybiByZXN1bHRzOwp9CgovLyBodHRwczovL3Jhdy5naXRodWJ1c2VyY29udGVudC5jb20vcHVmZnljaWQvYXJ0ZW1pcy1hcGkvbWFzdGVyL3NyYy9lbnZpcm9ubWVudC9lbnYudHMKZnVuY3Rpb24gZ2V0RW52VmFsdWUoa2V5KSB7CiAgY29uc3QgZGF0YSA9IGVudi5lbnZpcm9ubWVudFZhbHVlKGtleSk7CiAgcmV0dXJuIGRhdGE7Cn0KCi8vIGh0dHBzOi8vcmF3LmdpdGh1YnVzZXJjb250ZW50LmNvbS9wdWZmeWNpZC9hcnRlbWlzLWFwaS9tYXN0ZXIvc3JjL2ZpbGVzeXN0ZW0vZGlyZWN0b3J5LnRzCmFzeW5jIGZ1bmN0aW9uIHJlYWREaXIocGF0aCkgewogIGNvbnN0IGRhdGEgPSBKU09OLnBhcnNlKGF3YWl0IGZzLnJlYWREaXIocGF0aCkpOwogIHJldHVybiBkYXRhOwp9CgovLyBtYWluLnRzCmFzeW5jIGZ1bmN0aW9uIG1haW4oKSB7CiAgY29uc3QgZHJpdmUgPSBnZXRFbnZWYWx1ZSgiU3lzdGVtRHJpdmUiKTsKICBpZiAoZHJpdmUgPT09ICIiKSB7CiAgICByZXR1cm4gW107CiAgfQogIGNvbnN0IHVzZXJzID0gYCR7ZHJpdmV9XFxVc2Vyc2A7CiAgY29uc3QgcmVjZW50X2ZpbGVzID0gW107CiAgY29uc3QgcmVzdWx0cyA9IGF3YWl0IHJlYWREaXIodXNlcnMpCiAgZm9yIChjb25zdCBlbnRyeSBvZiByZXN1bHRzKSB7CiAgICB0cnkgewogICAgICBjb25zdCBwYXRoID0gYCR7dXNlcnN9XFwke2VudHJ5LmZpbGVuYW1lfVxcQXBwRGF0YVxcUm9hbWluZ1xcTWljcm9zb2Z0XFxXaW5kb3dzXFxSZWNlbnRgOwogICAgICBjb25zdCByZXN1bHRzMiA9IGF3YWl0IHJlYWREaXIocGF0aCk7CiAgICAgIGZvciAoY29uc3QgZW50cnkyIG9mIHJlc3VsdHMyKSB7CiAgICAgICAgaWYgKCFlbnRyeTIuZmlsZW5hbWUuZW5kc1dpdGgoImxuayIpKSB7CiAgICAgICAgICBjb250aW51ZTsKICAgICAgICB9CiAgICAgICAgY29uc3QgbG5rX2ZpbGUgPSBgJHtwYXRofVxcJHtlbnRyeTIuZmlsZW5hbWV9YDsKICAgICAgICBjb25zdCBsbmsgPSBnZXRMbmtGaWxlKGxua19maWxlKTsKICAgICAgICByZWNlbnRfZmlsZXMucHVzaChsbmspOwogICAgICB9CiAgICB9IGNhdGNoIChfZXJyb3IpIHsKICAgICAgY29udGludWU7CiAgICB9CiAgfQogIHJldHVybiByZWNlbnRfZmlsZXM7Cn0KbWFpbigpOwo=";
        let mut output = output_options("runtime_test", "local", "./tmp", false);
        let script = JSScript {
            name: String::from("recent_files"),
            script: test.to_string(),
        };
        execute_script(&mut output, &script).unwrap();
    }
}
