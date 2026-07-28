/**
 * Linux `Logon` entries are tracked in three (3) files: utmp, wtmp, and btmp
 *
 * btmp - contains failed logons
 * wtmp - historical logons
 * utmp - active logons
 *
 * References:
 *  `https://github.com/libyal/dtformats/blob/main/documentation/Utmp%20login%20records%20format.asciidoc`
 *
 * Other Parsers:
 *  `https://github.com/Velocidex/velociraptor`
 */
use crate::{
    accessor::{
        access::Accessor,
        entry::handle::{EntryKind, FileHandle},
    },
    artifacts::os::linux::logons::logon::logon_reader,
    structs::artifacts::os::linux::LogonOptions,
};
use common::linux::{Logon, Status};
use tracing::{error, warn};

/// Grab all logon data from default paths
pub(crate) fn grab_logons(options: &LogonOptions) -> Vec<Logon> {
    let mut logons = Vec::new();

    let paths = if let Some(alt_file) = &options.alt_file {
        vec![alt_file.clone()]
    } else {
        vec![
            String::from("/var/run/utmp"),
            String::from("/var/log/wtmp"),
            String::from("/var/log/btmp"),
        ]
    };

    let mut accessor = Accessor::with_defaults();
    for path in paths {
        logon_file_path(&mut accessor, &path, &mut logons);
    }

    logons
}

/// Parse the provided logon file
pub(crate) fn logon_file_path(accessor: &mut Accessor, path: &str, logons: &mut Vec<Logon>) {
    let files = match accessor.globfs(path) {
        Ok(result) => result,
        Err(err) => {
            warn!("Could not glob '{path}': {err:?}");
            return;
        }
    };

    for file in files {
        if file.meta.kind != EntryKind::File {
            continue;
        }
        let Some(file_handle) = file.handle.as_file() else {
            continue;
        };
        grab_logon_file(accessor, file_handle, logons);
    }
}

/// Parse logon files at provided path
pub(crate) fn grab_logon_file(
    accessor: &mut Accessor,
    file_handle: &FileHandle,
    logons: &mut Vec<Logon>,
) {
    if !file_handle.display_path().ends_with("wtmp")
        && !file_handle.display_path().ends_with("utmp")
        && !file_handle.display_path().ends_with("btmp")
    {
        warn!(
            "Provided unsupported logon file {}",
            file_handle.display_path()
        );
        return;
    }

    let mut reader = match accessor.open_reader_handle(file_handle) {
        Ok(result) => result,
        Err(err) => {
            error!(
                "Could not read file {}: {err:?}",
                file_handle.display_path()
            );
            return;
        }
    };

    let status = if file_handle.display_path().ends_with("btmp") {
        Status::Failed
    } else {
        Status::Success
    };

    let mut logon = logon_reader(&mut reader, status, file_handle);

    logons.append(&mut logon);
}

#[cfg(test)]
#[cfg(target_os = "linux")]
mod tests {
    use crate::{
        accessor::{access::Accessor, entry::handle::FileHandle},
        artifacts::os::linux::logons::parser::{grab_logon_file, grab_logons, logon_file_path},
        structs::artifacts::os::linux::LogonOptions,
    };
    use std::path::PathBuf;

    #[test]
    fn test_grab_logons() {
        let results = grab_logons(&LogonOptions { alt_file: None });
        assert!(!results.is_empty());
    }

    #[test]
    fn test_grab_logon_file() {
        let mut logons = Vec::new();
        let mut accessor = Accessor::with_defaults();
        let file_handle = FileHandle::host(PathBuf::from("/var/log/wtmp"));

        grab_logon_file(&mut accessor, &file_handle, &mut logons);
        assert!(!logons.is_empty());
    }

    #[test]
    fn test_logon_file_path() {
        let mut logons = Vec::new();
        let mut accessor = Accessor::with_defaults();
        logon_file_path(&mut accessor, "/var/log/wtmp", &mut logons);
        assert!(!logons.is_empty());
    }

    #[test]
    fn test_grab_logon_file_bad_file() {
        let mut logons = Vec::new();
        let mut accessor = Accessor::with_defaults();
        let file_handle = FileHandle::host(PathBuf::from("/var/log/asdfasdfasdf"));

        grab_logon_file(&mut accessor, &file_handle, &mut logons);
        assert!(logons.is_empty());
    }
}
