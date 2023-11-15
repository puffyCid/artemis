use crate::runtime::error::RuntimeError;
use deno_core::{error::AnyError, op2};
use log::error;
use macos_unifiedlogs::{
    dsc::SharedCacheStrings,
    parser::{
        build_log, collect_shared_strings_system, collect_strings_system, collect_timesync_system,
        parse_log,
    },
    timesync::TimesyncBoot,
    unified_log::LogData,
    uuidtext::UUIDText,
};

#[op2]
#[string]
/// Expose Unified Log parsing to `Deno`
pub(crate) fn get_unified_log(#[string] path: String) -> Result<String, AnyError> {
    // Not ideal but for now we have to parse the Unified Log metadata each time we want to parse a log file
    // Fortunately the metadata logs are really small and are parsed very quickly
    let strings = collect_strings_system()?;
    let shared_strings = collect_shared_strings_system()?;
    let timesync_data = collect_timesync_system()?;

    let logs = parse_trace_file(&strings, &shared_strings, &timesync_data, &path)?;

    let results = serde_json::to_string(&logs)?;
    Ok(results)
}

/// Parse the provided log (trace) file
fn parse_trace_file(
    string_results: &[UUIDText],
    shared_strings_results: &[SharedCacheStrings],
    timesync_data: &[TimesyncBoot],
    path: &str,
) -> Result<Vec<LogData>, RuntimeError> {
    let log_result = parse_log(path);
    let log_data = match log_result {
        Ok(results) => results,
        Err(err) => {
            error!("[runtime] Failed to parse {path} log entry: {err:?}");
            return Err(RuntimeError::ExecuteScript);
        }
    };

    let exclude_missing = false;
    let (results, _) = build_log(
        &log_data,
        string_results,
        shared_strings_results,
        timesync_data,
        exclude_missing,
    );
    Ok(results)
}

#[cfg(test)]
mod tests {
    use super::parse_trace_file;
    use crate::{
        filesystem::files::list_files, runtime::deno::execute_script,
        structs::artifacts::runtime::script::JSScript, structs::toml::Output,
    };
    use macos_unifiedlogs::parser::{
        collect_shared_strings_system, collect_strings_system, collect_timesync_system,
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
    fn test_get_unified_log() {
        let test = "Ly8gaHR0cHM6Ly9yYXcuZ2l0aHVidXNlcmNvbnRlbnQuY29tL3B1ZmZ5Y2lkL2FydGVtaXMtYXBpL21hc3Rlci9zcmMvbWFjb3MvdW5pZmllZGxvZ3MudHMKZnVuY3Rpb24gZ2V0VW5pZmllZExvZyhwYXRoKSB7CiAgY29uc3QgZGF0YSA9IERlbm8uY29yZS5vcHMuZ2V0X3VuaWZpZWRfbG9nKHBhdGgpOwogIGNvbnN0IGxvZ19kYXRhID0gSlNPTi5wYXJzZShkYXRhKTsKICByZXR1cm4gbG9nX2RhdGE7Cn0KCi8vIGh0dHBzOi8vcmF3LmdpdGh1YnVzZXJjb250ZW50LmNvbS9wdWZmeWNpZC9hcnRlbWlzLWFwaS9tYXN0ZXIvc3JjL2ZpbGVzeXN0ZW0vZGlyZWN0b3J5LnRzCmFzeW5jIGZ1bmN0aW9uIHJlYWREaXIocGF0aCkgewogIGNvbnN0IHJlc3VsdCA9IGF3YWl0IGZzLnJlYWREaXIocGF0aCk7CiAgaWYgKHJlc3VsdCBpbnN0YW5jZW9mIEVycm9yKSB7CiAgICByZXR1cm4gcmVzdWx0OwogIH0KICBjb25zdCBkYXRhID0gSlNPTi5wYXJzZShyZXN1bHQpOwogIHJldHVybiBkYXRhOwp9CgovLyBtYWluLnRzCmFzeW5jIGZ1bmN0aW9uIG1haW4oKSB7CiAgY29uc3QgcGF0aCA9ICIvdmFyL2RiL2RpYWdub3N0aWNzL1NwZWNpYWwiOwogIGNvbnN0IHhwcm90ZWN0X2VudHJpZXMgPSBbXTsKICBjb25zdCByZXN1bHQgPSBhd2FpdCByZWFkRGlyKHBhdGgpOwogIGlmIChyZXN1bHQgaW5zdGFuY2VvZiBFcnJvcikgewogICAgcmV0dXJuOwogIH0KICBmb3IgKGNvbnN0IGVudHJ5IG9mIHJlc3VsdCkgewogICAgaWYgKCFlbnRyeS5pc19maWxlKSB7CiAgICAgIGNvbnRpbnVlOwogICAgfQogICAgY29uc3QgcGVyc2lzdF9maWxlID0gZW50cnkuZmlsZW5hbWU7CiAgICBjb25zdCBwZXJzaXN0X2Z1bGxfcGF0aCA9IGAke3BhdGh9LyR7cGVyc2lzdF9maWxlfWA7CiAgICBjb25zdCBsb2dzID0gZ2V0VW5pZmllZExvZyhwZXJzaXN0X2Z1bGxfcGF0aCk7CiAgICBmb3IgKGxldCBsb2dfZW50cnkgPSAwOyBsb2dfZW50cnkgPCBsb2dzLmxlbmd0aDsgbG9nX2VudHJ5KyspIHsKICAgICAgaWYgKCFsb2dzW2xvZ19lbnRyeV0ubWVzc2FnZS50b0xvd2VyQ2FzZSgpLmluY2x1ZGVzKCJ4cHJvdGVjdCIpKSB7CiAgICAgICAgY29udGludWU7CiAgICAgIH0KICAgICAgeHByb3RlY3RfZW50cmllcy5wdXNoKGxvZ3NbbG9nX2VudHJ5XSk7CiAgICB9CiAgfQogIHJldHVybiB4cHJvdGVjdF9lbnRyaWVzOwp9Cm1haW4oKTsK";
        let mut output = output_options("runtime_test", "local", "./tmp", true);
        let script = JSScript {
            name: String::from("xprotect_entries"),
            script: test.to_string(),
        };
        let _ = execute_script(&mut output, &script).unwrap();
    }

    #[test]
    fn test_parse_trace_file() {
        let strings_results = collect_strings_system().unwrap();
        let shared_strings_results = collect_shared_strings_system().unwrap();
        let timesync_data_results = collect_timesync_system().unwrap();

        let files = list_files("/var/db/diagnostics/Persist").unwrap();
        for file in files {
            let result = parse_trace_file(
                &strings_results,
                &shared_strings_results,
                &timesync_data_results,
                &file,
            )
            .unwrap();
            assert!(result.len() > 2000);
            break;
        }
    }
}
