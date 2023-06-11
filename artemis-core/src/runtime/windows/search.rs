use crate::{
    artifacts::os::windows::search::parser::grab_search_path, runtime::error::RuntimeError,
};
use deno_core::{error::AnyError, op};
use log::error;

#[op]
/// Expose parsing Windows Search to `Deno`
fn get_search(path: String) -> Result<String, AnyError> {
    if path.is_empty() {
        error!("[runtime] Empty path to Search file");
        return Err(RuntimeError::ExecuteScript.into());
    }
    let search_results = grab_search_path(&path);
    let search = match search_results {
        Ok(results) => results,
        Err(err) => {
            error!("[runtime] Failed to parse Search: {err:?}");
            return Err(RuntimeError::ExecuteScript.into());
        }
    };

    let results = serde_json::to_string_pretty(&search)?;
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
            format: String::from("jsonl"),
            compress,
            url: Some(String::new()),
            port: Some(0),
            api_key: Some(String::new()),
            username: Some(String::new()),
            password: Some(String::new()),
            generic_keys: Some(Vec::new()),
            endpoint_id: String::from("abcd"),
            collection_id: 0,
            output: output.to_string(),
            filter_name: None,
            filter_script: None,
        }
    }

    #[test]
    #[ignore = "Can take a long time"]
    fn test_get_search() {
        let test = "Ly8gLi4vLi4vYXJ0ZW1pcy1hcGkvc3JjL3dpbmRvd3Mvc2VhcmNoLnRzCmZ1bmN0aW9uIGdldF9zZWFyY2gocGF0aCkgewogIGNvbnN0IGRhdGEgPSBEZW5vW0Rlbm8uaW50ZXJuYWxdLmNvcmUub3BzLmdldF9zZWFyY2gocGF0aCk7CiAgY29uc3Qgc3J1bSA9IEpTT04ucGFyc2UoZGF0YSk7CiAgcmV0dXJuIHNydW07Cn0KCi8vIC4uLy4uL2FydGVtaXMtYXBpL21vZC50cwpmdW5jdGlvbiBnZXRTZWFyY2gocGF0aCkgewogIHJldHVybiBnZXRfc2VhcmNoKHBhdGgpOwp9CgovLyBtYWluLnRzCmZ1bmN0aW9uIG1haW4oKSB7CiAgY29uc3QgZHJpdmUgPSBEZW5vLmVudi5nZXQoIlN5c3RlbURyaXZlIik7CiAgaWYgKGRyaXZlID09PSB2b2lkIDApIHsKICAgIHJldHVybiBbXTsKICB9CiAgY29uc3QgcGF0aCA9CiAgICBgJHtkcml2ZX1cXFByb2dyYW1EYXRhXFxNaWNyb3NvZnRcXFNlYXJjaFxcRGF0YVxcQXBwbGljYXRpb25zXFxXaW5kb3dzYDsKICB0cnkgewogICAgY29uc3Qgc2VhcmNoX3BhdGggPSBgJHtwYXRofVxcV2luZG93cy5lZGJgOwogICAgY29uc3Qgc3RhdHVzID0gRGVuby5sc3RhdFN5bmMoc2VhcmNoX3BhdGgpOwogICAgaWYgKCFzdGF0dXMuaXNGaWxlKSB7CiAgICAgIHJldHVybiBbXTsKICAgIH0KICAgIGNvbnN0IHJlc3VsdHMgPSBnZXRTZWFyY2goc2VhcmNoX3BhdGgpOwogICAgcmV0dXJuIHJlc3VsdHM7CiAgfSBjYXRjaCAoX2UpIHsKICAgIGNvbnN0IHNlYXJjaF9wYXRoID0gYCR7cGF0aH1cXFdpbmRvd3MuZGJgOwogICAgY29uc3Qgc3RhdHVzID0gRGVuby5sc3RhdFN5bmMoc2VhcmNoX3BhdGgpOwogICAgaWYgKCFzdGF0dXMuaXNGaWxlKSB7CiAgICAgIHJldHVybiBbXTsKICAgIH0KICAgIGNvbnN0IHJlc3VsdHMgPSBnZXRTZWFyY2goc2VhcmNoX3BhdGgpOwogICAgcmV0dXJuIHJlc3VsdHM7CiAgfQp9Cm1haW4oKTsK";
        let mut output = output_options("runtime_test", "local", "./tmp", false);
        let script = JSScript {
            name: String::from("search"),
            script: test.to_string(),
        };
        execute_script(&mut output, &script).unwrap();
    }
}
