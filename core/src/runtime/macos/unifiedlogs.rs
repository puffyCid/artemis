use crate::runtime::error::RuntimeError;
use deno_core::{error::AnyError, op2};
use log::error;
use macos_unifiedlogs::{
    dsc::SharedCacheStrings,
    parser::{
        build_log, collect_shared_strings, collect_shared_strings_system, collect_strings,
        collect_strings_system, collect_timesync, collect_timesync_system, parse_log,
    },
    timesync::TimesyncBoot,
    unified_log::LogData,
    uuidtext::UUIDText,
};

#[op2]
#[string]
/// Expose Unified Log parsing to `Deno`
pub(crate) fn get_unified_log(
    #[string] path: String,
    #[string] archive_path: String,
) -> Result<String, AnyError> {
    let (uuid, shared, timesync) = if archive_path.is_empty() {
        (
            collect_strings_system()?,
            collect_shared_strings_system()?,
            collect_timesync_system()?,
        )
    } else {
        (
            collect_strings(&archive_path)?,
            collect_shared_strings(&format!("{archive_path}/dsc"))?,
            collect_timesync(&format!("{archive_path}/timesync"))?,
        )
    };

    let logs = parse_trace_file(&uuid, &shared, &timesync, &path)?;

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
    fn test_get_unified_log() {
        let test = "Ly8gLi4vLi4vYXJ0ZW1pcy1hcGkvc3JjL3V0aWxzL2Vycm9yLnRzCnZhciBFcnJvckJhc2UgPSBjbGFzcyBleHRlbmRzIEVycm9yIHsKICBjb25zdHJ1Y3RvcihuYW1lLCBtZXNzYWdlKSB7CiAgICBzdXBlcigpOwogICAgdGhpcy5uYW1lID0gbmFtZTsKICAgIHRoaXMubWVzc2FnZSA9IG1lc3NhZ2U7CiAgfQp9OwoKLy8gLi4vLi4vYXJ0ZW1pcy1hcGkvc3JjL21hY29zL2Vycm9ycy50cwp2YXIgTWFjb3NFcnJvciA9IGNsYXNzIGV4dGVuZHMgRXJyb3JCYXNlIHsKfTsKCi8vIC4uLy4uL2FydGVtaXMtYXBpL3NyYy9tYWNvcy91bmlmaWVkbG9ncy50cwpmdW5jdGlvbiBnZXRVbmlmaWVkTG9nKHBhdGgpIHsKICB0cnkgewogICAgY29uc3QgZGF0YSA9IERlbm8uY29yZS5vcHMuZ2V0X3VuaWZpZWRfbG9nKHBhdGgsICIiKTsKICAgIGNvbnN0IGxvZ19kYXRhID0gSlNPTi5wYXJzZShkYXRhKTsKICAgIHJldHVybiBsb2dfZGF0YTsKICB9IGNhdGNoIChlcnIpIHsKICAgIHJldHVybiBuZXcgTWFjb3NFcnJvcigiVU5JRklFRExPR1MiLCBgZmFpbGVkIHRvIHBhcnNlICR7cGF0aH06ICR7ZXJyfWApOwogIH0KfQpmdW5jdGlvbiBzZXR1cFVuaWZpZWRMb2dQYXJzZXIocGF0aCkgewogIGlmIChwYXRoID09PSB2b2lkIDApIHsKICAgIHBhdGggPSAiIjsKICB9CiAgdHJ5IHsKICAgIGNvbnN0IGRhdGEgPSBEZW5vLmNvcmUub3BzLnNldHVwX3VuaWZpZWRfbG9nX3BhcnNlcihwYXRoKTsKICAgIHJldHVybiBkYXRhOwogIH0gY2F0Y2ggKGVycikgewogICAgcmV0dXJuIG5ldyBNYWNvc0Vycm9yKCJVTklGSUVETE9HUyIsIGBmYWlsZWQgdG8gcGFyc2UgJHtwYXRofTogJHtlcnJ9YCk7CiAgfQp9CgovLyBodHRwczovL3Jhdy5naXRodWJ1c2VyY29udGVudC5jb20vcHVmZnljaWQvYXJ0ZW1pcy1hcGkvbWFzdGVyL3NyYy91dGlscy9lcnJvci50cwp2YXIgRXJyb3JCYXNlMiA9IGNsYXNzIGV4dGVuZHMgRXJyb3IgewogIGNvbnN0cnVjdG9yKG5hbWUsIG1lc3NhZ2UpIHsKICAgIHN1cGVyKCk7CiAgICB0aGlzLm5hbWUgPSBuYW1lOwogICAgdGhpcy5tZXNzYWdlID0gbWVzc2FnZTsKICB9Cn07CgovLyBodHRwczovL3Jhdy5naXRodWJ1c2VyY29udGVudC5jb20vcHVmZnljaWQvYXJ0ZW1pcy1hcGkvbWFzdGVyL3NyYy9maWxlc3lzdGVtL2Vycm9ycy50cwp2YXIgRmlsZUVycm9yID0gY2xhc3MgZXh0ZW5kcyBFcnJvckJhc2UyIHsKfTsKCi8vIGh0dHBzOi8vcmF3LmdpdGh1YnVzZXJjb250ZW50LmNvbS9wdWZmeWNpZC9hcnRlbWlzLWFwaS9tYXN0ZXIvc3JjL2ZpbGVzeXN0ZW0vZGlyZWN0b3J5LnRzCmFzeW5jIGZ1bmN0aW9uIHJlYWREaXIocGF0aCkgewogIHRyeSB7CiAgICBjb25zdCByZXN1bHQgPSBhd2FpdCBmcy5yZWFkRGlyKHBhdGgpOwogICAgY29uc3QgZGF0YSA9IEpTT04ucGFyc2UocmVzdWx0KTsKICAgIHJldHVybiBkYXRhOwogIH0gY2F0Y2ggKGVycikgewogICAgcmV0dXJuIG5ldyBGaWxlRXJyb3IoCiAgICAgICJSRUFEX0RJUiIsCiAgICAgIGBmYWlsZWQgdG8gcmVhZCBkaXJlY3RvcnkgJHtwYXRofTogJHtlcnJ9YCwKICAgICk7CiAgfQp9CgovLyBtYWluLnRzCmFzeW5jIGZ1bmN0aW9uIG1haW4oKSB7CiAgY29uc3QgcGF0aCA9ICIvdmFyL2RiL2RpYWdub3N0aWNzL1BlcnNpc3QiOwogIGNvbnN0IHhwcm90ZWN0X2VudHJpZXMgPSBbXTsKICBjb25zdCByZXN1bHQgPSBhd2FpdCByZWFkRGlyKHBhdGgpOwogIGlmIChyZXN1bHQgaW5zdGFuY2VvZiBFcnJvcikgewogICAgcmV0dXJuOwogIH0KCiAgZm9yIChjb25zdCBlbnRyeSBvZiByZXN1bHQpIHsKICAgIGlmICghZW50cnkuaXNfZmlsZSkgewogICAgICBjb250aW51ZTsKICAgIH0KICAgIGNvbnN0IHBlcnNpc3RfZmlsZSA9IGVudHJ5LmZpbGVuYW1lOwogICAgY29uc3QgcGVyc2lzdF9mdWxsX3BhdGggPSBgJHtwYXRofS8ke3BlcnNpc3RfZmlsZX1gOwogICAgY29uc3QgbG9ncyA9IGdldFVuaWZpZWRMb2cocGVyc2lzdF9mdWxsX3BhdGgpOwogICAgZm9yIChsZXQgbG9nX2VudHJ5ID0gMDsgbG9nX2VudHJ5IDwgbG9ncy5sZW5ndGg7IGxvZ19lbnRyeSsrKSB7CiAgICAgIGlmICghbG9nc1tsb2dfZW50cnldLm1lc3NhZ2UudG9Mb3dlckNhc2UoKS5pbmNsdWRlcygieHByb3RlY3QiKSkgewogICAgICAgIGNvbnRpbnVlOwogICAgICB9CiAgICAgIHhwcm90ZWN0X2VudHJpZXMucHVzaChsb2dzW2xvZ19lbnRyeV0pOwogICAgfQogICAgYnJlYWs7CiAgfQogIHJldHVybiB4cHJvdGVjdF9lbnRyaWVzOwp9Cm1haW4oKTsK";
        let mut output = output_options("runtime_test", "local", "./tmp", true);
        let script = JSScript {
            name: String::from("xprotect_entries"),
            script: test.to_string(),
        };
        let _ = execute_script(&mut output, &script).unwrap();
    }

    #[test]
    #[cfg(target_os = "macos")]
    fn test_parse_trace_file() {
        use super::parse_trace_file;
        use crate::filesystem::files::list_files;
        use macos_unifiedlogs::parser::{
            collect_shared_strings_system, collect_strings_system, collect_timesync_system,
        };

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
