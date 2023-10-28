use crate::artifacts::os::macos::plist::property_list::{parse_plist_data, parse_plist_file};
use deno_core::{error::AnyError, op2, JsBuffer};
use log::error;

#[op2]
#[string]
/// Expose parsing plist file  to `Deno`
pub(crate) fn get_plist(#[string] path: String) -> Result<String, AnyError> {
    let plist_results = parse_plist_file(&path);
    let plist = match plist_results {
        Ok(results) => results,
        Err(err) => {
            // Parsing plist files could fail for many reasons
            error!("[runtime] Failed to parse plist: {err:?}");
            return Err(err.into());
        }
    };
    let results = serde_json::to_string(&plist)?;
    Ok(results)
}

#[op2]
#[string]
/// Expose parsing plist file  to `Deno`
pub(crate) fn get_plist_data(#[buffer] data: JsBuffer) -> Result<String, AnyError> {
    let plist_results = parse_plist_data(&data);
    let plist = match plist_results {
        Ok(results) => results,
        Err(err) => {
            // Parsing plist files could fail for many reasons
            error!("[runtime] Failed to parse plist: {err:?}");
            return Err(err.into());
        }
    };
    let results = serde_json::to_string(&plist)?;
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
            filter_name: Some(String::new()),
            filter_script: Some(String::new()),
            logging: Some(String::new()),
        }
    }

    #[test]
    fn test_get_plist() {
        // Grabs and attempts to parse all plist files under /User/*, will recurse all directories
        let test = "Ly8gaHR0cHM6Ly9yYXcuZ2l0aHVidXNlcmNvbnRlbnQuY29tL3B1ZmZ5Y2lkL2FydGVtaXMtYXBpL21hc3Rlci9zcmMvbWFjb3MvcGxpc3QudHMKZnVuY3Rpb24gZ2V0UGxpc3QocGF0aCkgewogIGlmIChwYXRoIGluc3RhbmNlb2YgVWludDhBcnJheSkgewogICAgY29uc3QgZGF0YTIgPSBEZW5vLmNvcmUub3BzLmdldF9wbGlzdF9kYXRhKHBhdGgpOwogICAgaWYgKGRhdGEyIGluc3RhbmNlb2YgRXJyb3IpIHsKICAgICAgcmV0dXJuIGRhdGEyOwogICAgfQogICAgY29uc3QgcGxpc3RfZGF0YTIgPSBKU09OLnBhcnNlKGRhdGEyKTsKICAgIHJldHVybiBwbGlzdF9kYXRhMjsKICB9CiAgY29uc3QgZGF0YSA9IERlbm8uY29yZS5vcHMuZ2V0X3BsaXN0KHBhdGgpOwogIGlmIChkYXRhIGluc3RhbmNlb2YgRXJyb3IpIHsKICAgIHJldHVybiBkYXRhOwogIH0KICBjb25zdCBwbGlzdF9kYXRhID0gSlNPTi5wYXJzZShkYXRhKTsKICByZXR1cm4gcGxpc3RfZGF0YTsKfQoKLy8gaHR0cHM6Ly9yYXcuZ2l0aHVidXNlcmNvbnRlbnQuY29tL3B1ZmZ5Y2lkL2FydGVtaXMtYXBpL21hc3Rlci9zcmMvc3lzdGVtL291dHB1dC50cwpmdW5jdGlvbiBvdXRwdXRSZXN1bHRzKGRhdGEsIGRhdGFfbmFtZSwgb3V0cHV0KSB7CiAgY29uc3Qgb3V0cHV0X3N0cmluZyA9IEpTT04uc3RyaW5naWZ5KG91dHB1dCk7CiAgY29uc3Qgc3RhdHVzID0gRGVuby5jb3JlLm9wcy5vdXRwdXRfcmVzdWx0cygKICAgIGRhdGEsCiAgICBkYXRhX25hbWUsCiAgICBvdXRwdXRfc3RyaW5nCiAgKTsKICByZXR1cm4gc3RhdHVzOwp9CgovLyBodHRwczovL3Jhdy5naXRodWJ1c2VyY29udGVudC5jb20vcHVmZnljaWQvYXJ0ZW1pcy1hcGkvbWFzdGVyL3NyYy9maWxlc3lzdGVtL2RpcmVjdG9yeS50cwphc3luYyBmdW5jdGlvbiByZWFkRGlyKHBhdGgpIHsKICBjb25zdCByZXN1bHQgPSBhd2FpdCBmcy5yZWFkRGlyKHBhdGgpOwogIGlmIChyZXN1bHQgaW5zdGFuY2VvZiBFcnJvcikgewogICAgcmV0dXJuIHJlc3VsdDsKICB9CiAgY29uc3QgZGF0YSA9IEpTT04ucGFyc2UocmVzdWx0KTsKICByZXR1cm4gZGF0YTsKfQoKLy8gbWFpbi50cwphc3luYyBmdW5jdGlvbiBtYWluKCkgewogIGNvbnN0IHN0YXJ0X3BhdGggPSAiL1VzZXJzIjsKICBjb25zdCBwbGlzdF9maWxlcyA9IFtdOwogIGF3YWl0IHJlY3Vyc2VfZGlyKHBsaXN0X2ZpbGVzLCBzdGFydF9wYXRoKTsKICByZXR1cm4gcGxpc3RfZmlsZXM7Cn0KYXN5bmMgZnVuY3Rpb24gcmVjdXJzZV9kaXIocGxpc3RfZmlsZXMsIHN0YXJ0X3BhdGgpIHsKICBpZiAocGxpc3RfZmlsZXMubGVuZ3RoID4gMjApIHsKICAgIGNvbnN0IG91dCA9IHsKICAgICAgbmFtZTogImFydGVtaXNfcGxpc3QiLAogICAgICBkaXJlY3Rvcnk6ICIuL3RtcCIsCiAgICAgIGZvcm1hdDogImpzb24iIC8qIEpTT04gKi8sCiAgICAgIGNvbXByZXNzOiBmYWxzZSwKICAgICAgZW5kcG9pbnRfaWQ6ICJhbnl0aGluZy1pLXdhbnQiLAogICAgICBjb2xsZWN0aW9uX2lkOiAxLAogICAgICBvdXRwdXQ6ICJsb2NhbCIgLyogTE9DQUwgKi8KICAgIH07CiAgICBjb25zdCBzdGF0dXMgPSBvdXRwdXRSZXN1bHRzKAogICAgICBKU09OLnN0cmluZ2lmeShwbGlzdF9maWxlcyksCiAgICAgICJhcnRlbWlzX2luZm8iLAogICAgICBvdXQKICAgICk7CiAgICBpZiAoIXN0YXR1cykgewogICAgICBjb25zb2xlLmxvZygiQ291bGQgbm90IG91dHB1dCB0byBsb2NhbCBkaXJlY3RvcnkiKTsKICAgIH0KICAgIHBsaXN0X2ZpbGVzID0gW107CiAgfQogIGNvbnN0IHJlc3VsdCA9IGF3YWl0IHJlYWREaXIoc3RhcnRfcGF0aCk7CiAgaWYgKHJlc3VsdCBpbnN0YW5jZW9mIEVycm9yKSB7CiAgICByZXR1cm47CiAgfQogIGZvciAoY29uc3QgZW50cnkgb2YgcmVzdWx0KSB7CiAgICBjb25zdCBwbGlzdF9wYXRoID0gYCR7c3RhcnRfcGF0aH0vJHtlbnRyeS5maWxlbmFtZX1gOwogICAgaWYgKGVudHJ5LmlzX2ZpbGUgJiYgZW50cnkuZmlsZW5hbWUuZW5kc1dpdGgoInBsaXN0IikpIHsKICAgICAgdHJ5IHsKICAgICAgICBjb25zdCBkYXRhID0gZ2V0UGxpc3QocGxpc3RfcGF0aCk7CiAgICAgICAgaWYgKGRhdGEgaW5zdGFuY2VvZiBFcnJvcikgewogICAgICAgICAgY29udGludWU7CiAgICAgICAgfQogICAgICAgIGNvbnN0IHBsaXN0X2luZm8gPSB7CiAgICAgICAgICBwbGlzdF9jb250ZW50OiBkYXRhLAogICAgICAgICAgZmlsZTogcGxpc3RfcGF0aAogICAgICAgIH07CiAgICAgICAgcGxpc3RfZmlsZXMucHVzaChwbGlzdF9pbmZvKTsKICAgICAgfSBjYXRjaCAoX2VycikgewogICAgICAgIGNvbnRpbnVlOwogICAgICB9CiAgICAgIGNvbnRpbnVlOwogICAgfQogICAgaWYgKGVudHJ5LmlzX2RpcmVjdG9yeSkgewogICAgICBhd2FpdCByZWN1cnNlX2RpcihwbGlzdF9maWxlcywgcGxpc3RfcGF0aCk7CiAgICB9CiAgfQp9Cm1haW4oKTsK";
        let mut output = output_options("runtime_test", "local", "./tmp", false);

        let script = JSScript {
            name: String::from("plist_files"),
            script: test.to_string(),
        };
        execute_script(&mut output, &script).unwrap();
    }

    #[test]
    fn test_get_plist_data() {
        let test = "Ly8gaHR0cHM6Ly9yYXcuZ2l0aHVidXNlcmNvbnRlbnQuY29tL3B1ZmZ5Y2lkL2FydGVtaXMtYXBpL21hc3Rlci9zcmMvbWFjb3MvcGxpc3QudHMKZnVuY3Rpb24gZ2V0UGxpc3QocGF0aCkgewogIGlmIChwYXRoIGluc3RhbmNlb2YgVWludDhBcnJheSkgewogICAgY29uc3QgZGF0YTIgPSBEZW5vLmNvcmUub3BzLmdldF9wbGlzdF9kYXRhKHBhdGgpOwogICAgaWYgKGRhdGEyIGluc3RhbmNlb2YgRXJyb3IpIHsKICAgICAgcmV0dXJuIGRhdGEyOwogICAgfQogICAgY29uc3QgcGxpc3RfZGF0YTIgPSBKU09OLnBhcnNlKGRhdGEyKTsKICAgIHJldHVybiBwbGlzdF9kYXRhMjsKICB9CiAgY29uc3QgZGF0YSA9IERlbm8uY29yZS5vcHMuZ2V0X3BsaXN0KHBhdGgpOwogIGlmIChkYXRhIGluc3RhbmNlb2YgRXJyb3IpIHsKICAgIHJldHVybiBkYXRhOwogIH0KICBjb25zdCBwbGlzdF9kYXRhID0gSlNPTi5wYXJzZShkYXRhKTsKICByZXR1cm4gcGxpc3RfZGF0YTsKfQoKLy8gaHR0cHM6Ly9yYXcuZ2l0aHVidXNlcmNvbnRlbnQuY29tL3B1ZmZ5Y2lkL2FydGVtaXMtYXBpL21hc3Rlci9zcmMvZW5jb2RpbmcvYmFzZTY0LnRzCmZ1bmN0aW9uIGRlY29kZShiNjQpIHsKICBjb25zdCBieXRlcyA9IGVuY29kaW5nLmF0b2IoYjY0KTsKICByZXR1cm4gYnl0ZXM7Cn0KCi8vIG1haW4udHMKZnVuY3Rpb24gbWFpbigpIHsKICBjb25zdCBkYXRhID0gIkFBQUFBQUZ1QUFJQUFBeE5ZV05wYm5SdmMyZ2dTRVFBQUFBQUFBQUFBQUFBQUFBQUFBQUFBQUFBUWtRQUFmLy8vLzhLYlhWc2RHbHdZWE56WkFBQUFBQUFBQUFBQUFBQUFBQUFBQUFBQUFBQUFBQUFBQUFBQUFBQUFBQUFBQUFBQUFBQUFBQUFBQUFBQUFBQUFBQUFBQUFBLy8vLy93QUFBQUFBQUFBQUFBQUFBUC8vLy84QUFBb2dZM1VBQUFBQUFBQUFBQUFBQUFBQUEySnBiZ0FBQWdCRUx6cE1hV0p5WVhKNU9rRndjR3hwWTJGMGFXOXVJRk4xY0hCdmNuUTZZMjl0TG1OaGJtOXVhV05oYkM1dGRXeDBhWEJoYzNNNlltbHVPbTExYkhScGNHRnpjMlFBRGdBV0FBb0FiUUIxQUd3QWRBQnBBSEFBWVFCekFITUFaQUFQQUJvQURBQk5BR0VBWXdCcEFHNEFkQUJ2QUhNQWFBQWdBRWdBUkFBU0FFSk1hV0p5WVhKNUwwRndjR3hwWTJGMGFXOXVJRk4xY0hCdmNuUXZZMjl0TG1OaGJtOXVhV05oYkM1dGRXeDBhWEJoYzNNdlltbHVMMjExYkhScGNHRnpjMlFBRXdBQkx3RC8vd0FBIjsKICBjb25zdCByYXdfcGxpc3QgPSBkZWNvZGUoZGF0YSk7CiAgdHJ5IHsKICAgY29uc3QgX3Jlc3VsdHMgPSBnZXRQbGlzdChyYXdfcGxpc3QpOwogIH0gY2F0Y2goX2Vycikge30KICAKfQptYWluKCk7Cg==";
        let mut output = output_options("runtime_test", "local", "./tmp", false);

        let script = JSScript {
            name: String::from("plist_raw"),
            script: test.to_string(),
        };
        execute_script(&mut output, &script).unwrap();
    }
}
