use crate::runtime::{error::RuntimeError, helper::string_arg};
use boa_engine::{Context, JsArgs, JsError, JsResult, JsValue, js_string};
use log::{error, warn};
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

/// Expose Unified Log parsing to `BoaJS`
pub(crate) fn js_unified_log(
    _this: &JsValue,
    args: &[JsValue],
    context: &mut Context,
) -> JsResult<JsValue> {
    let input_path = string_arg(args, &0)?;

    let archive_path = if args.get_or_undefined(1).is_undefined() {
        None
    } else {
        Some(string_arg(args, &1)?)
    };

    // If user provides /var/db re-map to /private/var/db
    let path = if input_path.starts_with("/var") {
        format!("/private{input_path}")
    } else {
        input_path
    };
    let logs_result = if archive_path.is_some() {
        let provider = LogarchiveProvider::new(Path::new(&archive_path.unwrap_or_default()));
        let string_results = collect_strings(&provider).unwrap_or_default();
        let shared_strings_results = collect_shared_strings(&provider).unwrap_or_default();
        let timesync_data = collect_timesync(&provider).unwrap_or_default();
        parse_trace_file(
            &string_results,
            &shared_strings_results,
            &timesync_data,
            &provider,
            &path,
        )
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
        )
    };

    let logs: Vec<LogData> = match logs_result {
        Ok(result) => result,
        Err(err) => {
            let issue = format!("Failed to get unifiedlog: {err:?}");
            return Err(JsError::from_opaque(js_string!(issue).into()));
        }
    };

    let results = serde_json::to_value(&logs).unwrap_or_default();
    let value = JsValue::from_json(&results, context)?;

    Ok(value)
}

