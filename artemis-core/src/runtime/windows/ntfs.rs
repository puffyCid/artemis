use crate::{
    filesystem::ntfs::raw_files::{raw_read_file, read_attribute},
    runtime::error::RuntimeError,
    utils::encoding::base64_encode_standard,
};
use deno_core::{error::AnyError, op};
use log::error;

#[op]
/// Expose reading a raw file to `Deno`
fn read_raw_file(path: String) -> Result<String, AnyError> {
    let data_result = raw_read_file(&path);
    let data = match data_result {
        Ok(results) => results,
        Err(err) => {
            error!("[runtime] Failed to read file {path}: {err:?}");
            return Err(RuntimeError::ExecuteScript.into());
        }
    };
    // We return a base64 string to play nice with Deno/V8 limitations on large byte arrays
    Ok(base64_encode_standard(&data))
}

#[op]
/// Expose reading an alternative data stream (ADS) to `Deno`
fn read_ads_data(path: String, ads_name: String) -> Result<String, AnyError> {
    let data_result = read_attribute(&path, &ads_name);
    let data = match data_result {
        Ok(results) => results,
        Err(err) => {
            error!("[runtime] Failed to ADS data at {path}: {err:?}");
            return Err(RuntimeError::ExecuteScript.into());
        }
    };
    // We return a base64 string to play nice with Deno/V8 limitations on large byte arrays
    Ok(base64_encode_standard(&data))
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
    fn test_read_ads_data_motw() {
        let test = "Ly8gaHR0cHM6Ly9yYXcuZ2l0aHVidXNlcmNvbnRlbnQuY29tL3B1ZmZ5Y2lkL2FydGVtaXMtYXBpL21hc3Rlci9zcmMvZW5jb2RpbmcvYmFzZTY0LnRzCmZ1bmN0aW9uIGRlY29kZShiNjQpIHsKICBjb25zdCBieXRlcyA9IGVuY29kaW5nLmF0b2IoYjY0KTsKICByZXR1cm4gYnl0ZXM7Cn0KCi8vIGh0dHBzOi8vcmF3LmdpdGh1YnVzZXJjb250ZW50LmNvbS9wdWZmeWNpZC9hcnRlbWlzLWFwaS9tYXN0ZXIvc3JjL3dpbmRvd3MvbnRmcy50cwpmdW5jdGlvbiByZWFkQWRzRGF0YShwYXRoLCBhZHNfbmFtZSkgewogIGNvbnN0IGRhdGEgPSBEZW5vLmNvcmUub3BzLnJlYWRfYWRzX2RhdGEoCiAgICBwYXRoLAogICAgYWRzX25hbWUKICApOwogIGlmIChkYXRhID09PSAiIikgewogICAgcmV0dXJuIG5ldyBVaW50OEFycmF5KCk7CiAgfQogIHJldHVybiBkZWNvZGUoZGF0YSk7Cn0KCi8vIGh0dHBzOi8vcmF3LmdpdGh1YnVzZXJjb250ZW50LmNvbS9wdWZmeWNpZC9hcnRlbWlzLWFwaS9tYXN0ZXIvc3JjL2ZpbGVzeXN0ZW0vZGlyZWN0b3J5LnRzCmFzeW5jIGZ1bmN0aW9uIHJlYWREaXIocGF0aCkgewogIGNvbnN0IGRhdGEgPSBKU09OLnBhcnNlKGF3YWl0IGZzLnJlYWREaXIocGF0aCkpOwogIHJldHVybiBkYXRhOwp9CgovLyBodHRwczovL3Jhdy5naXRodWJ1c2VyY29udGVudC5jb20vcHVmZnljaWQvYXJ0ZW1pcy1hcGkvbWFzdGVyL3NyYy9maWxlc3lzdGVtL2ZpbGVzLnRzCmZ1bmN0aW9uIHN0YXQocGF0aCkgewogIGNvbnN0IGRhdGEgPSBmcy5zdGF0KHBhdGgpOwogIGNvbnN0IHZhbHVlID0gSlNPTi5wYXJzZShkYXRhKTsKICByZXR1cm4gdmFsdWU7Cn0KCi8vIGh0dHBzOi8vcmF3LmdpdGh1YnVzZXJjb250ZW50LmNvbS9wdWZmeWNpZC9hcnRlbWlzLWFwaS9tYXN0ZXIvc3JjL2Vudmlyb25tZW50L2Vudi50cwpmdW5jdGlvbiBnZXRFbnZWYWx1ZShrZXkpIHsKICBjb25zdCBkYXRhID0gZW52LmVudmlyb25tZW50VmFsdWUoa2V5KTsKICByZXR1cm4gZGF0YTsKfQoKLy8gaHR0cHM6Ly9yYXcuZ2l0aHVidXNlcmNvbnRlbnQuY29tL3B1ZmZ5Y2lkL2FydGVtaXMtYXBpL21hc3Rlci9zcmMvZW5jb2Rpbmcvc3RyaW5ncy50cwpmdW5jdGlvbiBleHRyYWN0VXRmOFN0cmluZyhkYXRhKSB7CiAgY29uc3QgcmVzdWx0ID0gZW5jb2RpbmcuZXh0cmFjdF91dGY4X3N0cmluZyhkYXRhKTsKICByZXR1cm4gcmVzdWx0Owp9CgovLyBtYWluLnRzCmFzeW5jIGZ1bmN0aW9uIG1haW4oKSB7CiAgY29uc3QgZHJpdmUgPSBnZXRFbnZWYWx1ZSgiU3lzdGVtRHJpdmUiKTsKICBpZiAoZHJpdmUgPT09ICIiKSB7CiAgICByZXR1cm4gW107CiAgfQogIGNvbnN0IHdlYl9maWxlcyA9IFtdOwogIGNvbnN0IHVzZXJzID0gYCR7ZHJpdmV9XFxVc2Vyc2A7CiAgZm9yIChjb25zdCBlbnRyeSBvZiBhd2FpdCByZWFkRGlyKHVzZXJzKSkgewogICAgdHJ5IHsKICAgICAgY29uc3QgcGF0aCA9IGAke3VzZXJzfVxcJHtlbnRyeS5maWxlbmFtZX1cXERvd25sb2Fkc2A7CiAgICAgIGZvciAoY29uc3QgZmlsZV9lbnRyeSBvZiBhd2FpdCByZWFkRGlyKHBhdGgpKSB7CiAgICAgICAgdHJ5IHsKICAgICAgICAgIGlmICghZmlsZV9lbnRyeS5pc19maWxlKSB7CiAgICAgICAgICAgIGNvbnRpbnVlOwogICAgICAgICAgfQogICAgICAgICAgY29uc3QgZnVsbF9wYXRoID0gYCR7cGF0aH1cXCR7ZmlsZV9lbnRyeS5maWxlbmFtZX1gOwogICAgICAgICAgY29uc3QgYWRzID0gIlpvbmUuSWRlbnRpZmllciI7CiAgICAgICAgICBjb25zdCBkYXRhID0gcmVhZEFkc0RhdGEoZnVsbF9wYXRoLCBhZHMpOwogICAgICAgICAgaWYgKGRhdGEubGVuZ3RoID09PSAwKSB7CiAgICAgICAgICAgIGNvbnRpbnVlOwogICAgICAgICAgfQogICAgICAgICAgY29uc3QgaW5mbyA9IHN0YXQoZnVsbF9wYXRoKTsKICAgICAgICAgIGNvbnN0IG1hcmtfaW5mbyA9IGV4dHJhY3RVdGY4U3RyaW5nKGRhdGEpOwogICAgICAgICAgY29uc3Qgd2ViX2ZpbGUgPSB7CiAgICAgICAgICAgIG1hcms6IG1hcmtfaW5mbywKICAgICAgICAgICAgcGF0aDogZnVsbF9wYXRoLAogICAgICAgICAgICBjcmVhdGVkOiBpbmZvLmNyZWF0ZWQsCiAgICAgICAgICAgIG1vZGlmaWVkOiBpbmZvLm1vZGlmaWVkLAogICAgICAgICAgICBhY2Nlc3NlZDogaW5mby5hY2Nlc3NlZCwKICAgICAgICAgICAgc2l6ZTogaW5mby5zaXplCiAgICAgICAgICB9OwogICAgICAgICAgd2ViX2ZpbGVzLnB1c2god2ViX2ZpbGUpOwogICAgICAgIH0gY2F0Y2ggKF9lcnJvcikgewogICAgICAgICAgY29udGludWU7CiAgICAgICAgfQogICAgICB9CiAgICB9IGNhdGNoIChfZXJyb3IpIHsKICAgICAgY29udGludWU7CiAgICB9CiAgfQogIHJldHVybiB3ZWJfZmlsZXM7Cn0KbWFpbigpOwo=";
        let mut output = output_options("runtime_test", "local", "./tmp", false);
        let script = JSScript {
            name: String::from("read_ads_motw"),
            script: test.to_string(),
        };
        execute_script(&mut output, &script).unwrap();
    }

    #[test]
    fn test_read_raw_file_swapfile() {
        let test = "Ly8gaHR0cHM6Ly9yYXcuZ2l0aHVidXNlcmNvbnRlbnQuY29tL3B1ZmZ5Y2lkL2FydGVtaXMtYXBpL21hc3Rlci9zcmMvZW5jb2RpbmcvYmFzZTY0LnRzCmZ1bmN0aW9uIGRlY29kZShiNjQpIHsKICBjb25zdCBieXRlcyA9IGVuY29kaW5nLmF0b2IoYjY0KTsKICByZXR1cm4gYnl0ZXM7Cn0KCi8vIGh0dHBzOi8vcmF3LmdpdGh1YnVzZXJjb250ZW50LmNvbS9wdWZmeWNpZC9hcnRlbWlzLWFwaS9tYXN0ZXIvc3JjL3dpbmRvd3MvbnRmcy50cwpmdW5jdGlvbiByZWFkUmF3RmlsZShwYXRoKSB7CiAgY29uc3QgZGF0YSA9IERlbm8uY29yZS5vcHMucmVhZF9yYXdfZmlsZShwYXRoKTsKICByZXR1cm4gZGVjb2RlKGRhdGEpOwp9CgovLyBodHRwczovL3Jhdy5naXRodWJ1c2VyY29udGVudC5jb20vcHVmZnljaWQvYXJ0ZW1pcy1hcGkvbWFzdGVyL3NyYy9lbnZpcm9ubWVudC9lbnYudHMKZnVuY3Rpb24gZ2V0RW52VmFsdWUoa2V5KSB7CiAgY29uc3QgZGF0YSA9IGVudi5lbnZpcm9ubWVudFZhbHVlKGtleSk7CiAgcmV0dXJuIGRhdGE7Cn0KCi8vIGh0dHBzOi8vcmF3LmdpdGh1YnVzZXJjb250ZW50LmNvbS9wdWZmeWNpZC9hcnRlbWlzLWFwaS9tYXN0ZXIvc3JjL2ZpbGVzeXN0ZW0vZmlsZXMudHMKZnVuY3Rpb24gc3RhdChwYXRoKSB7CiAgY29uc3QgZGF0YSA9IGZzLnN0YXQocGF0aCk7CiAgY29uc3QgdmFsdWUgPSBKU09OLnBhcnNlKGRhdGEpOwogIHJldHVybiB2YWx1ZTsKfQoKLy8gbWFpbi50cwpmdW5jdGlvbiBtYWluKCkgewogIGNvbnN0IGRyaXZlID0gZ2V0RW52VmFsdWUoIlN5c3RlbURyaXZlIik7CiAgaWYgKGRyaXZlID09PSAiIikgewogICAgcmV0dXJuIDA7CiAgfQogIHRyeSB7CiAgICBjb25zdCBzd2FwID0gYCR7ZHJpdmV9XFxzd2FwZmlsZS5zeXNgOwogICAgY29uc3QgaW5mbyA9IHN0YXQoc3dhcCk7CiAgICBpZiAoIWluZm8uaXNfZmlsZSkgewogICAgICByZXR1cm4gMDsKICAgIH0KICAgIGNvbnN0IG1heF9zaXplID0gMjE0NzQ4MzY0ODsKICAgIGlmIChpbmZvLnNpemUgPiBtYXhfc2l6ZSkgewogICAgICByZXR1cm4gMDsKICAgIH0KICAgIGNvbnN0IGRhdGEgPSByZWFkUmF3RmlsZShzd2FwKTsKICAgIHJldHVybiBkYXRhLmxlbmd0aDsKICB9IGNhdGNoIChfZXJyb3IpIHsKICAgIHJldHVybiAwOwogIH0KfQptYWluKCk7Cg==";
        let mut output = output_options("runtime_test", "local", "./tmp", false);
        let script = JSScript {
            name: String::from("swapfile"),
            script: test.to_string(),
        };
        execute_script(&mut output, &script).unwrap();
    }
}
