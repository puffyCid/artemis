/**
 * Linux `Journal` files are the logs associated with the Systemd service  
 * Systemd is a popular system service that is common on most Linux distros
 * The logs can contain data related to application activity, sudo commands, and more
 *
 * References:  
 *  `https://systemd.io/JOURNAL_FILE_FORMAT/`
 *  `https://wiki.archlinux.org/title/Systemd/Journal`
 *  `https://github.com/systemd/systemd/blob/main/src/libsystemd/sd-journal/journal-def.h`
 *  `https://www.freedesktop.org/software/systemd/man/systemd.journal-fields.html`
 *
 * Other Parsers:
 *   `journalctl` command on Linux systems
 */
use super::{
    error::JournalError,
    journal::{parse_journal, parse_journal_file},
};
use crate::{
    filesystem::{
        directory::is_directory,
        files::{is_file, list_files, list_files_directories},
    },
    structs::{artifacts::os::linux::JournalOptions, toml::Output},
};
use common::linux::Journal;

/// Parse and grab `Journal` entries at default paths. This can be changed though via /etc/systemd/journald.conf
pub(crate) fn grab_journal(
    output: &mut Output,
    start_time: u64,
    filter: bool,
    options: &JournalOptions,
) -> Result<(), JournalError> {
    let paths = if let Some(alt_path) = &options.alt_path {
        vec![alt_path.clone()]
    } else {
        let persist = "/var/log/journal/";
        let tmp = "/run/systemd/journal";
        let mut logs = list_files_directories(persist).unwrap_or_default();
        let mut tmp_files = list_files_directories(tmp).unwrap_or_default();

        logs.append(&mut tmp_files);
        logs
    };

    for path in paths {
        if is_file(&path) && !path.ends_with("journal") {
            continue;
        }
        if is_file(&path) {
            let _ = parse_journal(&path, output, filter, start_time);
            continue;
        }

        if is_directory(&path) {
            let log_files = list_files(&path).unwrap_or_default();
            for log in log_files {
                if is_file(&log) && !log.ends_with("journal") {
                    continue;
                }
                if is_file(&log) {
                    let _ = parse_journal(&log, output, filter, start_time);
                }
            }
        }
    }

    Ok(())
}

/// Parse a `Journal` file and return its entries
pub(crate) fn grab_journal_file(path: &str) -> Result<Vec<Journal>, JournalError> {
    if !is_file(path) || !path.ends_with("journal") {
        return Err(JournalError::NotJournal);
    }

    parse_journal_file(path)
}

#[cfg(test)]
mod tests {
    use super::grab_journal;
    use crate::{
        artifacts::os::linux::journals::parser::grab_journal_file,
        structs::{artifacts::os::linux::JournalOptions, toml::Output},
    };
    use std::path::PathBuf;

    fn output_options(name: &str, output: &str, directory: &str, compress: bool) -> Output {
        Output {
            name: name.to_string(),
            directory: directory.to_string(),
            format: String::from("jsonl"),
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
    fn test_grab_journal() {
        let mut output = output_options("grab_journal", "local", "./tmp", false);
        grab_journal(&mut output, 0, false, &JournalOptions { alt_path: None }).unwrap();
    }

    #[test]
    fn test_grab_journal_file() {
        let mut test_location = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
        test_location.push("tests/test_data/linux/journal/user-1000@e755452aab34485787b6d73f3035fb8c-000000000000068d-0005ff8ae923c73b.journal");

        let result = grab_journal_file(&test_location.display().to_string()).unwrap();
        assert_eq!(result.len(), 410);
    }

    #[test]
    #[should_panic(expected = "NotJournal")]
    fn test_grab_journal_file_bad() {
        let mut test_location = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
        test_location.push("tests/test_data/windows.toml");

        let result = grab_journal_file(&test_location.display().to_string()).unwrap();
        assert_eq!(result.len(), 410);
    }
}
