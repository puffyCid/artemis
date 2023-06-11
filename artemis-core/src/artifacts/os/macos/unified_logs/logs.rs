use crate::{
    artifacts::os::macos::{artifacts::output_data, error::MacArtifactError},
    filesystem::files::{is_file, list_files},
    utils::artemis_toml::Output,
};
use log::{error, info};
use macos_unifiedlogs::{
    dsc::SharedCacheStrings,
    parser::{build_log, parse_log},
    timesync::TimesyncBoot,
    unified_log::UnifiedLogData,
    uuidtext::UUIDText,
};
use std::path::{Path, PathBuf};

struct UnifiedLog<'a> {
    strings: &'a [UUIDText],
    shared_strings: &'a [SharedCacheStrings],
    timesync_data: &'a [TimesyncBoot],
}

/// Use the provided strings, shared strings, timesync data to parse the Unified Log data at provided path.
pub(crate) fn grab_logs(
    string_results: &[UUIDText],
    shared_strings_results: &[SharedCacheStrings],
    timesync_data: &[TimesyncBoot],
    output: &mut Output,
    start_time: &u64,
    log_sources: &[String],
    filter: &bool,
) -> Result<(), MacArtifactError> {
    // We need to persist the Oversize log entries (they contain large strings that don't fit in normal log entries)
    // Some log entries have Oversize strings located in different tracev3 files.
    let mut oversize_strings = UnifiedLogData {
        header: Vec::new(),
        catalog_data: Vec::new(),
        oversize: Vec::new(),
    };

    let path = "/private/var/db/diagnostics";

    // Exclude missing data from returned output. Keep separate until we parse all oversize entries.
    // Then at end, go through all missing data and check all parsed oversize entries again
    let mut missing_data: Vec<UnifiedLogData> = Vec::new();
    let mut archive_path = PathBuf::from(path);

    let unified = UnifiedLog {
        strings: string_results,
        shared_strings: shared_strings_results,
        timesync_data,
    };

    for source in log_sources {
        archive_path.push(source);
        let options = ParseOptions {
            filter: *filter,
            start_time: *start_time,
            directory_name: source.to_string(),
        };
        parse_trace_files(
            &unified,
            &archive_path,
            output,
            &mut oversize_strings,
            &mut missing_data,
            &options,
        )?;
        archive_path.pop();
    }

    let exclude_missing = false;
    // Since we have all Oversize entries now. Go through any log entries that we were not able to build before
    for mut leftover_data in missing_data {
        // Add all of our previous oversize data to logs for lookups
        leftover_data
            .oversize
            .append(&mut oversize_strings.oversize.clone());

        // If we fail to find any missing data its probably due to the logs rolling
        // Ex: tracev3A rolls, tracev3B references Oversize entry in tracev3A will trigger missing data since tracev3A is gone
        let (results, _) = build_log(
            &leftover_data,
            string_results,
            shared_strings_results,
            timesync_data,
            exclude_missing,
        );

        if results.is_empty() {
            continue;
        }

        let serde_data_result = serde_json::to_value(&results);
        let serde_data = match serde_data_result {
            Ok(results) => results,
            Err(err) => {
                error!("[unifiedlogs] Failed to serialize leftover unified logs: {err:?}");
                continue;
            }
        };
        output_data(&serde_data, "unifiedlogs", output, start_time, filter)?;
    }
    Ok(())
}

struct ParseOptions {
    start_time: u64,
    filter: bool,
    directory_name: String,
}

