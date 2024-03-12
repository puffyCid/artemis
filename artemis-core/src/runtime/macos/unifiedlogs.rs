use crate::runtime::error::RuntimeError;
use deno_core::{error::AnyError, op2, JsBuffer, ToJsBuffer};
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
use serde::{Deserialize, Serialize};

#[op2]
#[string]
/// Expose Unified Log parsing to `Deno`
pub(crate) fn get_unified_log(
    #[string] path: String,
    #[buffer] meta: JsBuffer,
) -> Result<String, AnyError> {
    let serde_result = serde_json::from_slice(&meta);
    let store_meta: UnifiedLogMeta = match serde_result {
        Ok(results) => results,
        Err(err) => {
            error!("[runtime] Failed deserialize unifiedlog metadata: {err:?}");
            return Err(err.into());
        }
    };

    let logs = parse_trace_file(
        &store_meta.uuid,
        &store_meta.shared,
        &store_meta.timesync,
        &path,
    )?;

    let results = serde_json::to_string(&logs)?;
    Ok(results)
}

#[derive(Serialize, Deserialize)]
struct UnifiedLogMeta {
    uuid: Vec<UUIDText>,
    shared: Vec<SharedCacheStrings>,
    timesync: Vec<TimesyncBoot>,
}

#[op2]
#[serde]
/// Expose setting up UnifiedLog parser to `Deno`
pub(crate) fn setup_unified_log_parser(#[string] path: String) -> Result<ToJsBuffer, AnyError> {
    let (uuid, shared, timesync) = if path.is_empty() {
        (
            collect_strings_system()?,
            collect_shared_strings_system()?,
            collect_timesync_system()?,
        )
    } else {
        (
            collect_strings(&path)?,
            collect_shared_strings(&format!("{path}/dsc"))?,
            collect_timesync(&format!("{path}/timesync"))?,
        )
    };

    let meta = UnifiedLogMeta {
        uuid,
        shared,
        timesync,
    };

    let results = serde_json::to_vec(&meta)?;
    Ok(results.into())
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
        let test = "Ly8gLi4vLi4vYXJ0ZW1pcy1hcGkvc3JjL3V0aWxzL2Vycm9yLnRzCnZhciBFcnJvckJhc2UgPSBjbGFzcyBleHRlbmRzIEVycm9yIHsKICBjb25zdHJ1Y3RvcihuYW1lLCBtZXNzYWdlKSB7CiAgICBzdXBlcigpOwogICAgdGhpcy5uYW1lID0gbmFtZTsKICAgIHRoaXMubWVzc2FnZSA9IG1lc3NhZ2U7CiAgfQp9OwoKLy8gLi4vLi4vYXJ0ZW1pcy1hcGkvc3JjL21hY29zL2Vycm9ycy50cwp2YXIgTWFjb3NFcnJvciA9IGNsYXNzIGV4dGVuZHMgRXJyb3JCYXNlIHsKfTsKCi8vIC4uLy4uL2FydGVtaXMtYXBpL3NyYy9tYWNvcy91bmlmaWVkbG9ncy50cwpmdW5jdGlvbiBnZXRVbmlmaWVkTG9nKHBhdGgsIG1ldGEpIHsKICB0cnkgewogICAgY29uc3QgZGF0YSA9IERlbm8uY29yZS5vcHMuZ2V0X3VuaWZpZWRfbG9nKHBhdGgsIG1ldGEpOwogICAgY29uc3QgbG9nX2RhdGEgPSBKU09OLnBhcnNlKGRhdGEpOwogICAgcmV0dXJuIGxvZ19kYXRhOwogIH0gY2F0Y2ggKGVycikgewogICAgcmV0dXJuIG5ldyBNYWNvc0Vycm9yKCJVTklGSUVETE9HUyIsIGBmYWlsZWQgdG8gcGFyc2UgJHtwYXRofTogJHtlcnJ9YCk7CiAgfQp9CmZ1bmN0aW9uIHNldHVwVW5pZmllZExvZ1BhcnNlcihwYXRoKSB7CiAgaWYgKHBhdGggPT09IHZvaWQgMCkgewogICAgcGF0aCA9ICIiOwogIH0KICB0cnkgewogICAgY29uc3QgZGF0YSA9IERlbm8uY29yZS5vcHMuc2V0dXBfdW5pZmllZF9sb2dfcGFyc2VyKHBhdGgpOwogICAgcmV0dXJuIGRhdGE7CiAgfSBjYXRjaCAoZXJyKSB7CiAgICByZXR1cm4gbmV3IE1hY29zRXJyb3IoIlVOSUZJRURMT0dTIiwgYGZhaWxlZCB0byBwYXJzZSAke3BhdGh9OiAke2Vycn1gKTsKICB9Cn0KCi8vIGh0dHBzOi8vcmF3LmdpdGh1YnVzZXJjb250ZW50LmNvbS9wdWZmeWNpZC9hcnRlbWlzLWFwaS9tYXN0ZXIvc3JjL3V0aWxzL2Vycm9yLnRzCnZhciBFcnJvckJhc2UyID0gY2xhc3MgZXh0ZW5kcyBFcnJvciB7CiAgY29uc3RydWN0b3IobmFtZSwgbWVzc2FnZSkgewogICAgc3VwZXIoKTsKICAgIHRoaXMubmFtZSA9IG5hbWU7CiAgICB0aGlzLm1lc3NhZ2UgPSBtZXNzYWdlOwogIH0KfTsKCi8vIGh0dHBzOi8vcmF3LmdpdGh1YnVzZXJjb250ZW50LmNvbS9wdWZmeWNpZC9hcnRlbWlzLWFwaS9tYXN0ZXIvc3JjL2ZpbGVzeXN0ZW0vZXJyb3JzLnRzCnZhciBGaWxlRXJyb3IgPSBjbGFzcyBleHRlbmRzIEVycm9yQmFzZTIgewp9OwoKLy8gaHR0cHM6Ly9yYXcuZ2l0aHVidXNlcmNvbnRlbnQuY29tL3B1ZmZ5Y2lkL2FydGVtaXMtYXBpL21hc3Rlci9zcmMvZmlsZXN5c3RlbS9kaXJlY3RvcnkudHMKYXN5bmMgZnVuY3Rpb24gcmVhZERpcihwYXRoKSB7CiAgdHJ5IHsKICAgIGNvbnN0IHJlc3VsdCA9IGF3YWl0IGZzLnJlYWREaXIocGF0aCk7CiAgICBjb25zdCBkYXRhID0gSlNPTi5wYXJzZShyZXN1bHQpOwogICAgcmV0dXJuIGRhdGE7CiAgfSBjYXRjaCAoZXJyKSB7CiAgICByZXR1cm4gbmV3IEZpbGVFcnJvcigKICAgICAgIlJFQURfRElSIiwKICAgICAgYGZhaWxlZCB0byByZWFkIGRpcmVjdG9yeSAke3BhdGh9OiAke2Vycn1gCiAgICApOwogIH0KfQoKLy8gbWFpbi50cwphc3luYyBmdW5jdGlvbiBtYWluKCkgewogIGNvbnN0IHBhdGggPSAiL3Zhci9kYi9kaWFnbm9zdGljcy9QZXJzaXN0IjsKICBjb25zdCB4cHJvdGVjdF9lbnRyaWVzID0gW107CiAgY29uc3QgcmVzdWx0ID0gYXdhaXQgcmVhZERpcihwYXRoKTsKICBpZiAocmVzdWx0IGluc3RhbmNlb2YgRXJyb3IpIHsKICAgIHJldHVybjsKICB9CiAgY29uc3QgbWV0YSA9IHNldHVwVW5pZmllZExvZ1BhcnNlcigpOwogIGlmIChtZXRhIGluc3RhbmNlb2YgTWFjb3NFcnJvcikgewogICAgY29uc29sZS5sb2cobWV0YSk7CiAgICByZXR1cm47CiAgfQogIGZvciAoY29uc3QgZW50cnkgb2YgcmVzdWx0KSB7CiAgICBpZiAoIWVudHJ5LmlzX2ZpbGUpIHsKICAgICAgY29udGludWU7CiAgICB9CiAgICBjb25zdCBwZXJzaXN0X2ZpbGUgPSBlbnRyeS5maWxlbmFtZTsKICAgIGNvbnN0IHBlcnNpc3RfZnVsbF9wYXRoID0gYCR7cGF0aH0vJHtwZXJzaXN0X2ZpbGV9YDsKICAgIGNvbnN0IGxvZ3MgPSBnZXRVbmlmaWVkTG9nKHBlcnNpc3RfZnVsbF9wYXRoLCBtZXRhKTsKICAgIGZvciAobGV0IGxvZ19lbnRyeSA9IDA7IGxvZ19lbnRyeSA8IGxvZ3MubGVuZ3RoOyBsb2dfZW50cnkrKykgewogICAgICBpZiAoIWxvZ3NbbG9nX2VudHJ5XS5tZXNzYWdlLnRvTG93ZXJDYXNlKCkuaW5jbHVkZXMoInhwcm90ZWN0IikpIHsKICAgICAgICBjb250aW51ZTsKICAgICAgfQogICAgICB4cHJvdGVjdF9lbnRyaWVzLnB1c2gobG9nc1tsb2dfZW50cnldKTsKICAgIH0KICAgIGJyZWFrOwogIH0KICByZXR1cm4geHByb3RlY3RfZW50cmllczsKfQptYWluKCk7";
        let mut output = output_options("runtime_test", "local", "./tmp", true);
        let script = JSScript {
            name: String::from("xprotect_entries"),
            script: test.to_string(),
        };
        let _ = execute_script(&mut output, &script).unwrap();
    }

    #[test]
    fn test_setup_unified_log_parser() {
        let test = "Ly8gLi4vLi4vYXJ0ZW1pcy1hcGkvc3JjL3V0aWxzL2Vycm9yLnRzCnZhciBFcnJvckJhc2UgPSBjbGFzcyBleHRlbmRzIEVycm9yIHsKICBjb25zdHJ1Y3RvcihuYW1lLCBtZXNzYWdlKSB7CiAgICBzdXBlcigpOwogICAgdGhpcy5uYW1lID0gbmFtZTsKICAgIHRoaXMubWVzc2FnZSA9IG1lc3NhZ2U7CiAgfQp9OwoKLy8gLi4vLi4vYXJ0ZW1pcy1hcGkvc3JjL21hY29zL2Vycm9ycy50cwp2YXIgTWFjb3NFcnJvciA9IGNsYXNzIGV4dGVuZHMgRXJyb3JCYXNlIHsKfTsKCi8vIC4uLy4uL2FydGVtaXMtYXBpL3NyYy9tYWNvcy91bmlmaWVkbG9ncy50cwpmdW5jdGlvbiBnZXRVbmlmaWVkTG9nKHBhdGgsIG1ldGEpIHsKICB0cnkgewogICAgY29uc3QgZGF0YSA9IERlbm8uY29yZS5vcHMuZ2V0X3VuaWZpZWRfbG9nKHBhdGgsIG1ldGEpOwogICAgY29uc3QgbG9nX2RhdGEgPSBKU09OLnBhcnNlKGRhdGEpOwogICAgcmV0dXJuIGxvZ19kYXRhOwogIH0gY2F0Y2ggKGVycikgewogICAgcmV0dXJuIG5ldyBNYWNvc0Vycm9yKCJVTklGSUVETE9HUyIsIGBmYWlsZWQgdG8gcGFyc2UgJHtwYXRofTogJHtlcnJ9YCk7CiAgfQp9CmZ1bmN0aW9uIHNldHVwVW5pZmllZExvZ1BhcnNlcihwYXRoKSB7CiAgaWYgKHBhdGggPT09IHZvaWQgMCkgewogICAgcGF0aCA9ICIiOwogIH0KICB0cnkgewogICAgY29uc3QgZGF0YSA9IERlbm8uY29yZS5vcHMuc2V0dXBfdW5pZmllZF9sb2dfcGFyc2VyKHBhdGgpOwogICAgcmV0dXJuIGRhdGE7CiAgfSBjYXRjaCAoZXJyKSB7CiAgICByZXR1cm4gbmV3IE1hY29zRXJyb3IoIlVOSUZJRURMT0dTIiwgYGZhaWxlZCB0byBwYXJzZSAke3BhdGh9OiAke2Vycn1gKTsKICB9Cn0KCi8vIGh0dHBzOi8vcmF3LmdpdGh1YnVzZXJjb250ZW50LmNvbS9wdWZmeWNpZC9hcnRlbWlzLWFwaS9tYXN0ZXIvc3JjL3V0aWxzL2Vycm9yLnRzCnZhciBFcnJvckJhc2UyID0gY2xhc3MgZXh0ZW5kcyBFcnJvciB7CiAgY29uc3RydWN0b3IobmFtZSwgbWVzc2FnZSkgewogICAgc3VwZXIoKTsKICAgIHRoaXMubmFtZSA9IG5hbWU7CiAgICB0aGlzLm1lc3NhZ2UgPSBtZXNzYWdlOwogIH0KfTsKCi8vIGh0dHBzOi8vcmF3LmdpdGh1YnVzZXJjb250ZW50LmNvbS9wdWZmeWNpZC9hcnRlbWlzLWFwaS9tYXN0ZXIvc3JjL2ZpbGVzeXN0ZW0vZXJyb3JzLnRzCnZhciBGaWxlRXJyb3IgPSBjbGFzcyBleHRlbmRzIEVycm9yQmFzZTIgewp9OwoKLy8gaHR0cHM6Ly9yYXcuZ2l0aHVidXNlcmNvbnRlbnQuY29tL3B1ZmZ5Y2lkL2FydGVtaXMtYXBpL21hc3Rlci9zcmMvZmlsZXN5c3RlbS9kaXJlY3RvcnkudHMKYXN5bmMgZnVuY3Rpb24gcmVhZERpcihwYXRoKSB7CiAgdHJ5IHsKICAgIGNvbnN0IHJlc3VsdCA9IGF3YWl0IGZzLnJlYWREaXIocGF0aCk7CiAgICBjb25zdCBkYXRhID0gSlNPTi5wYXJzZShyZXN1bHQpOwogICAgcmV0dXJuIGRhdGE7CiAgfSBjYXRjaCAoZXJyKSB7CiAgICByZXR1cm4gbmV3IEZpbGVFcnJvcigKICAgICAgIlJFQURfRElSIiwKICAgICAgYGZhaWxlZCB0byByZWFkIGRpcmVjdG9yeSAke3BhdGh9OiAke2Vycn1gCiAgICApOwogIH0KfQoKLy8gbWFpbi50cwphc3luYyBmdW5jdGlvbiBtYWluKCkgewogIGNvbnN0IG1ldGEgPSBzZXR1cFVuaWZpZWRMb2dQYXJzZXIoKTsKICBpZiAobWV0YSBpbnN0YW5jZW9mIE1hY29zRXJyb3IpIHsKICAgIGNvbnNvbGUubG9nKG1ldGEpOwogICAgcmV0dXJuOwogIH0KfQptYWluKCk7";
        let mut output = output_options("runtime_test", "local", "./tmp", true);
        let script = JSScript {
            name: String::from("setup_unified_log"),
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
