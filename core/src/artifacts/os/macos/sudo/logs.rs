use crate::{
    artifacts::os::macos::error::MacArtifactError, structs::artifacts::os::macos::MacosSudoOptions,
};
use log::error;
use macos_unifiedlogs::{
    filesystem::{LiveSystemProvider, LogarchiveProvider},
    iterator::UnifiedLogIterator,
    parser::{build_log, collect_timesync},
    timesync::TimesyncBoot,
    traits::FileProvider,
    unified_log::LogData,
};
use std::{collections::HashMap, io::Read, path::Path};

/// Grab sudo log entries in the Unified Log files
pub(crate) fn grab_sudo_logs(options: &MacosSudoOptions) -> Result<Vec<LogData>, MacArtifactError> {
    if let Some(path) = &options.logarchive_path {
        let mut provider = LogarchiveProvider::new(Path::new(path));
        let timesync_data = collect_timesync(&provider).unwrap_or_default();
        parse_trace_file(&timesync_data, &mut provider)
    } else {
        let mut provider = LiveSystemProvider::default();
        let timesync_data = collect_timesync(&provider).unwrap_or_default();
        parse_trace_file(&timesync_data, &mut provider)
    }
}

fn parse_trace_file(
    timesync_data: &HashMap<String, TimesyncBoot>,
    provider: &mut dyn FileProvider,
) -> Result<Vec<LogData>, MacArtifactError> {
    let mut sudo_logs = Vec::new();

    for mut source in provider.tracev3_files() {
        if !source.source_path().contains("Persist") {
            continue;
        }
        let _ = iterate_logs(source.reader(), timesync_data, &mut sudo_logs, provider);
    }

    Ok(sudo_logs)
}

fn iterate_logs(
    mut reader: impl Read,
    timesync_data: &HashMap<String, TimesyncBoot>,
    sudo_logs: &mut Vec<LogData>,
    provider: &mut dyn FileProvider,
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
        let (results, _) = build_log(&chunk, provider, timesync_data, exclude_missing);

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
