use crate::{
    accessor::{access::Accessor, entry::handle::EntryKind},
    artifacts::os::linux::journals::{error::JournalError, journal::parse_journal_file},
    structs::artifacts::os::linux::LinuxSudoOptions,
};
use common::linux::Journal;
use tracing::warn;

/// Grab sudo log entries in the Journal files
pub(crate) fn grab_sudo_logs(options: &LinuxSudoOptions) -> Result<Vec<Journal>, JournalError> {
    let paths = if let Some(alt_dir) = &options.alt_dir {
        vec![alt_dir.clone()]
    } else {
        vec![
            String::from("/var/log/journal/*/*"),
            String::from("/run/log/journal/*/*"),
        ]
    };

    let mut sudo_logs: Vec<Journal> = Vec::new();
    let mut accessor = Accessor::with_defaults();

    for path in paths {
        let journals = match accessor.globfs(&path) {
            Ok(results) => results,
            Err(err) => {
                warn!("Could not glob '{path}': {err:?}");
                continue;
            }
        };
        for journal in journals {
            if journal.meta.kind != EntryKind::File {
                continue;
            }

            // Should always be a file since we check above
            let Some(file_handle) = journal.handle.as_file() else {
                continue;
            };
            let mut reader = match accessor.open_reader_handle(file_handle) {
                Ok(result) => result,
                Err(err) => {
                    warn!("Could not open reader for '{path}': {err:?}");
                    continue;
                }
            };
            let journal_entries = match parse_journal_file(&mut reader, file_handle) {
                Ok(results) => results,
                Err(err) => {
                    warn!("Could not parse journal file for sudo '{path}': {err:?}");
                    continue;
                }
            };
            filter_logs(journal_entries, &mut sudo_logs);
        }
    }

    Ok(sudo_logs)
}

/// Filter Journal files to look for any entry with sudo command
fn filter_logs(journal: Vec<Journal>, sudo_logs: &mut Vec<Journal>) {
    for entries in journal {
        if entries.comm != "sudo" {
            continue;
        }

        sudo_logs.push(entries);
    }
}

#[cfg(test)]
#[cfg(target_os = "linux")]
mod tests {
    use super::{filter_logs, grab_sudo_logs};
    use crate::{
        artifacts::os::linux::journals::parser::grab_journal_file,
        structs::artifacts::os::linux::LinuxSudoOptions,
    };
    use common::linux::Journal;
    use std::path::PathBuf;

    #[test]
    fn test_grab_sudo_logs() {
        let result = grab_sudo_logs(&LinuxSudoOptions { alt_dir: None }).unwrap();
        assert!(!result.is_empty());
    }

    #[test]
    fn test_filter_logs() {
        let mut test_location = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
        test_location.push("tests/test_data/linux/journal/user-1000@e755452aab34485787b6d73f3035fb8c-000000000000068d-0005ff8ae923c73b.journal");

        let result = grab_journal_file(&test_location.display().to_string()).unwrap();
        let mut filter_result: Vec<Journal> = Vec::new();
        filter_logs(result, &mut filter_result);
        assert_eq!(filter_result.len(), 2);
    }
}
