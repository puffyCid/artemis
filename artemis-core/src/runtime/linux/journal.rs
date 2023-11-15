use crate::artifacts::os::linux::journals::parser::grab_journal_file;
use deno_core::{error::AnyError, op2};

#[op2]
#[string]
/// Expose parsing journal file  to `Deno`
pub(crate) fn get_journal(#[string] path: String) -> Result<String, AnyError> {
    let journal_data = grab_journal_file(&path)?;
    let results = serde_json::to_string(&journal_data)?;
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
    fn test_get_journal() {
        let test = "Ly8gaHR0cHM6Ly9yYXcuZ2l0aHVidXNlcmNvbnRlbnQuY29tL3B1ZmZ5Y2lkL2FydGVtaXMtYXBpL21hc3Rlci9zcmMvbGludXgvam91cm5hbC50cwpmdW5jdGlvbiBnZXRKb3VybmFsKHBhdGgpIHsKICBjb25zdCBkYXRhID0gRGVuby5jb3JlLm9wcy5nZXRfam91cm5hbChwYXRoKTsKICBpZiAoZGF0YSA9PT0gIiIpIHsKICAgIHJldHVybiBudWxsOwogIH0KICBjb25zdCBqb3VybmFsID0gSlNPTi5wYXJzZShkYXRhKTsKICByZXR1cm4gam91cm5hbDsKfQoKLy8gaHR0cHM6Ly9yYXcuZ2l0aHVidXNlcmNvbnRlbnQuY29tL3B1ZmZ5Y2lkL2FydGVtaXMtYXBpL21hc3Rlci9zcmMvc3lzdGVtL291dHB1dC50cwpmdW5jdGlvbiBvdXRwdXRSZXN1bHRzKGRhdGEsIGRhdGFfbmFtZSwgb3V0cHV0KSB7CiAgY29uc3Qgb3V0cHV0X3N0cmluZyA9IEpTT04uc3RyaW5naWZ5KG91dHB1dCk7CiAgY29uc3Qgc3RhdHVzID0gRGVuby5jb3JlLm9wcy5vdXRwdXRfcmVzdWx0cygKICAgIGRhdGEsCiAgICBkYXRhX25hbWUsCiAgICBvdXRwdXRfc3RyaW5nCiAgKTsKICByZXR1cm4gc3RhdHVzOwp9CgovLyBodHRwczovL3Jhdy5naXRodWJ1c2VyY29udGVudC5jb20vcHVmZnljaWQvYXJ0ZW1pcy1hcGkvbWFzdGVyL3NyYy9maWxlc3lzdGVtL2RpcmVjdG9yeS50cwphc3luYyBmdW5jdGlvbiByZWFkRGlyKHBhdGgpIHsKICBjb25zdCBkYXRhID0gSlNPTi5wYXJzZShhd2FpdCBmcy5yZWFkRGlyKHBhdGgpKTsKICByZXR1cm4gZGF0YTsKfQoKLy8gbWFpbi50cwphc3luYyBmdW5jdGlvbiBtYWluKCkgewogIGNvbnN0IGpvdXJuYWxzID0gIi92YXIvbG9nL2pvdXJuYWwiOwogIGNvbnN0IG91dCA9IHsKICAgIG5hbWU6ICJkZW5vX2pvdXJuYWxzIiwKICAgIGRpcmVjdG9yeTogIi4vdG1wIiwKICAgIGZvcm1hdDogImpzb24iIC8qIEpTT04gKi8sCiAgICBjb21wcmVzczogZmFsc2UsCiAgICBlbmRwb2ludF9pZDogImFueXRoaW5nLWktd2FudCIsCiAgICBjb2xsZWN0aW9uX2lkOiAxLAogICAgb3V0cHV0OiAibG9jYWwiIC8qIExPQ0FMICovCiAgfTsKICBmb3IgKGNvbnN0IGVudHJ5IG9mIGF3YWl0IHJlYWREaXIoam91cm5hbHMpKSB7CiAgICBpZiAoIWVudHJ5LmlzX2RpcmVjdG9yeSkgewogICAgICBjb250aW51ZTsKICAgIH0KICAgIGNvbnN0IGZ1bGxfcGF0aCA9IGAke2pvdXJuYWxzfS8ke2VudHJ5LmZpbGVuYW1lfWA7CiAgICBmb3IgKGNvbnN0IGZpbGVzIG9mIGF3YWl0IHJlYWREaXIoZnVsbF9wYXRoKSkgewogICAgICBpZiAoIWZpbGVzLmZpbGVuYW1lLmVuZHNXaXRoKCJqb3VybmFsIikpIHsKICAgICAgICBjb250aW51ZTsKICAgICAgfQogICAgICBjb25zdCBqb3VybmFsX2ZpbGUgPSBgJHtmdWxsX3BhdGh9LyR7ZmlsZXMuZmlsZW5hbWV9YDsKICAgICAgY29uc3QgZGF0YSA9IGdldEpvdXJuYWwoam91cm5hbF9maWxlKTsKICAgICAgY29uc3Qgc3RhdHVzID0gb3V0cHV0UmVzdWx0cyhKU09OLnN0cmluZ2lmeShkYXRhKSwgImpvdXJuYWwiLCBvdXQpOwogICAgICBpZiAoIXN0YXR1cykgewogICAgICAgIGNvbnNvbGUubG9nKCJDb3VsZCBub3Qgb3V0cHV0IHRvIGxvY2FsIGRpcmVjdG9yeSIpOwogICAgICB9CiAgICB9CiAgfQp9Cm1haW4oKTsK";
        let mut output = output_options("runtime_test", "local", "./tmp", false);

        let script = JSScript {
            name: String::from("journal"),
            script: test.to_string(),
        };
        execute_script(&mut output, &script).unwrap();
    }
}
