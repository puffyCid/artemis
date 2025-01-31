use crate::{
    artifacts::os::macos::error::MacArtifactError, structs::artifacts::os::macos::MacosSudoOptions,
};
use log::error;
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

/// Grab sudo log entries in the Unified Log files
pub(crate) fn grab_sudo_logs(options: &MacosSudoOptions) -> Result<Vec<LogData>, MacArtifactError> {
    if let Some(path) = &options.logarchive_path {
        let provider = LogarchiveProvider::new(Path::new(path));
        let string_results = collect_strings(&provider).unwrap_or_default();
        let shared_strings_results = collect_shared_strings(&provider).unwrap_or_default();
        let timesync_data = collect_timesync(&provider).unwrap_or_default();
        parse_trace_file(
            &string_results,
            &shared_strings_results,
            &timesync_data,
            &provider,
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
        )
    }
}

fn parse_trace_file(
    string_results: &[UUIDText],
    shared_strings_results: &[SharedCacheStrings],
    timesync_data: &[TimesyncBoot],
    provider: &dyn FileProvider,
) -> Result<Vec<LogData>, MacArtifactError> {
    let mut sudo_logs = Vec::new();

    for mut source in provider.tracev3_files() {
        if !source.source_path().contains("Persist") {
            continue;
        }
        let _ = iterate_logs(
            source.reader(),
            string_results,
            shared_strings_results,
            timesync_data,
            &mut sudo_logs,
        );
    }

    Ok(sudo_logs)
}

fn iterate_logs(
    mut reader: impl Read,
    strings_data: &[UUIDText],
    shared_strings: &[SharedCacheStrings],
    timesync_data: &[TimesyncBoot],
    sudo_logs: &mut Vec<LogData>,
) -> Result<(), MacArtifactError> {
    let mut buf = Vec::new();

    if let Err(err) = reader.read_to_end(&mut buf) {
        error!("Failed to read tracev3 file: {err:?}");
        return Err(MacArtifactError::SudoLog);
    }

    let log_iterator = UnifiedLogIterator {
        data: buf,
        header: Vec::new(),
    };

    let exclude_missing = false;

    for chunk in log_iterator {
        let (results, _) = build_log(
            &chunk,
            strings_data,
            shared_strings,
            timesync_data,
            exclude_missing,
        );

        filter_logs(results, sudo_logs);
    }
    Ok(())
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

#[cfg(test)]
#[cfg(target_os = "macos")]
mod tests {
    use super::grab_sudo_logs;
    use crate::structs::artifacts::os::macos::MacosSudoOptions;

    #[test]
    fn test_grab_sudo_logs() {
        let options = MacosSudoOptions {
            logarchive_path: None,
        };
        let _ = grab_sudo_logs(&options).unwrap();
    }
}