/// Parse the provided log (trace) file
fn parse_trace_file(
    string_results: &[UUIDText],
    shared_strings_results: &[SharedCacheStrings],
    timesync_data: &[TimesyncBoot],
    provider: &dyn FileProvider,
    path: &str,
) -> Result<Vec<LogData>, RuntimeError> {
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
) -> Result<Vec<LogData>, RuntimeError> {
    let mut buf = Vec::new();

    let err = reader.read_to_end(&mut buf);
    if err.is_err() {
        error!(
            "[runtime] Could not read unifiedlogs: {:?}",
            err.unwrap_err()
        );
        return Err(RuntimeError::ExecuteScript);
    }

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
        runtime::run::execute_script,
        structs::{artifacts::runtime::script::JSScript, toml::Output},
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
    fn test_js_unified_log() {
        let test = "Ly8gLi4vLi4vYXJ0ZW1pcy1hcGkvc3JjL3V0aWxzL2Vycm9yLnRzCnZhciBFcnJvckJhc2UgPSBjbGFzcyBleHRlbmRzIEVycm9yIHsKICBjb25zdHJ1Y3RvcihuYW1lLCBtZXNzYWdlKSB7CiAgICBzdXBlcigpOwogICAgdGhpcy5uYW1lID0gbmFtZTsKICAgIHRoaXMubWVzc2FnZSA9IG1lc3NhZ2U7CiAgfQp9OwoKLy8gLi4vLi4vYXJ0ZW1pcy1hcGkvc3JjL21hY29zL2Vycm9ycy50cwp2YXIgTWFjb3NFcnJvciA9IGNsYXNzIGV4dGVuZHMgRXJyb3JCYXNlIHsKfTsKCi8vIC4uLy4uL2FydGVtaXMtYXBpL3NyYy9tYWNvcy91bmlmaWVkbG9ncy50cwpmdW5jdGlvbiBnZXRVbmlmaWVkTG9nKHBhdGgpIHsKICB0cnkgewogICAgY29uc3QgZGF0YSA9IGpzX3VuaWZpZWRfbG9nKHBhdGgsIHVuZGVmaW5lZCk7CiAgICByZXR1cm4gZGF0YTsKICB9IGNhdGNoIChlcnIpIHsKICAgIHJldHVybiBuZXcgTWFjb3NFcnJvcigiVU5JRklFRExPR1MiLCBgZmFpbGVkIHRvIHBhcnNlICR7cGF0aH06ICR7ZXJyfWApOwogIH0KfQovLyBodHRwczovL3Jhdy5naXRodWJ1c2VyY29udGVudC5jb20vcHVmZnljaWQvYXJ0ZW1pcy1hcGkvbWFzdGVyL3NyYy91dGlscy9lcnJvci50cwp2YXIgRXJyb3JCYXNlMiA9IGNsYXNzIGV4dGVuZHMgRXJyb3IgewogIGNvbnN0cnVjdG9yKG5hbWUsIG1lc3NhZ2UpIHsKICAgIHN1cGVyKCk7CiAgICB0aGlzLm5hbWUgPSBuYW1lOwogICAgdGhpcy5tZXNzYWdlID0gbWVzc2FnZTsKICB9Cn07CgovLyBodHRwczovL3Jhdy5naXRodWJ1c2VyY29udGVudC5jb20vcHVmZnljaWQvYXJ0ZW1pcy1hcGkvbWFzdGVyL3NyYy9maWxlc3lzdGVtL2Vycm9ycy50cwp2YXIgRmlsZUVycm9yID0gY2xhc3MgZXh0ZW5kcyBFcnJvckJhc2UyIHsKfTsKCi8vIGh0dHBzOi8vcmF3LmdpdGh1YnVzZXJjb250ZW50LmNvbS9wdWZmeWNpZC9hcnRlbWlzLWFwaS9tYXN0ZXIvc3JjL2ZpbGVzeXN0ZW0vZGlyZWN0b3J5LnRzCmFzeW5jIGZ1bmN0aW9uIHJlYWREaXIocGF0aCkgewogIHRyeSB7CiAgICBjb25zdCByZXN1bHQgPSBhd2FpdCBqc19yZWFkX2RpcihwYXRoKTsKICAgIHJldHVybiByZXN1bHQ7CiAgfSBjYXRjaCAoZXJyKSB7CiAgICByZXR1cm4gbmV3IEZpbGVFcnJvcigKICAgICAgIlJFQURfRElSIiwKICAgICAgYGZhaWxlZCB0byByZWFkIGRpcmVjdG9yeSAke3BhdGh9OiAke2Vycn1gLAogICAgKTsKICB9Cn0KCi8vIG1haW4udHMKYXN5bmMgZnVuY3Rpb24gbWFpbigpIHsKICBjb25zdCBwYXRoID0gIi92YXIvZGIvZGlhZ25vc3RpY3MvUGVyc2lzdCI7CiAgY29uc3QgeHByb3RlY3RfZW50cmllcyA9IFtdOwogIGNvbnN0IHJlc3VsdCA9IGF3YWl0IHJlYWREaXIocGF0aCk7CiAgaWYgKHJlc3VsdCBpbnN0YW5jZW9mIEVycm9yKSB7CiAgICByZXR1cm47CiAgfQoKICBmb3IgKGNvbnN0IGVudHJ5IG9mIHJlc3VsdCkgewogICAgaWYgKCFlbnRyeS5pc19maWxlKSB7CiAgICAgIGNvbnRpbnVlOwogICAgfQogICAgY29uc3QgcGVyc2lzdF9maWxlID0gZW50cnkuZmlsZW5hbWU7CiAgICBjb25zdCBwZXJzaXN0X2Z1bGxfcGF0aCA9IGAke3BhdGh9LyR7cGVyc2lzdF9maWxlfWA7CiAgICBjb25zdCBsb2dzID0gZ2V0VW5pZmllZExvZyhwZXJzaXN0X2Z1bGxfcGF0aCk7CiAgICBmb3IgKGxldCBsb2dfZW50cnkgPSAwOyBsb2dfZW50cnkgPCBsb2dzLmxlbmd0aDsgbG9nX2VudHJ5KyspIHsKICAgICAgaWYgKCFsb2dzW2xvZ19lbnRyeV0ubWVzc2FnZS50b0xvd2VyQ2FzZSgpLmluY2x1ZGVzKCJ4cHJvdGVjdCIpKSB7CiAgICAgICAgY29udGludWU7CiAgICAgIH0KICAgICAgeHByb3RlY3RfZW50cmllcy5wdXNoKGxvZ3NbbG9nX2VudHJ5XSk7CiAgICB9CiAgICBicmVhazsKICB9CiAgcmV0dXJuIHhwcm90ZWN0X2VudHJpZXM7Cn0KbWFpbigpOwo=";
        let mut output = output_options("runtime_test", "local", "./tmp", true);
        let script = JSScript {
            name: String::from("xprotect_entries"),
            script: test.to_string(),
        };
        let _ = execute_script(&mut output, &script).unwrap();
    }
}
