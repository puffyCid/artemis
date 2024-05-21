use crate::{
    artifacts::os::windows::search::parser::grab_search_path, runtime::error::RuntimeError,
};
use deno_core::{error::AnyError, op2};
use log::error;

#[op2]
#[string]
/// Expose parsing Windows Search to `Deno`
pub(crate) fn get_search(#[string] path: String, page_limit: u32) -> Result<String, AnyError> {
    if path.is_empty() {
        error!("[runtime] Empty path to Search file");
        return Err(RuntimeError::ExecuteScript.into());
    }
    let search = grab_search_path(&path, &page_limit)?;

    let results = serde_json::to_string(&search)?;
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
            format: String::from("jsonl"),
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
    fn test_get_search() {
        let test = "Ly8gaHR0cHM6Ly9yYXcuZ2l0aHVidXNlcmNvbnRlbnQuY29tL3B1ZmZ5Y2lkL2FydGVtaXMtYXBpL21hc3Rlci9zcmMvd2luZG93cy9zZWFyY2gudHMKZnVuY3Rpb24gZ2V0U2VhcmNoKHBhdGgpIHsKICBjb25zdCBkYXRhID0gRGVuby5jb3JlLm9wcy5nZXRfc2VhcmNoKHBhdGgsIDUwKTsKICBjb25zdCByZXN1bHQgPSBKU09OLnBhcnNlKGRhdGEpOwogIHJldHVybiByZXN1bHQ7Cn0KCi8vIGh0dHBzOi8vcmF3LmdpdGh1YnVzZXJjb250ZW50LmNvbS9wdWZmeWNpZC9hcnRlbWlzLWFwaS9tYXN0ZXIvc3JjL2Vudmlyb25tZW50L2Vudi50cwpmdW5jdGlvbiBnZXRFbnZWYWx1ZShrZXkpIHsKICBjb25zdCBkYXRhID0gZW52LmVudmlyb25tZW50VmFsdWUoa2V5KTsKICByZXR1cm4gZGF0YTsKfQoKLy8gaHR0cHM6Ly9yYXcuZ2l0aHVidXNlcmNvbnRlbnQuY29tL3B1ZmZ5Y2lkL2FydGVtaXMtYXBpL21hc3Rlci9zcmMvZmlsZXN5c3RlbS9maWxlcy50cwpmdW5jdGlvbiBzdGF0KHBhdGgpIHsKICBjb25zdCBkYXRhID0gZnMuc3RhdChwYXRoKTsKICBjb25zdCB2YWx1ZSA9IEpTT04ucGFyc2UoZGF0YSk7CiAgcmV0dXJuIHZhbHVlOwp9CgovLyBtYWluLnRzCmZ1bmN0aW9uIG1haW4oKSB7CiAgY29uc3QgZHJpdmUgPSBnZXRFbnZWYWx1ZSgiU3lzdGVtRHJpdmUiKTsKICBpZiAoZHJpdmUgPT09ICIiKSB7CiAgICByZXR1cm4gW107CiAgfQogIGNvbnN0IHBhdGggPSBgJHtkcml2ZX1cXFByb2dyYW1EYXRhXFxNaWNyb3NvZnRcXFNlYXJjaFxcRGF0YVxcQXBwbGljYXRpb25zXFxXaW5kb3dzYDsKICB0cnkgewogICAgY29uc3Qgc2VhcmNoX3BhdGggPSBgJHtwYXRofVxcV2luZG93cy5lZGJgOwogICAgY29uc3Qgc3RhdHVzID0gc3RhdChzZWFyY2hfcGF0aCk7CiAgICBpZiAoIXN0YXR1cy5pc19maWxlKSB7CiAgICAgIHJldHVybiBbXTsKICAgIH0KICAgIGNvbnN0IHJlc3VsdHMgPSBnZXRTZWFyY2goc2VhcmNoX3BhdGgpOwogICAgcmV0dXJuIHJlc3VsdHM7CiAgfSBjYXRjaCAoX2UpIHsKICAgIGNvbnN0IHNlYXJjaF9wYXRoID0gYCR7cGF0aH1cXFdpbmRvd3MuZGJgOwogICAgY29uc3Qgc3RhdHVzID0gc3RhdChzZWFyY2hfcGF0aCk7CiAgICBpZiAoIXN0YXR1cy5pc19maWxlKSB7CiAgICAgIHJldHVybiBbXTsKICAgIH0KICAgIGNvbnN0IHJlc3VsdHMgPSBnZXRTZWFyY2goc2VhcmNoX3BhdGgpOwogICAgcmV0dXJuIHJlc3VsdHM7CiAgfQp9Cm1haW4oKTsK";
        let mut output = output_options("runtime_test", "local", "./tmp", false);
        let script = JSScript {
            name: String::from("search"),
            script: test.to_string(),
        };
        execute_script(&mut output, &script).unwrap();
    }
}
