use std::path::PathBuf;

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
    accessor::{
        access::Accessor,
        entry::{
            handle::{EntryKind, FileHandle},
            locator::FileLocator,
        },
    },
    output::manager::OutputManager,
    structs::artifacts::os::linux::JournalOptions,
};
use common::linux::Journal;
use tracing::{error, warn};

/// Parse and grab `Journal` entries at default paths. This can be changed though via /etc/systemd/journald.conf
pub(crate) fn grab_journal(
    manager: &mut OutputManager,
    options: &JournalOptions,
) -> Result<(), JournalError> {
    let paths = if let Some(alt_dir) = &options.alt_dir {
        vec![alt_dir.as_str()]
    } else {
        vec!["/var/log/journal/*/*", "/run/log/journal/*/*"]
    };

    let mut accessor = Accessor::with_defaults();
    for path in paths {
        let journals = match accessor.globfs(path) {
            Ok(results) => results,
            Err(err) => {
                warn!("Could not glob journals '{path}': {err:?}");
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

            let _ = parse_journal(&mut accessor, file_handle, manager, options);
        }
    }

    Ok(())
}

/// Parse a `Journal` file and return its entries
pub(crate) fn grab_journal_file(path: &str) -> Result<Vec<Journal>, JournalError> {
    let mut accessor = Accessor::with_defaults();
    let mut reader = match accessor.open_reader(path) {
        Ok(result) => result,
        Err(err) => {
            error!("Could not read journal file '{path}': {err:?}");
            return Err(JournalError::NotJournal);
        }
    };

    let file = FileHandle::new(FileLocator::Host {
        path: PathBuf::from(path),
    });

    parse_journal_file(&mut reader, &file)
}

#[cfg(test)]
mod tests {
    use super::grab_journal;
    use crate::{
        artifacts::os::linux::journals::{error::JournalError, parser::grab_journal_file},
        output::manager::OutputManager,
        structs::{
            artifacts::os::linux::JournalOptions,
            toml::{OutputConfig, OutputDestination, OutputFormat},
        },
    };
    use std::path::PathBuf;

    fn output_options(name: &str, directory: &str, compress: bool) -> OutputManager {
        let config = OutputConfig {
            name: name.to_string(),
            directory: PathBuf::from(directory),
            format: OutputFormat::Jsonl,
            compress,
            endpoint_id: String::from("abcd"),
            destination: OutputDestination::Local,
            ..Default::default()
        };
        OutputManager::new(config).unwrap()
    }

    #[test]
    fn test_grab_journal() {
        let mut manager = output_options("grab_journal", "./tmp", false);
        grab_journal(&mut manager, &JournalOptions { alt_dir: None }).unwrap();
    }

    #[test]
    fn test_grab_journal_file() {
        let mut test_location = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
        test_location.push("tests/test_data/linux/journal/user-1000@e755452aab34485787b6d73f3035fb8c-000000000000068d-0005ff8ae923c73b.journal");

        let result = grab_journal_file(&test_location.display().to_string()).unwrap();
        assert_eq!(result.len(), 410);
    }

    #[test]
    fn test_grab_journal_file_bad() {
        let mut test_location = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
        test_location.push("tests/test_data/windows.toml");

        let err = grab_journal_file(&test_location.display().to_string()).unwrap_err();
        assert!(matches!(err, JournalError::JournalHeader))
    }
}
