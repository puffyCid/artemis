use crate::{
    artifacts::os::macos::error::MacArtifactError,
    output::{manager::OutputManager, record::serialize_records_to_stream},
    structs::artifacts::os::macos::UnifiedLogsOptions,
};
use macos_unifiedlogs::{
    filesystem::{LiveSystemProvider, LogarchiveProvider},
    iterator::UnifiedLogIterator,
    parser::{build_log, collect_timesync},
    timesync::TimesyncBoot,
    traits::FileProvider,
    unified_log::UnifiedLogData,
};
use std::{collections::HashMap, io::Read, path::Path};
use tracing::error;

/// Use the provided strings, shared strings, timesync data to parse the Unified Log data at provided path.
pub(crate) fn grab_logs(
    options: &UnifiedLogsOptions,
    manager: &mut OutputManager,
) -> Result<(), MacArtifactError> {
    let mut parse_options = ParseOptions {
        // Persist oversize strings as we parse the Unified Logs
        oversize_strings: UnifiedLogData {
            header: Vec::new(),
            catalog_data: Vec::new(),
            oversize: Vec::new(),
            evidence: String::new(),
        },
        // Track missing entries. We may be able to parse them once we have all oversize strings
        missing: Vec::new(),
        sources: options.sources.clone(),
    };
    if let Some(path) = &options.logarchive_path {
        let mut provider = LogarchiveProvider::new(Path::new(path));
        // Parse all timesync files
        let timesync_data = collect_timesync(&provider).unwrap_or_default();
        let _ = parse_trace_file(
            &timesync_data,
            &mut provider,
            &mut parse_options,
            manager,
            options,
        );
    } else {
        let mut provider = LiveSystemProvider::default();
        let timesync_data = collect_timesync(&provider).unwrap_or_default();
        let _ = parse_trace_file(
            &timesync_data,
            &mut provider,
            &mut parse_options,
            manager,
            options,
        );
    };
    Ok(())
}

struct ParseOptions {
    oversize_strings: UnifiedLogData,
    missing: Vec<UnifiedLogData>,
    sources: Vec<String>,
}

fn parse_trace_file(
    timesync_data: &HashMap<String, TimesyncBoot>,
    provider: &mut dyn FileProvider,
    options: &mut ParseOptions,
    manager: &mut OutputManager,
    params: &UnifiedLogsOptions,
) -> Result<(), MacArtifactError> {
    for mut source in provider.tracev3_files() {
        let path = source.source_path().to_string();
        // Only go through provided log sources
        if !options.sources.is_empty() {
            for entry in &options.sources.clone() {
                if !source.source_path().contains(entry) {
                    continue;
                }
                let _ = iterate_logs(
                    source.reader(),
                    timesync_data,
                    options,
                    manager,
                    params,
                    provider,
                    &path,
                );
            }
            continue;
        }

        let _ = iterate_logs(
            source.reader(),
            timesync_data,
            options,
            manager,
            params,
            provider,
            &path,
        );
    }

    let include_missing = false;
    // Now parse any missing entries
    for leftover_data in &mut options.missing {
        // Add all of our previous oversize data to logs for lookups
        leftover_data.oversize = options.oversize_strings.oversize.clone();

        // If we fail to find any missing data its probably due to the logs rolling
        // Ex: tracev3A rolls, tracev3B references Oversize entry in tracev3A will trigger missing data since tracev3A is gone
        let (entries, _) = build_log(leftover_data, provider, timesync_data, include_missing);
        if entries.is_empty() {
            continue;
        }
        let mut records = match serialize_records_to_stream(entries) {
            Ok(results) => results,
            Err(err) => {
                error!("Failed to serialize remaining unifiedlogs: {err:?}");
                continue;
            }
        };

        let artifact_name = "unifiedlogs";
        if let Err(err) = manager.write_artifact(artifact_name, params, &mut records) {
            error!("Failed to output remaining unifiedlogs: {err:?}");
            continue;
        }

        // Done with oversize entries for this log set
        leftover_data.oversize = Vec::new();
    }

    Ok(())
}

fn iterate_logs(
    mut reader: impl Read,
    timesync_data: &HashMap<String, TimesyncBoot>,
    options: &mut ParseOptions,
    manager: &mut OutputManager,
    params: &UnifiedLogsOptions,
    provider: &mut dyn FileProvider,
    evidence: &str,
) -> Result<(), MacArtifactError> {
    let mut buf = Vec::new();

    if let Err(err) = reader.read_to_end(&mut buf) {
        error!("Failed to read tracev3 file: {err:?}");
        return Err(MacArtifactError::UnifiedLogs);
    }

    let log_iterator = UnifiedLogIterator {
        data: buf,
        header: Vec::new(),
        evidence: evidence.to_string(),
    };

    // Exclude missing data from returned output. Keep separate until we parse all oversize entries.
    // Then after parsing all logs, go through all missing data and check all parsed oversize entries again
    let exclude_missing = true;

    for mut chunk in log_iterator {
        chunk
            .oversize
            .append(&mut options.oversize_strings.oversize);
        let (entries, missing_logs) = build_log(&chunk, provider, timesync_data, exclude_missing);
        options.oversize_strings.oversize = chunk.oversize;
        if !missing_logs.catalog_data.is_empty()
            || !missing_logs.header.is_empty()
            || !missing_logs.oversize.is_empty()
        {
            // Track possible missing log data due to oversize strings being in another file
            options.missing.push(missing_logs);
        }

        if entries.is_empty() {
            continue;
        }

        let mut records = match serialize_records_to_stream(entries) {
            Ok(results) => results,
            Err(err) => {
                error!("Failed to serialize unifiedlogs: {err:?}");
                continue;
            }
        };

        let artifact_name = "unifiedlogs";
        if let Err(err) = manager.write_artifact(artifact_name, params, &mut records) {
            error!("Failed to output unifiedlogs: {err:?}");
        }
    }
    Ok(())
}

#[cfg(test)]
#[cfg(target_os = "macos")]
mod tests {
    use super::grab_logs;
    use crate::structs::toml::{OutputConfig, OutputDestination, OutputFormat};
    use crate::{
        output::manager::OutputManager, structs::artifacts::os::macos::UnifiedLogsOptions,
    };
    use std::path::PathBuf;

    fn output_options(name: &str, directory: &str, compress: bool) -> OutputConfig {
        OutputConfig {
            name: name.to_string(),
            directory: PathBuf::from(directory),
            format: OutputFormat::Csv,
            compress,
            endpoint_id: String::from("abcd"),
            destination: OutputDestination::Local,
            ..Default::default()
        }
    }

    #[test]
    fn test_grab_logs() {
        let sources = vec![String::from("Special")];
        let output = output_options("unified_log_test", "./tmp", false);
        let mut manage = OutputManager::new(output).unwrap();

        grab_logs(
            &UnifiedLogsOptions {
                logarchive_path: None,
                sources,
            },
            &mut manage,
        )
        .unwrap();
    }
}
