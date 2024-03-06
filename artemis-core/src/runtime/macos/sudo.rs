use crate::{artifacts::os::macos::sudo::logs::grab_sudo_logs, runtime::error::RuntimeError};
use deno_core::{error::AnyError, op2};
use log::error;
use macos_unifiedlogs::parser::{
    collect_shared_strings, collect_shared_strings_system, collect_strings, collect_strings_system,
    collect_timesync, collect_timesync_system,
};

#[op2]
#[string]
/// Get `Sudo log` data
pub(crate) fn get_sudologs(#[string] logarchive_path: String) -> Result<String, AnyError> {
    let mut path = String::from("/var/db/diagnostics/Persist");

    let (strings, shared_strings, timesync_data) = if !logarchive_path.is_empty() {
        path = format!("{logarchive_path}/Persist");
        (
            collect_strings(&logarchive_path)?,
            collect_shared_strings(&format!("{logarchive_path}/dsc"))?,
            collect_timesync(&format!("{logarchive_path}/timesync"))?,
        )
    } else {
        (
            collect_strings_system()?,
            collect_shared_strings_system()?,
            collect_timesync_system()?,
        )
    };
    let sudo_results = grab_sudo_logs(&strings, &shared_strings, &timesync_data, &path);
    let sudo = match sudo_results {
        Ok(results) => results,
        Err(err) => {
            error!("[runtime] Failed to get sudo log data: {err:?}");
            return Err(RuntimeError::ExecuteScript.into());
        }
    };
    let results = serde_json::to_string(&sudo)?;
    Ok(results)
}

#[cfg(test)]
#[cfg(target_os = "macos")]
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
    fn test_get_sudologs() {
        let test = "Ly8gaHR0cHM6Ly9yYXcuZ2l0aHVidXNlcmNvbnRlbnQuY29tL3B1ZmZ5Y2lkL2FydGVtaXMtYXBpL21hc3Rlci9zcmMvdW5peC9zdWRvbG9ncy50cwpmdW5jdGlvbiBnZXRNYWNvc1N1ZG9Mb2dzKCkgewogIGNvbnN0IGRhdGEgPSBEZW5vLmNvcmUub3BzLmdldF9zdWRvbG9ncygiIik7CiAgY29uc3QgbG9nX2RhdGEgPSBKU09OLnBhcnNlKGRhdGEpOwogIHJldHVybiBsb2dfZGF0YTsKfQoKLy8gbWFpbi50cwpmdW5jdGlvbiBtYWluKCkgewogIGNvbnN0IGRhdGEgPSBnZXRNYWNvc1N1ZG9Mb2dzKCk7CiAgcmV0dXJuIGRhdGE7Cn0KbWFpbigpOwo=";
        let mut output = output_options("runtime_test", "local", "./tmp", false);
        let script = JSScript {
            name: String::from("sudo_script"),
            script: test.to_string(),
        };
        execute_script(&mut output, &script).unwrap();
    }
}
