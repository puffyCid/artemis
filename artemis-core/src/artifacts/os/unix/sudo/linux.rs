use crate::{
    artifacts::os::linux::journals::{error::JournalError, journal::Journal},
    filesystem::{
        directory::is_directory,
        files::{is_file, list_files, list_files_directories},
    },
};

/// Grab sudo log entries in the Journal files
pub(crate) fn grab_sudo_logs() -> Result<Vec<Journal>, JournalError> {
    let persist = "/var/log/journal/";
    let tmp = "/run/systemd/journal";
    let mut logs = list_files_directories(persist).unwrap_or_default();
    let mut tmp_files = list_files_directories(tmp).unwrap_or_default();

    logs.append(&mut tmp_files);

    let mut sudo_logs: Vec<Journal> = Vec::new();
    for path in logs {
        if is_file(&path) && !path.ends_with("journal") {
            continue;
        }
        if is_file(&path) {
            let journal_entries = Journal::parse_journal_file(&path)?;
            filter_logs(journal_entries, &mut sudo_logs);
            continue;
        }

        if is_directory(&path) {
            let log_files = list_files(&path).unwrap_or_default();
            for log in log_files {
                if is_file(&log) && !log.ends_with("journal") {
                    continue;
                }
                if is_file(&log) {
                    let journal_entries = Journal::parse_journal_file(&log)?;
                    filter_logs(journal_entries, &mut sudo_logs);
                }
            }
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
mod tests {
    use super::{filter_logs, grab_sudo_logs};
    use crate::artifacts::os::linux::journals::{journal::Journal, parser::grab_journal_file};
    use std::path::PathBuf;

    #[test]
    fn test_grab_sudo_logs() {
        let result = grab_sudo_logs().unwrap();
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