/// Parse trace files one at a time
fn parse_trace_files(
    unified: &UnifiedLog<'_>,
    archive_path: &Path,
    output: &mut Output,
    oversize_strings: &mut UnifiedLogData,
    missing_data: &mut Vec<UnifiedLogData>,
    options: &ParseOptions,
) -> Result<(), MacArtifactError> {
    let exclude_missing = true;
    let files_results = list_files(&archive_path.display().to_string());
    let files = match files_results {
        Ok(result) => result,
        Err(err) => {
            error!("[unifiedlogs] Failed to get files: {err:?}");
            return Err(MacArtifactError::UnifiedLogs);
        }
    };

    for file in files {
        info!("Parsing: {}", file);

        let log_data_results = if is_file(&file) {
            parse_log(&file)
        } else {
            continue;
        };

        let log_data = match log_data_results {
            Ok(results) => results,
            Err(err) => {
                error!(
                    "[unifiedlogs] Failed to parse {} log entry: {err:?}",
                    options.directory_name
                );
                continue;
            }
        };

        // Get all constructed logs and any log data that failed to get constructed (exclude_missing = true)
        let (results, missing_logs) = build_log(
            &log_data,
            unified.strings,
            unified.shared_strings,
            unified.timesync_data,
            exclude_missing,
        );
        // Track Oversize entries
        oversize_strings
            .oversize
            .append(&mut log_data.oversize.clone());

        // Track missing logs
        missing_data.push(missing_logs);

        let serde_data_result = serde_json::to_value(results);
        let serde_data = match serde_data_result {
            Ok(results) => results,
            Err(err) => {
                error!(
                    "[unifiedlogs] Failed to serialize {} unified logs: {err:?}",
                    options.directory_name
                );
                continue;
            }
        };
        output_data(
            &serde_data,
            "unifiedlogs",
            output,
            &options.start_time,
            &options.filter,
        )?;
    }

    Ok(())
}

#[cfg(test)]
mod tests {

    use std::path::PathBuf;

    use super::{grab_logs, parse_trace_files, ParseOptions, UnifiedLog};
    use crate::utils::{artemis_toml::Output, time};
    use macos_unifiedlogs::{
        parser::{collect_shared_strings_system, collect_strings_system, collect_timesync_system},
        unified_log::UnifiedLogData,
    };

    fn output_options(name: &str, output: &str, directory: &str, compress: bool) -> Output {
        Output {
            name: name.to_string(),
            directory: directory.to_string(),
            format: String::from("json"),
            compress,
            url: Some(String::new()),
            port: Some(0),
            api_key: Some(String::new()),
            username: Some(String::new()),
            password: Some(String::new()),
            generic_keys: Some(Vec::new()),
            endpoint_id: String::from("abcd"),
            collection_id: 0,
            output: output.to_string(),
            filter_name: Some(String::new()),
            filter_script: Some(String::new()),
        }
    }

    #[test]
    fn test_grab_logs() {
        let strings = collect_strings_system().unwrap();
        let shared_strings = collect_shared_strings_system().unwrap();
        let timesync_data = collect_timesync_system().unwrap();
        let mut output = output_options("unified_log_test", "json", "./tmp", false);
        let start_time = time::time_now();
        let sources = vec![String::from("Special")];

        grab_logs(
            &strings,
            &shared_strings,
            &timesync_data,
            &mut output,
            &start_time,
            &sources,
            &false,
        )
        .unwrap();
    }

    #[test]
    fn test_parse_trace_files() {
        let strings = collect_strings_system().unwrap();
        let shared_strings = collect_shared_strings_system().unwrap();
        let timesync_data = collect_timesync_system().unwrap();
        let mut output = output_options("unified_log_test", "json", "./tmp", false);
        let start_time = time::time_now();

        let unified = UnifiedLog {
            strings: &strings,
            shared_strings: &shared_strings,
            timesync_data: &timesync_data,
        };
        let mut oversize_strings = UnifiedLogData {
            header: Vec::new(),
            catalog_data: Vec::new(),
            oversize: Vec::new(),
        };
        let mut missing_data: Vec<UnifiedLogData> = Vec::new();
        let options = ParseOptions {
            filter: false,
            directory_name: String::from("Persist"),
            start_time,
        };
        parse_trace_files(
            &unified,
            &PathBuf::from("/var/db/diagnostics/Special"),
            &mut output,
            &mut oversize_strings,
            &mut missing_data,
            &options,
        )
        .unwrap();
    }
}
