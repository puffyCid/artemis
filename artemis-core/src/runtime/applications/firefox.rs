use crate::artifacts::applications::firefox;
use deno_core::{error::AnyError, op2};

#[op2]
#[string]
/// Get `Firefox` history for all users
pub(crate) fn get_firefox_users_history() -> Result<String, AnyError> {
    let history = firefox::history::get_firefox_history()?;
    let results = serde_json::to_string(&history)?;
    Ok(results)
}

#[op2]
#[string]
/// Get `Firefox` history from provided path
pub(crate) fn get_firefox_history(#[string] path: String) -> Result<String, AnyError> {
    let history = firefox::history::history_query(&path)?;
    let results = serde_json::to_string(&history)?;
    Ok(results)
}

#[op2]
#[string]
/// Get `Firefox` downloads for all users
pub(crate) fn get_firefox_users_downloads() -> Result<String, AnyError> {
    let downloads = firefox::downloads::get_firefox_downloads()?;

    let results = serde_json::to_string(&downloads)?;
    Ok(results)
}

#[op2]
#[string]
/// Get `Firefox` downloads from provided path
pub(crate) fn get_firefox_downloads(#[string] path: String) -> Result<String, AnyError> {
    let downloads = firefox::downloads::downloads_query(&path)?;
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
    fn test_get_firefox_users_history() {
        let test = "Ly8gLi4vLi4vYXJ0ZW1pcy1hcGkvc3JjL2FwcGxpY2F0aW9ucy9maXJlZm94LnRzCmZ1bmN0aW9uIGdldF9maXJlZm94X3VzZXJzX2hpc3RvcnkoKSB7CiAgY29uc3QgZGF0YSA9IERlbm8uY29yZS5vcHMuZ2V0X2ZpcmVmb3hfdXNlcnNfaGlzdG9yeSgpOwogIGNvbnN0IGhpc3RvcnkgPSBKU09OLnBhcnNlKGRhdGEpOwogIHJldHVybiBoaXN0b3J5Owp9CgovLyAuLi8uLi9hcnRlbWlzLWFwaS9tb2QudHMKZnVuY3Rpb24gZ2V0RmlyZWZveFVzZXJzSGlzdG9yeSgpIHsKICByZXR1cm4gZ2V0X2ZpcmVmb3hfdXNlcnNfaGlzdG9yeSgpOwp9CgovLyBtYWluLnRzCmZ1bmN0aW9uIG1haW4oKSB7CiAgY29uc3QgZGF0YSA9IGdldEZpcmVmb3hVc2Vyc0hpc3RvcnkoKTsKICByZXR1cm4gZGF0YTsKfQptYWluKCk7Cg==";
        let mut output = output_options("runtime_test", "local", "./tmp", false);
        let script = JSScript {
            name: String::from("firefox_history"),
            script: test.to_string(),
        };
        execute_script(&mut output, &script).unwrap();
    }

    #[test]
    fn test_get_firefox_users_downloads() {
        let test = "Ly8gLi4vLi4vYXJ0ZW1pcy1hcGkvc3JjL2FwcGxpY2F0aW9ucy9maXJlZm94LnRzCmZ1bmN0aW9uIGdldF9maXJlZm94X3VzZXJzX2Rvd25sb2FkcygpIHsKICBjb25zdCBkYXRhID0gRGVuby5jb3JlLm9wcy5nZXRfZmlyZWZveF91c2Vyc19kb3dubG9hZHMoKTsKICBjb25zdCBkb3dubG9hZHMgPSBKU09OLnBhcnNlKGRhdGEpOwogIHJldHVybiBkb3dubG9hZHM7Cn0KCi8vIC4uLy4uL2FydGVtaXMtYXBpL21vZC50cwpmdW5jdGlvbiBnZXRGaXJlZm94VXNlcnNEb3dubG9hZHMoKSB7CiAgcmV0dXJuIGdldF9maXJlZm94X3VzZXJzX2Rvd25sb2FkcygpOwp9CgovLyBtYWluLnRzCmZ1bmN0aW9uIG1haW4oKSB7CiAgcmV0dXJuIGdldEZpcmVmb3hVc2Vyc0Rvd25sb2FkcygpOwp9Cm1haW4oKTsK";
        let mut output = output_options("runtime_test", "local", "./tmp", false);
        let script = JSScript {
            name: String::from("firefox_downloads"),
            script: test.to_string(),
        };
        execute_script(&mut output, &script).unwrap();
    }

    #[test]
    #[cfg(target_os = "macos")]
    fn test_get_firefox_history() {
        let test = "Ly8gaHR0cHM6Ly9yYXcuZ2l0aHVidXNlcmNvbnRlbnQuY29tL3B1ZmZ5Y2lkL2FydGVtaXMtYXBpL21hc3Rlci9zcmMvZmlsZXN5c3RlbS9maWxlcy50cwpmdW5jdGlvbiBnbG9iKHBhdHRlcm4pIHsKICBjb25zdCByZXN1bHQgPSBmcy5nbG9iKHBhdHRlcm4pOwogIGlmIChyZXN1bHQgaW5zdGFuY2VvZiBFcnJvcikgewogICAgcmV0dXJuIHJlc3VsdDsKICB9CiAgY29uc3QgZGF0YSA9IEpTT04ucGFyc2UocmVzdWx0KTsKICByZXR1cm4gZGF0YTsKfQoKLy8gaHR0cHM6Ly9yYXcuZ2l0aHVidXNlcmNvbnRlbnQuY29tL3B1ZmZ5Y2lkL2FydGVtaXMtYXBpL21hc3Rlci9zcmMvYXBwbGljYXRpb25zL2ZpcmVmb3gudHMKZnVuY3Rpb24gZ2V0RmlyZWZveEhpc3RvcnkocGF0aCkgewogIGNvbnN0IGRhdGEgPSBEZW5vLmNvcmUub3BzLmdldF9maXJlZm94X2hpc3RvcnkocGF0aCk7CiAgY29uc3QgaGlzdG9yeSA9IEpTT04ucGFyc2UoZGF0YSk7CiAgcmV0dXJuIGhpc3Rvcnk7Cn0KCi8vIG1haW4udHMKZnVuY3Rpb24gbWFpbigpIHsKICBjb25zdCBwYXRocyA9IGdsb2IoIi9Vc2Vycy8qL0xpYnJhcnkvQXBwbGljYXRpb24gU3VwcG9ydC9GaXJlZm94L1Byb2ZpbGVzLyouZGVmYXVsdC1yZWxlYXNlL3BsYWNlcy5zcWxpdGUiKTsKICBpZiAocGF0aHMgaW5zdGFuY2VvZiBFcnJvcikgewogICAgcmV0dXJuOwogIH0KICBmb3IgKGNvbnN0IHBhdGggb2YgcGF0aHMpIHsKICAgIHJldHVybiBnZXRGaXJlZm94SGlzdG9yeShwYXRoLmZ1bGxfcGF0aCk7CiAgfQp9Cm1haW4oKTsK";
        let mut output = output_options("runtime_test", "local", "./tmp", false);
        let script = JSScript {
            name: String::from("firefox_history_path"),
            script: test.to_string(),
        };
        execute_script(&mut output, &script).unwrap();
    }

    #[test]
    #[cfg(target_os = "windows")]
    fn test_get_firefox_history() {
        let test = "Ly8gaHR0cHM6Ly9yYXcuZ2l0aHVidXNlcmNvbnRlbnQuY29tL3B1ZmZ5Y2lkL2FydGVtaXMtYXBpL21hc3Rlci9zcmMvZmlsZXN5c3RlbS9maWxlcy50cwpmdW5jdGlvbiBnbG9iKHBhdHRlcm4pIHsKICBjb25zdCByZXN1bHQgPSBmcy5nbG9iKHBhdHRlcm4pOwogIGlmIChyZXN1bHQgaW5zdGFuY2VvZiBFcnJvcikgewogICAgcmV0dXJuIHJlc3VsdDsKICB9CiAgY29uc3QgZGF0YSA9IEpTT04ucGFyc2UocmVzdWx0KTsKICByZXR1cm4gZGF0YTsKfQoKLy8gaHR0cHM6Ly9yYXcuZ2l0aHVidXNlcmNvbnRlbnQuY29tL3B1ZmZ5Y2lkL2FydGVtaXMtYXBpL21hc3Rlci9zcmMvYXBwbGljYXRpb25zL2ZpcmVmb3gudHMKZnVuY3Rpb24gZ2V0RmlyZWZveEhpc3RvcnkocGF0aCkgewogIGNvbnN0IGRhdGEgPSBEZW5vLmNvcmUub3BzLmdldF9maXJlZm94X2hpc3RvcnkocGF0aCk7CiAgY29uc3QgaGlzdG9yeSA9IEpTT04ucGFyc2UoZGF0YSk7CiAgcmV0dXJuIGhpc3Rvcnk7Cn0KCi8vIG1haW4udHMKZnVuY3Rpb24gbWFpbigpIHsKICBjb25zdCBwYXRocyA9IGdsb2IoIkM6XFxVc2Vyc1xcKlxcQXBwRGF0YVxcUm9hbWluZ1xcTW96aWxsYVxcRmlyZWZveFxcUHJvZmlsZXMqLmRlZmF1bHQtcmVsZWFzZS9wbGFjZXMuc3FsaXRlIik7CiAgaWYgKHBhdGhzIGluc3RhbmNlb2YgRXJyb3IpIHsKICAgIHJldHVybjsKICB9CiAgZm9yIChjb25zdCBwYXRoIG9mIHBhdGhzKSB7CiAgICByZXR1cm4gZ2V0RmlyZWZveEhpc3RvcnkocGF0aC5mdWxsX3BhdGgpOwogIH0KfQptYWluKCk7Cg==";
        let mut output = output_options("runtime_test", "local", "./tmp", false);
        let script = JSScript {
            name: String::from("firefox_history_path"),
            script: test.to_string(),
        };
        execute_script(&mut output, &script).unwrap();
    }

    #[test]
    #[cfg(target_os = "macos")]
    fn test_get_firefox_downloads() {
        let test = "Ly8gaHR0cHM6Ly9yYXcuZ2l0aHVidXNlcmNvbnRlbnQuY29tL3B1ZmZ5Y2lkL2FydGVtaXMtYXBpL21hc3Rlci9zcmMvZmlsZXN5c3RlbS9maWxlcy50cwpmdW5jdGlvbiBnbG9iKHBhdHRlcm4pIHsKICBjb25zdCByZXN1bHQgPSBmcy5nbG9iKHBhdHRlcm4pOwogIGlmIChyZXN1bHQgaW5zdGFuY2VvZiBFcnJvcikgewogICAgcmV0dXJuIHJlc3VsdDsKICB9CiAgY29uc3QgZGF0YSA9IEpTT04ucGFyc2UocmVzdWx0KTsKICByZXR1cm4gZGF0YTsKfQoKLy8gaHR0cHM6Ly9yYXcuZ2l0aHVidXNlcmNvbnRlbnQuY29tL3B1ZmZ5Y2lkL2FydGVtaXMtYXBpL21hc3Rlci9zcmMvYXBwbGljYXRpb25zL2ZpcmVmb3gudHMKZnVuY3Rpb24gZ2V0RmlyZWZveERvd25sb2FkcyhwYXRoKSB7CiAgY29uc3QgZGF0YSA9IERlbm8uY29yZS5vcHMuZ2V0X2ZpcmVmb3hfZG93bmxvYWRzKHBhdGgpOwogIGNvbnN0IGRvd25sb2FkcyA9IEpTT04ucGFyc2UoZGF0YSk7CiAgcmV0dXJuIGRvd25sb2FkczsKfQoKLy8gbWFpbi50cwpmdW5jdGlvbiBtYWluKCkgewogIGNvbnN0IHBhdGhzID0gZ2xvYigiL1VzZXJzLyovTGlicmFyeS9BcHBsaWNhdGlvbiBTdXBwb3J0L0ZpcmVmb3gvUHJvZmlsZXMvKi5kZWZhdWx0LXJlbGVhc2UvcGxhY2VzLnNxbGl0ZSIpOwogIGlmIChwYXRocyBpbnN0YW5jZW9mIEVycm9yKSB7CiAgICByZXR1cm47CiAgfQogIGZvciAoY29uc3QgcGF0aCBvZiBwYXRocykgewogICAgcmV0dXJuIGdldEZpcmVmb3hEb3dubG9hZHMocGF0aC5mdWxsX3BhdGgpOwogIH0KfQptYWluKCk7Cg==";
        let mut output = output_options("runtime_test", "local", "./tmp", false);
        let script = JSScript {
            name: String::from("firefox_downloads_path"),
            script: test.to_string(),
        };
        execute_script(&mut output, &script).unwrap();
    }

    #[test]
    #[cfg(target_os = "windows")]
    fn test_get_firefox_downloads() {
        let test = "Ly8gaHR0cHM6Ly9yYXcuZ2l0aHVidXNlcmNvbnRlbnQuY29tL3B1ZmZ5Y2lkL2FydGVtaXMtYXBpL21hc3Rlci9zcmMvZmlsZXN5c3RlbS9maWxlcy50cwpmdW5jdGlvbiBnbG9iKHBhdHRlcm4pIHsKICBjb25zdCByZXN1bHQgPSBmcy5nbG9iKHBhdHRlcm4pOwogIGlmIChyZXN1bHQgaW5zdGFuY2VvZiBFcnJvcikgewogICAgcmV0dXJuIHJlc3VsdDsKICB9CiAgY29uc3QgZGF0YSA9IEpTT04ucGFyc2UocmVzdWx0KTsKICByZXR1cm4gZGF0YTsKfQoKLy8gaHR0cHM6Ly9yYXcuZ2l0aHVidXNlcmNvbnRlbnQuY29tL3B1ZmZ5Y2lkL2FydGVtaXMtYXBpL21hc3Rlci9zcmMvYXBwbGljYXRpb25zL2ZpcmVmb3gudHMKZnVuY3Rpb24gZ2V0RmlyZWZveERvd25sb2FkcyhwYXRoKSB7CiAgY29uc3QgZGF0YSA9IERlbm8uY29yZS5vcHMuZ2V0X2ZpcmVmb3hfZG93bmxvYWRzKHBhdGgpOwogIGNvbnN0IGRvd25sb2FkcyA9IEpTT04ucGFyc2UoZGF0YSk7CiAgcmV0dXJuIGRvd25sb2FkczsKfQoKLy8gbWFpbi50cwpmdW5jdGlvbiBtYWluKCkgewogIGNvbnN0IHBhdGhzID0gZ2xvYigiQzpcXFVzZXJzXFwqXFxBcHBEYXRhXFxSb2FtaW5nXFxNb3ppbGxhXFxGaXJlZm94XFxQcm9maWxlcyouZGVmYXVsdC1yZWxlYXNlL3BsYWNlcy5zcWxpdGUiKTsKICBpZiAocGF0aHMgaW5zdGFuY2VvZiBFcnJvcikgewogICAgcmV0dXJuOwogIH0KICBmb3IgKGNvbnN0IHBhdGggb2YgcGF0aHMpIHsKICAgIHJldHVybiBnZXRGaXJlZm94RG93bmxvYWRzKHBhdGguZnVsbF9wYXRoKTsKICB9Cn0KbWFpbigpOwo=";
        let mut output = output_options("runtime_test", "local", "./tmp", false);
        let script = JSScript {
            name: String::from("firefox_downloads_path"),
            script: test.to_string(),
        };
        execute_script(&mut output, &script).unwrap();
    }
}
