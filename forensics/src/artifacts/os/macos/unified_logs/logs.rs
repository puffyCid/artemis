use crate::{
    artifacts::{os::macos::error::MacArtifactError, output::output_artifact},
    structs::{artifacts::os::macos::UnifiedLogsOptions, toml::Output},
    utils::time::time_now,
};
use log::error;
use macos_unifiedlogs::{
    filesystem::{LiveSystemProvider, LogarchiveProvider},
    iterator::UnifiedLogIterator,
    parser::{build_log, collect_timesync},
    timesync::TimesyncBoot,
    traits::FileProvider,
    unified_log::UnifiedLogData,
};
use std::{collections::HashMap, io::Read, path::Path};

/// Use the provided strings, shared strings, timesync data to parse the Unified Log data at provided path.
pub(crate) fn grab_logs(
    options: &UnifiedLogsOptions,
    output: &mut Output,
    filter: bool,
) -> Result<(), MacArtifactError> {
    let mut parse_options = ParseOptions {
        start_time: time_now(),
        filter: filter,
        // Persist oversize strings as we parse the Unified Logs
        oversize_strings: UnifiedLogData {
            header: Vec::new(),
            catalog_data: Vec::new(),
            oversize: Vec::new(),
        },
        // Track missing entries. We may be able to parse them once we have all oversize strings
        missing: Vec::new(),
        sources: options.sources.clone(),
    };
    if let Some(path) = &options.logarchive_path {
        let mut provider = LogarchiveProvider::new(Path::new(path));
        // Parse all timesync files
        let timesync_data = collect_timesync(&provider).unwrap_or_default();
        let _ = parse_trace_file(&timesync_data, &mut provider, &mut parse_options, output);
    } else {
        let mut provider = LiveSystemProvider::default();
        let timesync_data = collect_timesync(&provider).unwrap_or_default();
        let _ = parse_trace_file(&timesync_data, &mut provider, &mut parse_options, output);
    };
    Ok(())
}

struct ParseOptions {
    start_time: u64,
    filter: bool,
    oversize_strings: UnifiedLogData,
    missing: Vec<UnifiedLogData>,
    sources: Vec<String>,
}

fn parse_trace_file(
    timesync_data: &HashMap<String, TimesyncBoot>,
    provider: &mut dyn FileProvider,
    options: &mut ParseOptions,
    output: &mut Output,
) -> Result<(), MacArtifactError> {
    for mut source in provider.tracev3_files() {
        // Only go through provided log sources
        if !options.sources.is_empty() {
            for entry in &options.sources.clone() {
                if !source.source_path().contains(entry) {
                    continue;
                }
                let _ = iterate_logs(source.reader(), timesync_data, options, output, provider);
            }
            continue;
        }

        let _ = iterate_logs(source.reader(), timesync_data, options, output, provider);
    }

    let include_missing = false;
    // Now parse any missing entries
    for leftover_data in &mut options.missing {
        // Add all of our previous oversize data to logs for lookups
        leftover_data.oversize = options.oversize_strings.oversize.clone();

        // If we fail to find any missing data its probably due to the logs rolling
        // Ex: tracev3A rolls, tracev3B references Oversize entry in tracev3A will trigger missing data since tracev3A is gone
        let (results, _) = build_log(leftover_data, provider, timesync_data, include_missing);

        let serde_data_result = serde_json::to_value(&results);
        let mut serde_data = match serde_data_result {
            Ok(results) => results,
            Err(err) => {
                error!("[unifiedlogs] Failed to serialize unified logs: {err:?}");
                continue;
            }
        };
        // Done with oversize entries for this log set
        leftover_data.oversize = Vec::new();

        let _ = output_artifact(
            &mut serde_data,
            "unifiedlogs",
            output,
            &options.start_time,
            options.filter,
        );
    }

    Ok(())
}

fn iterate_logs(
    mut reader: impl Read,
    timesync_data: &HashMap<String, TimesyncBoot>,
    options: &mut ParseOptions,
    output: &mut Output,
    provider: &mut dyn FileProvider,
) -> Result<(), MacArtifactError> {
    let mut buf = Vec::new();

    if let Err(err) = reader.read_to_end(&mut buf) {
        error!("Failed to read tracev3 file: {err:?}");
        return Err(MacArtifactError::UnifiedLogs);
    }

    let log_iterator = UnifiedLogIterator {
        data: buf,
        header: Vec::new(),
    };

    // Exclude missing data from returned output. Keep separate until we parse all oversize entries.
    // Then after parsing all logs, go through all missing data and check all parsed oversize entries again
    let exclude_missing = true;

    for mut chunk in log_iterator {
        chunk
            .oversize
            .append(&mut options.oversize_strings.oversize);
        let (results, missing_logs) = build_log(&chunk, provider, timesync_data, exclude_missing);
        options.oversize_strings.oversize = chunk.oversize;
        let serde_data_result = serde_json::to_value(&results);
        let mut serde_data = match serde_data_result {
            Ok(results) => results,
            Err(err) => {
                error!("[unifiedlogs] Failed to serialize unified logs: {err:?}");
                continue;
            }
        };

        let _ = output_artifact(
            &mut serde_data,
            "unifiedlogs",
            output,
            &options.start_time,
            options.filter,
        );
        if missing_logs.catalog_data.is_empty()
            && missing_logs.header.is_empty()
            && missing_logs.oversize.is_empty()
        {
            continue;
        }
        // Track possible missing log data due to oversize strings being in another file
        options.missing.push(missing_logs);
    }
    Ok(())
}

#[cfg(test)]
#[cfg(target_os = "macos")]
mod tests {
    use super::grab_logs;
    use crate::structs::{artifacts::os::macos::UnifiedLogsOptions, toml::Output};

    fn output_options(name: &str, output: &str, directory: &str, compress: bool) -> Output {
        Output {
            name: name.to_string(),
            directory: directory.to_string(),
            format: String::from("csv"),
            compress,
            timeline: false,
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
    fn test_grab_logs() {
        let mut output = output_options("unified_log_test", "local", "./tmp", false);
        let sources = vec![String::from("Special")];

        grab_logs(
            &UnifiedLogsOptions {
                logarchive_path: None,
                sources,
            },
            &mut output,
            false,
        )
        .unwrap();
    }
}
