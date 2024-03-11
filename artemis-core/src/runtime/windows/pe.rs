use crate::artifacts::os::windows::pe::parser::parse_pe_file;
use deno_core::{error::AnyError, op2};

#[op2]
#[string]
/// Expose parsing pe file  to `Deno`
pub(crate) fn get_pe(#[string] path: String) -> Result<String, AnyError> {
    let pe = parse_pe_file(&path)?;
    let results = serde_json::to_string(&pe)?;
    Ok(results)
}

#[cfg(test)]
#[cfg(target_os = "windows")]
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
    fn test_get_pe() {
        let test = "Ly8gaHR0cHM6Ly9yYXcuZ2l0aHVidXNlcmNvbnRlbnQuY29tL3B1ZmZ5Y2lkL2FydGVtaXMtYXBpL21hc3Rlci9zcmMvd2luZG93cy9wZS50cwpmdW5jdGlvbiBnZXRQZShwYXRoKSB7CiAgY29uc3QgZGF0YSA9IERlbm8uY29yZS5vcHMuZ2V0X3BlKHBhdGgpOwogIGlmIChkYXRhID09PSAiIikgewogICAgcmV0dXJuIG51bGw7CiAgfQogIGNvbnN0IHJlc3VsdCA9IEpTT04ucGFyc2UoZGF0YSk7CiAgcmV0dXJuIHJlc3VsdDsKfQoKLy8gaHR0cHM6Ly9yYXcuZ2l0aHVidXNlcmNvbnRlbnQuY29tL3B1ZmZ5Y2lkL2FydGVtaXMtYXBpL21hc3Rlci9zcmMvZW52aXJvbm1lbnQvZW52LnRzCmZ1bmN0aW9uIGdldEVudlZhbHVlKGtleSkgewogIGNvbnN0IGRhdGEgPSBlbnYuZW52aXJvbm1lbnRWYWx1ZShrZXkpOwogIHJldHVybiBkYXRhOwp9CgovLyBodHRwczovL3Jhdy5naXRodWJ1c2VyY29udGVudC5jb20vcHVmZnljaWQvYXJ0ZW1pcy1hcGkvbWFzdGVyL3NyYy9maWxlc3lzdGVtL2RpcmVjdG9yeS50cwphc3luYyBmdW5jdGlvbiByZWFkRGlyKHBhdGgpIHsKICBjb25zdCBkYXRhID0gSlNPTi5wYXJzZShhd2FpdCBmcy5yZWFkRGlyKHBhdGgpKTsKICByZXR1cm4gZGF0YTsKfQoKLy8gbWFpbi50cwphc3luYyBmdW5jdGlvbiBtYWluKCkgewogIGNvbnN0IGRyaXZlID0gZ2V0RW52VmFsdWUoIlN5c3RlbURyaXZlIik7CiAgaWYgKGRyaXZlID09PSAiIikgewogICAgcmV0dXJuIFtdOwogIH0KICBjb25zdCBwYXRoID0gYCR7ZHJpdmV9XFxXaW5kb3dzXFxTeXN0ZW0zMmA7CiAgY29uc3QgcGVzID0gW107CiAgZm9yIChjb25zdCBlbnRyeSBvZiBhd2FpdCByZWFkRGlyKHBhdGgpKSB7CiAgICBpZiAoIWVudHJ5LmlzX2ZpbGUpIHsKICAgICAgY29udGludWU7CiAgICB9CiAgICBjb25zdCBwZV9wYXRoID0gYCR7cGF0aH1cXCR7ZW50cnkuZmlsZW5hbWV9YDsKICAgIGNvbnN0IGluZm8gPSBnZXRQZShwZV9wYXRoKTsKICAgIGlmIChpbmZvID09PSBudWxsKSB7CiAgICAgIGNvbnRpbnVlOwogICAgfQogICAgY29uc3QgbWV0YSA9IHsKICAgICAgcGF0aDogcGVfcGF0aCwKICAgICAgcGU6IGluZm8KICAgIH07CiAgICBwZXMucHVzaChtZXRhKTsKICB9CiAgcmV0dXJuIHBlczsKfQptYWluKCk7Cg==";
        let mut output = output_options("runtime_test", "local", "./tmp", false);

        let script = JSScript {
            name: String::from("system32_pe"),
            script: test.to_string(),
        };
        execute_script(&mut output, &script).unwrap();
    }
}
