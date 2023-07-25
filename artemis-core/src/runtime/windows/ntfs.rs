use crate::{
    filesystem::ntfs::raw_files::{raw_read_file, read_attribute},
    runtime::error::RuntimeError,
};
use deno_core::{error::AnyError, op, ByteString};
use log::error;

#[op]
/// Expose reading a raw file to `Deno`
fn read_raw_file(path: String) -> Result<ByteString, AnyError> {
    let data_result = raw_read_file(&path);
    let data = match data_result {
        Ok(results) => results,
        Err(err) => {
            error!("[runtime] Failed to read file {path}: {err:?}");
            return Err(RuntimeError::ExecuteScript.into());
        }
    };
    Ok(data.into())
}

#[op]
/// Expose reading an alternative data stream (ADS) to `Deno`
fn read_ads_data(path: String, ads_name: String) -> Result<ByteString, AnyError> {
    let data_result = read_attribute(&path, &ads_name);
    let data = match data_result {
        Ok(results) => results,
        Err(err) => {
            error!("[runtime] Failed to ADS data at {path}: {err:?}");
            return Err(RuntimeError::ExecuteScript.into());
        }
    };

    Ok(data.into())
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
        let test = "Ly8gaHR0cHM6Ly9yYXcuZ2l0aHVidXNlcmNvbnRlbnQuY29tL3B1ZmZ5Y2lkL2FydGVtaXMtYXBpL21hc3Rlci9zcmMvZW5jb2RpbmcvYmFzZTY0LnRzCmZ1bmN0aW9uIGRlY29kZShiNjQpIHsKICBjb25zdCBieXRlcyA9IGVuY29kaW5nLmF0b2IoYjY0KTsKICByZXR1cm4gYnl0ZXM7Cn0KCi8vIGh0dHBzOi8vcmF3LmdpdGh1YnVzZXJjb250ZW50LmNvbS9wdWZmeWNpZC9hcnRlbWlzLWFwaS9tYXN0ZXIvc3JjL3dpbmRvd3MvbnRmcy50cwpmdW5jdGlvbiByZWFkQWRzRGF0YShwYXRoLCBhZHNfbmFtZSkgewogIGNvbnN0IGRhdGEgPSBEZW5vLmNvcmUub3BzLnJlYWRfYWRzX2RhdGEoCiAgICBwYXRoLAogICAgYWRzX25hbWUKICApOwogIHJldHVybiBkYXRhOwp9CgovLyBodHRwczovL3Jhdy5naXRodWJ1c2VyY29udGVudC5jb20vcHVmZnljaWQvYXJ0ZW1pcy1hcGkvbWFzdGVyL3NyYy9maWxlc3lzdGVtL2RpcmVjdG9yeS50cwphc3luYyBmdW5jdGlvbiByZWFkRGlyKHBhdGgpIHsKICBjb25zdCBkYXRhID0gSlNPTi5wYXJzZShhd2FpdCBmcy5yZWFkRGlyKHBhdGgpKTsKICByZXR1cm4gZGF0YTsKfQoKLy8gaHR0cHM6Ly9yYXcuZ2l0aHVidXNlcmNvbnRlbnQuY29tL3B1ZmZ5Y2lkL2FydGVtaXMtYXBpL21hc3Rlci9zcmMvZmlsZXN5c3RlbS9maWxlcy50cwpmdW5jdGlvbiBzdGF0KHBhdGgpIHsKICBjb25zdCBkYXRhID0gZnMuc3RhdChwYXRoKTsKICBjb25zdCB2YWx1ZSA9IEpTT04ucGFyc2UoZGF0YSk7CiAgcmV0dXJuIHZhbHVlOwp9CgovLyBodHRwczovL3Jhdy5naXRodWJ1c2VyY29udGVudC5jb20vcHVmZnljaWQvYXJ0ZW1pcy1hcGkvbWFzdGVyL3NyYy9lbnZpcm9ubWVudC9lbnYudHMKZnVuY3Rpb24gZ2V0RW52VmFsdWUoa2V5KSB7CiAgY29uc3QgZGF0YSA9IGVudi5lbnZpcm9ubWVudFZhbHVlKGtleSk7CiAgcmV0dXJuIGRhdGE7Cn0KCi8vIGh0dHBzOi8vcmF3LmdpdGh1YnVzZXJjb250ZW50LmNvbS9wdWZmeWNpZC9hcnRlbWlzLWFwaS9tYXN0ZXIvc3JjL2VuY29kaW5nL3N0cmluZ3MudHMKZnVuY3Rpb24gZXh0cmFjdFV0ZjhTdHJpbmcoZGF0YSkgewogIGNvbnN0IHJlc3VsdCA9IGVuY29kaW5nLmV4dHJhY3RfdXRmOF9zdHJpbmcoZGF0YSk7CiAgcmV0dXJuIHJlc3VsdDsKfQoKLy8gbWFpbi50cwphc3luYyBmdW5jdGlvbiBtYWluKCkgewogIGNvbnN0IGRyaXZlID0gZ2V0RW52VmFsdWUoIlN5c3RlbURyaXZlIik7CiAgaWYgKGRyaXZlID09PSAiIikgewogICAgcmV0dXJuIFtdOwogIH0KICBjb25zdCB3ZWJfZmlsZXMgPSBbXTsKICBjb25zdCB1c2VycyA9IGAke2RyaXZlfVxcVXNlcnNgOwogIGZvciAoY29uc3QgZW50cnkgb2YgYXdhaXQgcmVhZERpcih1c2VycykpIHsKICAgIHRyeSB7CiAgICAgIGNvbnN0IHBhdGggPSBgJHt1c2Vyc31cXCR7ZW50cnkuZmlsZW5hbWV9XFxEb3dubG9hZHNgOwogICAgICBmb3IgKGNvbnN0IGZpbGVfZW50cnkgb2YgYXdhaXQgcmVhZERpcihwYXRoKSkgewogICAgICAgIHRyeSB7CiAgICAgICAgICBpZiAoIWZpbGVfZW50cnkuaXNfZmlsZSkgewogICAgICAgICAgICBjb250aW51ZTsKICAgICAgICAgIH0KICAgICAgICAgIGNvbnN0IGZ1bGxfcGF0aCA9IGAke3BhdGh9XFwke2ZpbGVfZW50cnkuZmlsZW5hbWV9YDsKICAgICAgICAgIGNvbnN0IGFkcyA9ICJab25lLklkZW50aWZpZXIiOwogICAgICAgICAgY29uc3QgZGF0YSA9IHJlYWRBZHNEYXRhKGZ1bGxfcGF0aCwgYWRzKTsKICAgICAgICAgIGlmIChkYXRhLmxlbmd0aCA9PT0gMCkgewogICAgICAgICAgICBjb250aW51ZTsKICAgICAgICAgIH0KICAgICAgICAgIGNvbnN0IGluZm8gPSBzdGF0KGZ1bGxfcGF0aCk7CiAgICAgICAgICBjb25zdCBtYXJrX2luZm8gPSBleHRyYWN0VXRmOFN0cmluZyhkYXRhKTsKICAgICAgICAgIGNvbnN0IHdlYl9maWxlID0gewogICAgICAgICAgICBtYXJrOiBtYXJrX2luZm8sCiAgICAgICAgICAgIHBhdGg6IGZ1bGxfcGF0aCwKICAgICAgICAgICAgY3JlYXRlZDogaW5mby5jcmVhdGVkLAogICAgICAgICAgICBtb2RpZmllZDogaW5mby5tb2RpZmllZCwKICAgICAgICAgICAgYWNjZXNzZWQ6IGluZm8uYWNjZXNzZWQsCiAgICAgICAgICAgIHNpemU6IGluZm8uc2l6ZQogICAgICAgICAgfTsKICAgICAgICAgIHdlYl9maWxlcy5wdXNoKHdlYl9maWxlKTsKICAgICAgICB9IGNhdGNoIChfZXJyb3IpIHsKICAgICAgICAgIGNvbnRpbnVlOwogICAgICAgIH0KICAgICAgfQogICAgfSBjYXRjaCAoX2Vycm9yKSB7CiAgICAgIGNvbnRpbnVlOwogICAgfQogIH0KICByZXR1cm4gd2ViX2ZpbGVzOwp9Cm1haW4oKTsK";
        let mut output = output_options("runtime_test", "local", "./tmp", false);
        let script = JSScript {
            name: String::from("read_ads_motw"),
            script: test.to_string(),
        };
        execute_script(&mut output, &script).unwrap();
    }

    #[test]
    fn test_read_raw_file_swapfile() {
        let test = "Ly8gaHR0cHM6Ly9yYXcuZ2l0aHVidXNlcmNvbnRlbnQuY29tL3B1ZmZ5Y2lkL2FydGVtaXMtYXBpL21hc3Rlci9zcmMvd2luZG93cy9udGZzLnRzCmZ1bmN0aW9uIHJlYWRSYXdGaWxlKHBhdGgpIHsKICBjb25zdCBkYXRhID0gRGVuby5jb3JlLm9wcy5yZWFkX3Jhd19maWxlKHBhdGgpOwogIHJldHVybiBkYXRhOwp9CgovLyBodHRwczovL3Jhdy5naXRodWJ1c2VyY29udGVudC5jb20vcHVmZnljaWQvYXJ0ZW1pcy1hcGkvbWFzdGVyL3NyYy9lbnZpcm9ubWVudC9lbnYudHMKZnVuY3Rpb24gZ2V0RW52VmFsdWUoa2V5KSB7CiAgY29uc3QgZGF0YSA9IGVudi5lbnZpcm9ubWVudFZhbHVlKGtleSk7CiAgcmV0dXJuIGRhdGE7Cn0KCi8vIGh0dHBzOi8vcmF3LmdpdGh1YnVzZXJjb250ZW50LmNvbS9wdWZmeWNpZC9hcnRlbWlzLWFwaS9tYXN0ZXIvc3JjL2ZpbGVzeXN0ZW0vZmlsZXMudHMKZnVuY3Rpb24gc3RhdChwYXRoKSB7CiAgY29uc3QgZGF0YSA9IGZzLnN0YXQocGF0aCk7CiAgY29uc3QgdmFsdWUgPSBKU09OLnBhcnNlKGRhdGEpOwogIHJldHVybiB2YWx1ZTsKfQoKLy8gbWFpbi50cwpmdW5jdGlvbiBtYWluKCkgewogIGNvbnN0IGRyaXZlID0gZ2V0RW52VmFsdWUoIlN5c3RlbURyaXZlIik7CiAgaWYgKGRyaXZlID09PSAiIikgewogICAgcmV0dXJuIDA7CiAgfQogIHRyeSB7CiAgICBjb25zdCBzd2FwID0gYCR7ZHJpdmV9XFxzd2FwZmlsZS5zeXNgOwogICAgY29uc3QgaW5mbyA9IHN0YXQoc3dhcCk7CiAgICBpZiAoIWluZm8uaXNfZmlsZSkgewogICAgICByZXR1cm4gMDsKICAgIH0KICAgIGNvbnN0IG1heF9zaXplID0gMjE0NzQ4MzY0ODsKICAgIGlmIChpbmZvLnNpemUgPiBtYXhfc2l6ZSkgewogICAgICByZXR1cm4gMDsKICAgIH0KICAgIGNvbnN0IGRhdGEgPSByZWFkUmF3RmlsZShzd2FwKTsKICAgIHJldHVybiBkYXRhLmxlbmd0aDsKICB9IGNhdGNoIChfZXJyb3IpIHsKICAgIHJldHVybiAwOwogIH0KfQptYWluKCk7Cg==";
        let mut output = output_options("runtime_test", "local", "./tmp", false);
        let script = JSScript {
            name: String::from("swapfile"),
            script: test.to_string(),
        };
        execute_script(&mut output, &script).unwrap();
    }
}
