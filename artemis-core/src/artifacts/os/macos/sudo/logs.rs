use crate::{artifacts::os::macos::error::MacArtifactError, filesystem::files::list_files};
use log::error;
use macos_unifiedlogs::{
    dsc::SharedCacheStrings,
    parser::{build_log, parse_log},
    timesync::TimesyncBoot,
    unified_log::LogData,
    uuidtext::UUIDText,
};

/// Grab sudo log entries in the Unified Log files
pub(crate) fn grab_sudo_logs(
    strings: &[UUIDText],
    shared_strings: &[SharedCacheStrings],
    timesync_data: &[TimesyncBoot],
    path: &str,
) -> Result<Vec<LogData>, MacArtifactError> {
    let log_files = list_files(path).unwrap_or_default();
    let mut sudo_logs: Vec<LogData> = Vec::new();

    for file in log_files {
        let logs_result = parse_trace_file(strings, shared_strings, timesync_data, &file);
        if logs_result.is_err() {
            continue;
        }
        let logs = logs_result.unwrap_or_default();
        filter_logs(logs, &mut sudo_logs);
    }

    Ok(sudo_logs)
}

/// Filter Unified Log files to look for any entry with sudo command
fn filter_logs(log: Vec<LogData>, sudo_logs: &mut Vec<LogData>) {
    for entries in log {
        if entries.process != "/usr/bin/sudo" {
            continue;
        }

        sudo_logs.push(entries);
    }
}

/// Parse the provided log (trace) file
fn parse_trace_file(
    string_results: &[UUIDText],
    shared_strings_results: &[SharedCacheStrings],
    timesync_data: &[TimesyncBoot],
    path: &str,
) -> Result<Vec<LogData>, MacArtifactError> {
    let log_result = parse_log(path);
    let log_data = match log_result {
        Ok(results) => results,
        Err(err) => {
            error!("[sudologs] Failed to parse {path} log entry: {err:?}");
            return Err(MacArtifactError::SudoLog);
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
    use super::{filter_logs, grab_sudo_logs, parse_trace_file};
    use crate::filesystem::files::list_files;
    use macos_unifiedlogs::{
        parser::{collect_shared_strings_system, collect_strings_system, collect_timesync_system},
        unified_log::LogData,
    };

    #[test]
    fn test_grab_sudo_logs() {
        let strings = collect_strings_system().unwrap();
        let shared_strings = collect_shared_strings_system().unwrap();
        let timesync_data = collect_timesync_system().unwrap();
        grab_sudo_logs(
            &strings,
            &shared_strings,
            &timesync_data,
            &"/var/db/diagnostics/HighVolume",
        )
        .unwrap();
    }

    #[test]
    fn test_filter_logs() {
        let files = list_files("/var/db/diagnostics/Persist").unwrap();
        let strings = collect_strings_system().unwrap();
        let shared_strings = collect_shared_strings_system().unwrap();
        let timesync_data = collect_timesync_system().unwrap();

        for file in files {
            let mut filter_result: Vec<LogData> = Vec::new();

            let logs = parse_trace_file(&strings, &shared_strings, &timesync_data, &file).unwrap();
            filter_logs(logs, &mut filter_result);
            break;
        }
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
