use crate::runtime::error::RuntimeError;
use deno_core::{error::AnyError, op};
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

#[op]
/// Expose Unified Log parsing to `Deno`
fn get_unified_log(path: String) -> Result<String, AnyError> {
    // Not ideal but for now we have to parse the Unified Log metadata each time we want to parse a log file
    // Fortunately the metadata logs are really small and are parsed very quickly
    let strings_results = collect_strings_system();
    let shared_strings_results = collect_shared_strings_system();
    let timesync_data_results = collect_timesync_system();

    let strings = match strings_results {
        Ok(results) => results,
        Err(err) => {
            error!("[runtime] Failed to parse UUIDText files: {err:?}");
            return Err(RuntimeError::ExecuteScript.into());
        }
    };

    let shared_strings = match shared_strings_results {
        Ok(results) => results,
        Err(err) => {
            error!("[runtime] Failed to parse dsc files: {err:?}");
            return Err(RuntimeError::ExecuteScript.into());
        }
    };

    let timesync_data = match timesync_data_results {
        Ok(results) => results,
        Err(err) => {
            error!("[runtime] Failed to parse timesync files: {err:?}");
            return Err(RuntimeError::ExecuteScript.into());
        }
    };

    let logs = parse_trace_file(&strings, &shared_strings, &timesync_data, &path)?;

    let results = serde_json::to_string_pretty(&logs)?;
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
        structs::artifacts::runtime::script::JSScript, utils::artemis_toml::Output,
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
        }
    }

    #[test]
    fn test_get_unified_log() {
        let test = "ZnVuY3Rpb24gZ2V0WHByb3RlY3RFbnRyaWVzKCkgewogICAgY29uc3QgcGF0aCA9ICIvdmFyL2RiL2RpYWdub3N0aWNzL1BlcnNpc3QiOwogICAgY29uc3QgeHByb3RlY3RfbG9ncyA9IFtdOwogICAgZm9yIChjb25zdCBwZXJzaXN0X2VudHJ5IG9mIERlbm8ucmVhZERpclN5bmMocGF0aCkpewogICAgICAgIGlmICghcGVyc2lzdF9lbnRyeS5pc0ZpbGUpIHsKICAgICAgICAgICAgY29udGludWU7CiAgICAgICAgfQogICAgICAgIGNvbnN0IHBlcnNpc3RfZmlsZSA9IHBlcnNpc3RfZW50cnkubmFtZTsKICAgICAgICBjb25zdCBwZXJzaXN0X2Z1bGxfcGF0aCA9IGAke3BhdGh9LyR7cGVyc2lzdF9maWxlfWA7CiAgICAgICAgY29uc3QgZGF0YSA9IERlbm9bRGVuby5pbnRlcm5hbF0uY29yZS5vcHMuZ2V0X3VuaWZpZWRfbG9nKHBlcnNpc3RfZnVsbF9wYXRoKTsKICAgICAgICBjb25zdCBsb2dfZGF0YSA9IEpTT04ucGFyc2UoZGF0YSk7CiAgICAgICAgZm9yIChjb25zdCBlbnRyeSBvZiBsb2dfZGF0YSl7CiAgICAgICAgICAgIGlmICghZW50cnkubWVzc2FnZS50b0xvd2VyQ2FzZSgpLmluY2x1ZGVzKCJ4cHJvdGVjdCIpKSB7CiAgICAgICAgICAgICAgICBjb250aW51ZTsKICAgICAgICAgICAgfQogICAgICAgICAgICB4cHJvdGVjdF9sb2dzLnB1c2goZW50cnkpOwogICAgICAgIH0KICAgICAgICBicmVhazsKICAgIH0KICAgIHJldHVybiB4cHJvdGVjdF9sb2dzOwp9CmZ1bmN0aW9uIG1haW4oKSB7CiAgICByZXR1cm4gZ2V0WHByb3RlY3RFbnRyaWVzKCk7Cn0KbWFpbigpOw==";
        let mut output = output_options("runtime_test", "local", "./tmp", false);
        let script = JSScript {
            name: String::from("xprotect_entries"),
            script: test.to_string(),
        };
        execute_script(&mut output, &script).unwrap();
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
