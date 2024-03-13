use crate::artifacts::applications::chromium;
use deno_core::{error::AnyError, op2};

#[op2]
#[string]
/// Get `Chromium` history for all users
pub(crate) fn get_chromium_users_history() -> Result<String, AnyError> {
    let history = chromium::history::get_chromium_history()?;
    let results = serde_json::to_string(&history)?;
    Ok(results)
}

#[op2]
#[string]
/// Get `Chromium` history from provided path
pub(crate) fn get_chromium_history(#[string] path: String) -> Result<String, AnyError> {
    let history = chromium::history::history_query(&path)?;
    let results = serde_json::to_string(&history)?;
    Ok(results)
}

#[op2]
#[string]
/// Get `Chromium` downloads for all users
pub(crate) fn get_chromium_users_downloads() -> Result<String, AnyError> {
    let downloads = chromium::downloads::get_chromium_downloads()?;
    let results = serde_json::to_string(&downloads)?;
    Ok(results)
}

#[op2]
#[string]
/// Get `Chromium` downloads from provided path
pub(crate) fn get_chromium_downloads(#[string] path: String) -> Result<String, AnyError> {
    let downloads = chromium::downloads::downloads_query(&path)?;
    let results = serde_json::to_string(&downloads)?;
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
            filter_name: Some(String::new()),
            filter_script: Some(String::new()),
            logging: Some(String::new()),
        }
    }

    #[test]
    fn test_get_chromium_users_history() {
        let test = "Ly8gZGVuby1mbXQtaWdub3JlLWZpbGUKLy8gZGVuby1saW50LWlnbm9yZS1maWxlCi8vIFRoaXMgY29kZSB3YXMgYnVuZGxlZCB1c2luZyBgZGVubyBidW5kbGVgIGFuZCBpdCdzIG5vdCByZWNvbW1lbmRlZCB0byBlZGl0IGl0IG1hbnVhbGx5CgpmdW5jdGlvbiBnZXRfY2hyb21pdW1fdXNlcnNfaGlzdG9yeSgpIHsKICAgIGNvbnN0IGRhdGEgPSBEZW5vLmNvcmUub3BzLmdldF9jaHJvbWl1bV91c2Vyc19oaXN0b3J5KCk7CiAgICBjb25zdCBoaXN0b3J5ID0gSlNPTi5wYXJzZShkYXRhKTsKICAgIHJldHVybiBoaXN0b3J5Owp9CmZ1bmN0aW9uIGdldENocm9taXVtVXNlcnNIaXN0b3J5KCkgewogICAgcmV0dXJuIGdldF9jaHJvbWl1bV91c2Vyc19oaXN0b3J5KCk7Cn0KZnVuY3Rpb24gbWFpbigpIHsKICAgIGNvbnN0IGRhdGEgPSBnZXRDaHJvbWl1bVVzZXJzSGlzdG9yeSgpOwogICAgcmV0dXJuIGRhdGE7Cn0KbWFpbigpOw==";
        let mut output = output_options("runtime_test", "local", "./tmp", false);
        let script = JSScript {
            name: String::from("chromium_history"),
            script: test.to_string(),
        };
        execute_script(&mut output, &script).unwrap();
    }

    #[test]
    #[cfg(target_os = "macos")]
    fn test_get_chromium_history() {
        let test = "aW1wb3J0IHsgZ2V0Q2hyb21pdW1IaXN0b3J5IH0gZnJvbSAiaHR0cHM6Ly9yYXcuZ2l0aHVidXNlcmNvbnRlbnQuY29tL3B1ZmZ5Y2lkL2FydGVtaXMtYXBpL21hc3Rlci9tb2QudHMiOwppbXBvcnQgeyBnbG9iIH0gZnJvbSAiaHR0cHM6Ly9yYXcuZ2l0aHVidXNlcmNvbnRlbnQuY29tL3B1ZmZ5Y2lkL2FydGVtaXMtYXBpL21hc3Rlci9zcmMvZmlsZXN5c3RlbS9tb2QudHMiOwoKZnVuY3Rpb24gbWFpbigpIHsKICBjb25zdCBwYXRocyA9IGdsb2IoCiAgICAiL1VzZXJzLyovTGlicmFyeS9BcHBsaWNhdGlvbiBTdXBwb3J0L0Nocm9taXVtL0RlZmF1bHQvSGlzdG9yeSIsCiAgKTsKICBpZiAocGF0aHMgaW5zdGFuY2VvZiBFcnJvcikgewogICAgcmV0dXJuOwogIH0KCiAgZm9yIChjb25zdCBwYXRoIG9mIHBhdGhzKSB7CiAgICByZXR1cm4gZ2V0Q2hyb21pdW1IaXN0b3J5KHBhdGguZnVsbF9wYXRoKTsKICB9Cn0KCm1haW4oKTsK";
        let mut output = output_options("runtime_test", "local", "./tmp", false);
        let script = JSScript {
            name: String::from("chromium_path_history"),
            script: test.to_string(),
        };
        execute_script(&mut output, &script).unwrap();
    }

    #[test]
    #[cfg(target_os = "windows")]
    fn test_get_chromium_history() {
        let test = "aW1wb3J0IHsgZ2V0Q2hyb21pdW1IaXN0b3J5IH0gZnJvbSAiaHR0cHM6Ly9yYXcuZ2l0aHVidXNlcmNvbnRlbnQuY29tL3B1ZmZ5Y2lkL2FydGVtaXMtYXBpL21hc3Rlci9tb2QudHMiOwppbXBvcnQgeyBnbG9iIH0gZnJvbSAiaHR0cHM6Ly9yYXcuZ2l0aHVidXNlcmNvbnRlbnQuY29tL3B1ZmZ5Y2lkL2FydGVtaXMtYXBpL21hc3Rlci9zcmMvZmlsZXN5c3RlbS9tb2QudHMiOwoKZnVuY3Rpb24gbWFpbigpIHsKICBjb25zdCBwYXRocyA9IGdsb2IoCiAgICAiQzpcXFVzZXJzXFwqXFxBcHBEYXRhXFxMb2NhbFxcQ2hyb21pdW1cXFVzZXIgRGF0YVxcRGVmYXVsdFxcSGlzdG9yeSIsCiAgKTsKICBpZiAocGF0aHMgaW5zdGFuY2VvZiBFcnJvcikgewogICAgcmV0dXJuOwogIH0KCiAgZm9yIChjb25zdCBwYXRoIG9mIHBhdGhzKSB7CiAgICByZXR1cm4gZ2V0Q2hyb21pdW1IaXN0b3J5KHBhdGguZnVsbF9wYXRoKTsKICB9Cn0KCm1haW4oKTsK";
        let mut output = output_options("runtime_test", "local", "./tmp", false);
        let script = JSScript {
            name: String::from("chromium_path_history"),
            script: test.to_string(),
        };
        execute_script(&mut output, &script).unwrap();
    }

    #[test]
    fn test_get_chromium_users_downloads() {
        let test = "Ly8gLi4vLi4vYXJ0ZW1pcy1hcGkvc3JjL2FwcGxpY2F0aW9ucy9jaHJvbWl1bS50cwpmdW5jdGlvbiBnZXRfY2hyb21pdW1fdXNlcnNfZG93bmxvYWRzKCkgewogIGNvbnN0IGRhdGEgPSBEZW5vLmNvcmUub3BzLmdldF9jaHJvbWl1bV91c2Vyc19kb3dubG9hZHMoKTsKICBjb25zdCBkb3dubG9hZHMgPSBKU09OLnBhcnNlKGRhdGEpOwogIHJldHVybiBkb3dubG9hZHM7Cn0KCi8vIC4uLy4uL2FydGVtaXMtYXBpL21vZC50cwpmdW5jdGlvbiBnZXRDaHJvbWl1bVVzZXJzRG93bmxvYWRzKCkgewogIHJldHVybiBnZXRfY2hyb21pdW1fdXNlcnNfZG93bmxvYWRzKCk7Cn0KCi8vIG1haW4udHMKZnVuY3Rpb24gbWFpbigpIHsKICByZXR1cm4gZ2V0Q2hyb21pdW1Vc2Vyc0Rvd25sb2FkcygpOwp9Cm1haW4oKTsK";
        let mut output = output_options("runtime_test", "local", "./tmp", false);
        let script = JSScript {
            name: String::from("chromium_downloads"),
            script: test.to_string(),
        };
        execute_script(&mut output, &script).unwrap();
    }

    #[test]
    #[cfg(target_os = "macos")]
    fn test_get_chromium_downloads() {
        let test = "Ly8gaHR0cHM6Ly9yYXcuZ2l0aHVidXNlcmNvbnRlbnQuY29tL3B1ZmZ5Y2lkL2FydGVtaXMtYXBpL21hc3Rlci9zcmMvZmlsZXN5c3RlbS9maWxlcy50cwpmdW5jdGlvbiBnbG9iKHBhdHRlcm4pIHsKICBjb25zdCByZXN1bHQgPSBmcy5nbG9iKHBhdHRlcm4pOwogIGlmIChyZXN1bHQgaW5zdGFuY2VvZiBFcnJvcikgewogICAgcmV0dXJuIHJlc3VsdDsKICB9CiAgY29uc3QgZGF0YSA9IEpTT04ucGFyc2UocmVzdWx0KTsKICByZXR1cm4gZGF0YTsKfQoKLy8gaHR0cHM6Ly9yYXcuZ2l0aHVidXNlcmNvbnRlbnQuY29tL3B1ZmZ5Y2lkL2FydGVtaXMtYXBpL21hc3Rlci9zcmMvYXBwbGljYXRpb25zL2Nocm9taXVtLnRzCmZ1bmN0aW9uIGdldENocm9taXVtRG93bmxvYWRzKHBhdGgpIHsKICBjb25zdCBkYXRhID0gRGVuby5jb3JlLm9wcy5nZXRfY2hyb21pdW1fZG93bmxvYWRzKHBhdGgpOwogIGNvbnN0IGRvd25sb2FkcyA9IEpTT04ucGFyc2UoZGF0YSk7CiAgcmV0dXJuIGRvd25sb2FkczsKfQoKLy8gbWFpbi50cwphc3luYyBmdW5jdGlvbiBtYWluKCkgewogIGNvbnN0IHBhdGhzID0gZ2xvYigiL1VzZXJzLyovTGlicmFyeS9BcHBsaWNhdGlvbiBTdXBwb3J0L0Nocm9taXVtL0RlZmF1bHQvSGlzdG9yeSIpOwogIGlmIChwYXRocyBpbnN0YW5jZW9mIEVycm9yKSB7CiAgICByZXR1cm47CiAgfQogIGZvciAoY29uc3QgcGF0aCBvZiBwYXRocykgewogICAgcmV0dXJuIGdldENocm9taXVtRG93bmxvYWRzKHBhdGguZnVsbF9wYXRoKTsKICB9Cn0KbWFpbigpOwo=";
        let mut output = output_options("runtime_test", "local", "./tmp", false);
        let script = JSScript {
            name: String::from("chromium_path_downloads"),
            script: test.to_string(),
        };
        execute_script(&mut output, &script).unwrap();
    }

    #[test]
    #[cfg(target_os = "windows")]
    fn test_get_chromium_downloads() {
        let test = "Ly8gaHR0cHM6Ly9yYXcuZ2l0aHVidXNlcmNvbnRlbnQuY29tL3B1ZmZ5Y2lkL2FydGVtaXMtYXBpL21hc3Rlci9zcmMvZmlsZXN5c3RlbS9maWxlcy50cwpmdW5jdGlvbiBnbG9iKHBhdHRlcm4pIHsKICBjb25zdCByZXN1bHQgPSBmcy5nbG9iKHBhdHRlcm4pOwogIGlmIChyZXN1bHQgaW5zdGFuY2VvZiBFcnJvcikgewogICAgcmV0dXJuIHJlc3VsdDsKICB9CiAgY29uc3QgZGF0YSA9IEpTT04ucGFyc2UocmVzdWx0KTsKICByZXR1cm4gZGF0YTsKfQoKLy8gaHR0cHM6Ly9yYXcuZ2l0aHVidXNlcmNvbnRlbnQuY29tL3B1ZmZ5Y2lkL2FydGVtaXMtYXBpL21hc3Rlci9zcmMvYXBwbGljYXRpb25zL2Nocm9taXVtLnRzCmZ1bmN0aW9uIGdldENocm9taXVtRG93bmxvYWRzKHBhdGgpIHsKICBjb25zdCBkYXRhID0gRGVuby5jb3JlLm9wcy5nZXRfY2hyb21pdW1fZG93bmxvYWRzKHBhdGgpOwogIGNvbnN0IGRvd25sb2FkcyA9IEpTT04ucGFyc2UoZGF0YSk7CiAgcmV0dXJuIGRvd25sb2FkczsKfQoKLy8gbWFpbi50cwphc3luYyBmdW5jdGlvbiBtYWluKCkgewogIGNvbnN0IHBhdGhzID0gZ2xvYigiQzpcXFVzZXJzKlxcQXBwRGF0YVxcTG9jYWxcXENocm9taXVtXFxVc2VyIERhdGFcXERlZmF1bHRcXEhpc3RvcnkiKTsKICBpZiAocGF0aHMgaW5zdGFuY2VvZiBFcnJvcikgewogICAgcmV0dXJuOwogIH0KICBmb3IgKGNvbnN0IHBhdGggb2YgcGF0aHMpIHsKICAgIHJldHVybiBnZXRDaHJvbWl1bURvd25sb2FkcyhwYXRoLmZ1bGxfcGF0aCk7CiAgfQp9Cm1haW4oKTsK";
        let mut output = output_options("runtime_test", "local", "./tmp", false);
        let script = JSScript {
            name: String::from("chromium_path_downloads"),
            script: test.to_string(),
        };
        execute_script(&mut output, &script).unwrap();
    }
}
