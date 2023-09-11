use crate::artifacts::os::macos::plist::property_list::{parse_plist_data, parse_plist_file};
use deno_core::{error::AnyError, op, JsBuffer};
use log::error;

#[op]
/// Expose parsing plist file  to `Deno`
fn get_plist(path: String) -> Result<String, AnyError> {
    let plist_results = parse_plist_file(&path);
    let plist = match plist_results {
        Ok(results) => results,
        Err(err) => {
            // Parsing plist files could fail for many reasons
            // Instead of cancelling the whole script, return empty result
            error!("[runtime] Failed to parse plist: {err:?}");

            return Ok(String::new());
        }
    };
    let results = serde_json::to_string(&plist)?;
    Ok(results)
}

#[op]
/// Expose parsing plist file  to `Deno`
fn get_plist_data(data: JsBuffer) -> Result<String, AnyError> {
    let plist_results = parse_plist_data(&data);
    let plist = match plist_results {
        Ok(results) => results,
        Err(err) => {
            // Parsing plist files could fail for many reasons
            // Instead of cancelling the whole script, return empty result
            error!("[runtime] Failed to parse plist: {err:?}");

            return Ok(String::new());
        }
    };
    let results = serde_json::to_string(&plist)?;
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
            filter_name: Some(String::new()),
            filter_script: Some(String::new()),
            logging: Some(String::new()),
        }
    }

    #[test]
    fn test_get_plist() {
        // Grabs and attempts to parse all plist files under /User/*, will recurse all directories
        let test = "Ly8gaHR0cHM6Ly9yYXcuZ2l0aHVidXNlcmNvbnRlbnQuY29tL3B1ZmZ5Y2lkL2FydGVtaXMtYXBpL21hc3Rlci9zcmMvbWFjb3MvcGxpc3QudHMKZnVuY3Rpb24gZ2V0UGxpc3QocGF0aCkgewogIGNvbnN0IGRhdGEgPSBEZW5vLmNvcmUub3BzLmdldF9wbGlzdChwYXRoKTsKICBpZiAoZGF0YSA9PT0gIiIpIHsKICAgIHJldHVybiBudWxsOwogIH0KICBjb25zdCBwbGlzdF9kYXRhID0gSlNPTi5wYXJzZShkYXRhKTsKICByZXR1cm4gcGxpc3RfZGF0YTsKfQoKLy8gaHR0cHM6Ly9yYXcuZ2l0aHVidXNlcmNvbnRlbnQuY29tL3B1ZmZ5Y2lkL2FydGVtaXMtYXBpL21hc3Rlci9zcmMvc3lzdGVtL291dHB1dC50cwpmdW5jdGlvbiBvdXRwdXRSZXN1bHRzKGRhdGEsIGRhdGFfbmFtZSwgb3V0cHV0KSB7CiAgY29uc3Qgb3V0cHV0X3N0cmluZyA9IEpTT04uc3RyaW5naWZ5KG91dHB1dCk7CiAgY29uc3Qgc3RhdHVzID0gRGVuby5jb3JlLm9wcy5vdXRwdXRfcmVzdWx0cygKICAgIGRhdGEsCiAgICBkYXRhX25hbWUsCiAgICBvdXRwdXRfc3RyaW5nLAogICk7CiAgcmV0dXJuIHN0YXR1czsKfQoKLy8gaHR0cHM6Ly9yYXcuZ2l0aHVidXNlcmNvbnRlbnQuY29tL3B1ZmZ5Y2lkL2FydGVtaXMtYXBpL21hc3Rlci9zcmMvZmlsZXN5c3RlbS9kaXJlY3RvcnkudHMKYXN5bmMgZnVuY3Rpb24gcmVhZERpcihwYXRoKSB7CiAgY29uc3QgZGF0YSA9IEpTT04ucGFyc2UoYXdhaXQgZnMucmVhZERpcihwYXRoKSk7CiAgcmV0dXJuIGRhdGE7Cn0KCi8vIG1haW4udHMKYXN5bmMgZnVuY3Rpb24gbWFpbigpIHsKICBjb25zdCBzdGFydF9wYXRoID0gIi9Vc2VycyI7CiAgY29uc3QgcGxpc3RfZmlsZXMgPSBbXTsKICBhd2FpdCByZWN1cnNlX2RpcihwbGlzdF9maWxlcywgc3RhcnRfcGF0aCk7CiAgcmV0dXJuIHBsaXN0X2ZpbGVzOwp9CmFzeW5jIGZ1bmN0aW9uIHJlY3Vyc2VfZGlyKHBsaXN0X2ZpbGVzLCBzdGFydF9wYXRoKSB7CiAgaWYgKHBsaXN0X2ZpbGVzLmxlbmd0aCA+IDIwKSB7CiAgICBjb25zdCBvdXQgPSB7CiAgICAgIG5hbWU6ICJhcnRlbWlzX3BsaXN0IiwKICAgICAgZGlyZWN0b3J5OiAiLi90bXAiLAogICAgICBmb3JtYXQ6ICJqc29uIiwgLyogSlNPTiAqLwogICAgICBjb21wcmVzczogZmFsc2UsCiAgICAgIGVuZHBvaW50X2lkOiAiYW55dGhpbmctaS13YW50IiwKICAgICAgY29sbGVjdGlvbl9pZDogMSwKICAgICAgb3V0cHV0OiAibG9jYWwiLCAvKiBMT0NBTCAqLwogICAgfTsKICAgIGNvbnN0IHN0YXR1cyA9IG91dHB1dFJlc3VsdHMoCiAgICAgIEpTT04uc3RyaW5naWZ5KHBsaXN0X2ZpbGVzKSwKICAgICAgImFydGVtaXNfaW5mbyIsCiAgICAgIG91dCwKICAgICk7CiAgICBpZiAoIXN0YXR1cykgewogICAgICBjb25zb2xlLmxvZygiQ291bGQgbm90IG91dHB1dCB0byBsb2NhbCBkaXJlY3RvcnkiKTsKICAgIH0KICAgIHBsaXN0X2ZpbGVzID0gW107CiAgfQogIGZvciAoY29uc3QgZW50cnkgb2YgYXdhaXQgcmVhZERpcihzdGFydF9wYXRoKSkgewogICAgY29uc3QgcGxpc3RfcGF0aCA9IGAke3N0YXJ0X3BhdGh9LyR7ZW50cnkuZmlsZW5hbWV9YDsKICAgIGlmIChlbnRyeS5pc19maWxlICYmIGVudHJ5LmZpbGVuYW1lLmVuZHNXaXRoKCJwbGlzdCIpKSB7CiAgICAgIGNvbnN0IGRhdGEgPSBnZXRQbGlzdChwbGlzdF9wYXRoKTsKICAgICAgaWYgKGRhdGEgPT09IG51bGwpIHsKICAgICAgICBjb250aW51ZTsKICAgICAgfQogICAgICBjb25zdCBwbGlzdF9pbmZvID0gewogICAgICAgIHBsaXN0X2NvbnRlbnQ6IGRhdGEsCiAgICAgICAgZmlsZTogcGxpc3RfcGF0aCwKICAgICAgfTsKICAgICAgcGxpc3RfZmlsZXMucHVzaChwbGlzdF9pbmZvKTsKICAgICAgY29udGludWU7CiAgICB9CiAgICBpZiAoZW50cnkuaXNfZGlyZWN0b3J5KSB7CiAgICAgIGF3YWl0IHJlY3Vyc2VfZGlyKHBsaXN0X2ZpbGVzLCBwbGlzdF9wYXRoKTsKICAgIH0KICB9Cn0KbWFpbigpOwo=";
        let mut output = output_options("runtime_test", "local", "./tmp", false);

        let script = JSScript {
            name: String::from("plist_files"),
            script: test.to_string(),
        };
        execute_script(&mut output, &script).unwrap();
    }

    #[test]
    fn test_get_plist_data() {
        let test = "Ly8gaHR0cHM6Ly9yYXcuZ2l0aHVidXNlcmNvbnRlbnQuY29tL3B1ZmZ5Y2lkL2FydGVtaXMtYXBpL21hc3Rlci9zcmMvbWFjb3MvcGxpc3QudHMKZnVuY3Rpb24gZ2V0UGxpc3QocGF0aCkgewogIGlmIChwYXRoIGluc3RhbmNlb2YgVWludDhBcnJheSkgewogICAgY29uc3QgZGF0YTIgPSBEZW5vLmNvcmUub3BzLmdldF9wbGlzdF9kYXRhKHBhdGgpOwogICAgaWYgKGRhdGEyIGluc3RhbmNlb2YgRXJyb3IpIHsKICAgICAgcmV0dXJuIGRhdGEyOwogICAgfQogICAgY29uc3QgcGxpc3RfZGF0YTIgPSBKU09OLnBhcnNlKGRhdGEyKTsKICAgIHJldHVybiBwbGlzdF9kYXRhMjsKICB9CiAgY29uc3QgZGF0YSA9IERlbm8uY29yZS5vcHMuZ2V0X3BsaXN0KHBhdGgpOwogIGlmIChkYXRhIGluc3RhbmNlb2YgRXJyb3IpIHsKICAgIHJldHVybiBkYXRhOwogIH0KICBjb25zdCBwbGlzdF9kYXRhID0gSlNPTi5wYXJzZShkYXRhKTsKICByZXR1cm4gcGxpc3RfZGF0YTsKfQoKLy8gaHR0cHM6Ly9yYXcuZ2l0aHVidXNlcmNvbnRlbnQuY29tL3B1ZmZ5Y2lkL2FydGVtaXMtYXBpL21hc3Rlci9zcmMvZW5jb2RpbmcvYmFzZTY0LnRzCmZ1bmN0aW9uIGRlY29kZShiNjQpIHsKICBjb25zdCBieXRlcyA9IGVuY29kaW5nLmF0b2IoYjY0KTsKICByZXR1cm4gYnl0ZXM7Cn0KCi8vIG1haW4udHMKZnVuY3Rpb24gbWFpbigpIHsKICBjb25zdCBkYXRhID0gIkFBQUFBQUZ1QUFJQUFBeE5ZV05wYm5SdmMyZ2dTRVFBQUFBQUFBQUFBQUFBQUFBQUFBQUFBQUFBUWtRQUFmLy8vLzhLYlhWc2RHbHdZWE56WkFBQUFBQUFBQUFBQUFBQUFBQUFBQUFBQUFBQUFBQUFBQUFBQUFBQUFBQUFBQUFBQUFBQUFBQUFBQUFBQUFBQUFBQUFBQUFBLy8vLy93QUFBQUFBQUFBQUFBQUFBUC8vLy84QUFBb2dZM1VBQUFBQUFBQUFBQUFBQUFBQUEySnBiZ0FBQWdCRUx6cE1hV0p5WVhKNU9rRndjR3hwWTJGMGFXOXVJRk4xY0hCdmNuUTZZMjl0TG1OaGJtOXVhV05oYkM1dGRXeDBhWEJoYzNNNlltbHVPbTExYkhScGNHRnpjMlFBRGdBV0FBb0FiUUIxQUd3QWRBQnBBSEFBWVFCekFITUFaQUFQQUJvQURBQk5BR0VBWXdCcEFHNEFkQUJ2QUhNQWFBQWdBRWdBUkFBU0FFSk1hV0p5WVhKNUwwRndjR3hwWTJGMGFXOXVJRk4xY0hCdmNuUXZZMjl0TG1OaGJtOXVhV05oYkM1dGRXeDBhWEJoYzNNdlltbHVMMjExYkhScGNHRnpjMlFBRXdBQkx3RC8vd0FBIjsKICBjb25zdCByYXdfcGxpc3QgPSBkZWNvZGUoZGF0YSk7CiAgY29uc3QgX3Jlc3VsdHMgPSBnZXRQbGlzdChyYXdfcGxpc3QpOwp9Cm1haW4oKTsK";
        let mut output = output_options("runtime_test", "local", "./tmp", false);

        let script = JSScript {
            name: String::from("plist_raw"),
            script: test.to_string(),
        };
        execute_script(&mut output, &script).unwrap();
    }
}
