use deno_core::{error::AnyError, op2};
use log::warn;
use macos_unifiedlogs::{
    dsc::SharedCacheStrings,
    filesystem::{LiveSystemProvider, LogarchiveProvider},
    iterator::UnifiedLogIterator,
    parser::{build_log, collect_shared_strings, collect_strings, collect_timesync},
    timesync::TimesyncBoot,
    traits::FileProvider,
    unified_log::LogData,
    uuidtext::UUIDText,
};
use std::{io::Read, path::Path};

#[op2]
#[string]
/// Expose Unified Log parsing to `Deno`
pub(crate) fn get_unified_log(
    #[string] input_path: String,
    #[string] archive_path: String,
) -> Result<String, AnyError> {
    // If user provides /var/db re-map to /private/var/db
    let path = if input_path.starts_with("/var") {
        format!("/private{input_path}")
    } else {
        input_path
    };
    let logs = if !archive_path.is_empty() {
        let provider = LogarchiveProvider::new(Path::new(&archive_path));
        let string_results = collect_strings(&provider).unwrap_or_default();
        let shared_strings_results = collect_shared_strings(&provider).unwrap_or_default();
        let timesync_data = collect_timesync(&provider).unwrap_or_default();
        parse_trace_file(
            &string_results,
            &shared_strings_results,
            &timesync_data,
            &provider,
            &path,
        )?
    } else {
        let provider = LiveSystemProvider::default();
        let string_results = collect_strings(&provider).unwrap_or_default();
        let shared_strings_results = collect_shared_strings(&provider).unwrap_or_default();
        let timesync_data = collect_timesync(&provider).unwrap_or_default();
        parse_trace_file(
            &string_results,
            &shared_strings_results,
            &timesync_data,
            &provider,
            &path,
        )?
    };

    let results = serde_json::to_string(&logs)?;
    Ok(results)
}

/// Parse the provided log (trace) file
fn parse_trace_file(
    string_results: &[UUIDText],
    shared_strings_results: &[SharedCacheStrings],
    timesync_data: &[TimesyncBoot],
    provider: &dyn FileProvider,
    path: &str,
) -> Result<Vec<LogData>, AnyError> {
    for mut source in provider.tracev3_files() {
        // Only go through provided log path
        if source.source_path() != path {
            continue;
        }

        return iterate_logs(
            source.reader(),
            string_results,
            shared_strings_results,
            timesync_data,
        );
    }

    warn!("[runtime] Failed to iterate through logs");
    Ok(Vec::new())
}

fn iterate_logs(
    mut reader: impl Read,
    strings_data: &[UUIDText],
    shared_strings: &[SharedCacheStrings],
    timesync_data: &[TimesyncBoot],
) -> Result<Vec<LogData>, AnyError> {
    let mut buf = Vec::new();

    let _ = reader.read_to_end(&mut buf)?;

    let log_iterator = UnifiedLogIterator {
        data: buf,
        header: Vec::new(),
    };

    let exclude_missing = false;
    let mut logs = Vec::new();
    for chunk in log_iterator {
        let (mut results, _) = build_log(
            &chunk,
            strings_data,
            shared_strings,
            timesync_data,
            exclude_missing,
        );

        logs.append(&mut results);
    }
    Ok(logs)
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
}
