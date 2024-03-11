use crate::{
    artifacts::os::windows::search::parser::grab_search_path, runtime::error::RuntimeError,
};
use deno_core::{error::AnyError, op2};
use log::error;

#[op2]
#[string]
/// Expose parsing Windows Search to `Deno`
pub(crate) fn get_search(#[string] path: String) -> Result<String, AnyError> {
    if path.is_empty() {
        error!("[runtime] Empty path to Search file");
        return Err(RuntimeError::ExecuteScript.into());
    }
    let search = grab_search_path(&path)?;

    let results = serde_json::to_string(&search)?;
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
        let test = "Ly8gaHR0cHM6Ly9yYXcuZ2l0aHVidXNlcmNvbnRlbnQuY29tL3B1ZmZ5Y2lkL2FydGVtaXMtYXBpL21hc3Rlci9zcmMvd2luZG93cy9zZWFyY2gudHMKZnVuY3Rpb24gZ2V0U2VhcmNoKHBhdGgpIHsKICBjb25zdCBkYXRhID0gRGVuby5jb3JlLm9wcy5nZXRfc2VhcmNoKHBhdGgpOwogIGNvbnN0IHJlc3VsdCA9IEpTT04ucGFyc2UoZGF0YSk7CiAgcmV0dXJuIHJlc3VsdDsKfQoKLy8gaHR0cHM6Ly9yYXcuZ2l0aHVidXNlcmNvbnRlbnQuY29tL3B1ZmZ5Y2lkL2FydGVtaXMtYXBpL21hc3Rlci9zcmMvZW52aXJvbm1lbnQvZW52LnRzCmZ1bmN0aW9uIGdldEVudlZhbHVlKGtleSkgewogIGNvbnN0IGRhdGEgPSBlbnYuZW52aXJvbm1lbnRWYWx1ZShrZXkpOwogIHJldHVybiBkYXRhOwp9CgovLyBodHRwczovL3Jhdy5naXRodWJ1c2VyY29udGVudC5jb20vcHVmZnljaWQvYXJ0ZW1pcy1hcGkvbWFzdGVyL3NyYy9maWxlc3lzdGVtL2ZpbGVzLnRzCmZ1bmN0aW9uIHN0YXQocGF0aCkgewogIGNvbnN0IGRhdGEgPSBmcy5zdGF0KHBhdGgpOwogIGNvbnN0IHZhbHVlID0gSlNPTi5wYXJzZShkYXRhKTsKICByZXR1cm4gdmFsdWU7Cn0KCi8vIG1haW4udHMKZnVuY3Rpb24gbWFpbigpIHsKICBjb25zdCBkcml2ZSA9IGdldEVudlZhbHVlKCJTeXN0ZW1Ecml2ZSIpOwogIGlmIChkcml2ZSA9PT0gIiIpIHsKICAgIHJldHVybiBbXTsKICB9CiAgY29uc3QgcGF0aCA9IGAke2RyaXZlfVxcUHJvZ3JhbURhdGFcXE1pY3Jvc29mdFxcU2VhcmNoXFxEYXRhXFxBcHBsaWNhdGlvbnNcXFdpbmRvd3NgOwogIHRyeSB7CiAgICBjb25zdCBzZWFyY2hfcGF0aCA9IGAke3BhdGh9XFxXaW5kb3dzLmVkYmA7CiAgICBjb25zdCBzdGF0dXMgPSBzdGF0KHNlYXJjaF9wYXRoKTsKICAgIGlmICghc3RhdHVzLmlzX2ZpbGUpIHsKICAgICAgcmV0dXJuIFtdOwogICAgfQogICAgY29uc3QgcmVzdWx0cyA9IGdldFNlYXJjaChzZWFyY2hfcGF0aCk7CiAgICByZXR1cm4gcmVzdWx0czsKICB9IGNhdGNoIChfZSkgewogICAgY29uc3Qgc2VhcmNoX3BhdGggPSBgJHtwYXRofVxcV2luZG93cy5kYmA7CiAgICBjb25zdCBzdGF0dXMgPSBzdGF0KHNlYXJjaF9wYXRoKTsKICAgIGlmICghc3RhdHVzLmlzX2ZpbGUpIHsKICAgICAgcmV0dXJuIFtdOwogICAgfQogICAgY29uc3QgcmVzdWx0cyA9IGdldFNlYXJjaChzZWFyY2hfcGF0aCk7CiAgICByZXR1cm4gcmVzdWx0czsKICB9Cn0KbWFpbigpOwo=";
        let mut output = output_options("runtime_test", "local", "./tmp", false);
        let script = JSScript {
            name: String::from("search"),
            script: test.to_string(),
        };
        execute_script(&mut output, &script).unwrap();
    }
}
