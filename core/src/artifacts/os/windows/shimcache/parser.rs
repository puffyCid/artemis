/**
 * Windows `Shimcache` (also called: `AppCompatCache`, `Application Compatability Cache`, `AppCompat`) are Registry entries that track application execution.
 * These entries are only written when the system is shutdown/rebooted
 *
 * References:
 *  `https://www.mandiant.com/resources/blog/caching-out-the-val`
 *  `https://winreg-kb.readthedocs.io/en/latest/sources/system-keys/Application-compatibility-cache.html`
 *
 * Other Parsers:
 *  `https://github.com/Velocidex/velociraptor`
 *  `https://ericzimmerman.github.io/RegistryExplorer.zip`
 */
use super::{error::ShimcacheError, os::shim::parse_shimdata, registry::get_shimcache_data};
use crate::{
    structs::artifacts::os::windows::ShimcacheOptions, utils::environment::get_systemdrive,
};
use common::windows::ShimcacheEntry;
use log::error;

pub(crate) fn grab_shimcache(
    options: &ShimcacheOptions,
) -> Result<Vec<ShimcacheEntry>, ShimcacheError> {
    if let Some(file) = &options.alt_file {
        return parse_shimcache(file);
    }
    let drive_result = get_systemdrive();
    let drive = match drive_result {
        Ok(result) => result,
        Err(err) => {
            error!("[shimcache] Could not determine systemdrive: {err:?}");
            return Err(ShimcacheError::Drive);
        }
    };

    drive_shimcache(&drive)
}

/// Get `Shimcache` entries using an alternative path
fn drive_shimcache(drive: &char) -> Result<Vec<ShimcacheEntry>, ShimcacheError> {
    let path = format!("{drive}:\\Windows\\System32\\config\\SYSTEM");
    parse_shimcache(&path)
}

/// Get `Shimcache` entries for all `ControlSets`. Then parse the `Shimcache` data
fn parse_shimcache(path: &str) -> Result<Vec<ShimcacheEntry>, ShimcacheError> {
    let results = get_shimcache_data(path)?;
    let mut shimcache_entries = Vec::new();

    for entry in results {
        let mut entries = parse_shimdata(&entry.shim_data, &entry.key_path, path)?;
        shimcache_entries.append(&mut entries);
    }
    Ok(shimcache_entries)
}

#[cfg(test)]
#[cfg(target_os = "windows")]
mod tests {
    use crate::{
        artifacts::os::windows::shimcache::parser::{
            drive_shimcache, grab_shimcache, parse_shimcache,
        },
        structs::artifacts::os::windows::ShimcacheOptions,
    };

    #[test]
    fn test_drive_shimcache() {
        let results = drive_shimcache(&'C').unwrap();
        assert!(results.len() > 3);
    }

    #[test]
    fn test_parse_shimcache() {
        let results = parse_shimcache("C:\\Windows\\System32\\config\\SYSTEM").unwrap();
        assert!(results.len() > 3);
    }

    #[test]
    fn test_grab_shimcache() {
        let options = ShimcacheOptions { alt_file: None };

        let results = grab_shimcache(&options).unwrap();
        assert!(results.len() > 5);
    }
}
