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
use super::logon::{Logon, Status};
use crate::{filesystem::files::file_reader, structs::artifacts::os::linux::LogonOptions};
use log::{error, warn};

/// Grab all logon data from default paths
pub(crate) fn grab_logons(options: &LogonOptions) -> Vec<Logon> {
    let mut logons = Vec::new();

    if let Some(alt_file) = &options.alt_file {
        grab_logon_file(alt_file, &mut logons);
        return logons;
    }
    let paths = vec![
        String::from("/var/run/utmp"),
        String::from("/var/log/wtmp"),
        String::from("/var/log/btmp"),
    ];

    for path in paths {
        grab_logon_file(&path, &mut logons);
    }

    logons
}

/// Parse logon files at provided path
pub(crate) fn grab_logon_file(path: &str, logons: &mut Vec<Logon>) {
    if !path.ends_with("wtmp") && !path.ends_with("utmp") && !path.ends_with("btmp") {
        warn!("[logons] Provided unsupported logon file {path}");
        return;
    }

    let read_result = file_reader(path);
    let mut reader = match read_result {
        Ok(result) => result,
        Err(err) => {
            error!("[logons] Could not read file {path}: {err:?}");
            return;
        }
    };

    let status = if path.ends_with("btmp") {
        Status::Failed
    } else {
        Status::Success
    };

    let mut logon = Logon::logon_reader(&mut reader, status);

    logons.append(&mut logon);
}

#[cfg(test)]
#[cfg(target_os = "linux")]
mod tests {
    use crate::structs::artifacts::os::linux::LogonOptions;

    use super::{grab_logon_file, grab_logons};

    #[test]
    fn test_grab_logons() {
        let results = grab_logons(&LogonOptions { alt_file: None });
        assert!(!results.is_empty());
    }

    #[test]
    fn test_grab_logon_file() {
        let mut logons = Vec::new();

        grab_logon_file("/var/log/wtmp", &mut logons);
        assert!(!logons.is_empty());
    }

    #[test]
    fn test_grab_logon_file_bad_file() {
        let mut logons = Vec::new();

        grab_logon_file("/var/log/asdfasdfasdf", &mut logons);
        assert!(logons.is_empty());
    }
}
