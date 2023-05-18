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
use super::{error::ShimcacheError, os::shim::ShimcacheEntry, registry::get_shimcache_data};
use crate::{
    structs::artifacts::os::windows::ShimcacheOptions, utils::environment::get_systemdrive,
};
use log::error;

pub(crate) fn grab_shimcache(
    options: &ShimcacheOptions,
) -> Result<Vec<ShimcacheEntry>, ShimcacheError> {
    if let Some(alt_drive) = options.alt_drive {
        return alt_drive_shimcache(&alt_drive);
    }
    default_shimcache()
}

/// Get `Shimcache` entries using the default systemdrive
fn default_shimcache() -> Result<Vec<ShimcacheEntry>, ShimcacheError> {
    let drive_result = get_systemdrive();
    let drive = match drive_result {
        Ok(result) => result,
        Err(err) => {
            error!("[shimcache] Could not determine systemdrive: {err:?}");
            return Err(ShimcacheError::Drive);
        }
    };

    parse_shimcache(&drive)
}

/// Get `Shimcache` entries using an alternative drive letter
fn alt_drive_shimcache(drive: &char) -> Result<Vec<ShimcacheEntry>, ShimcacheError> {
    parse_shimcache(drive)
}

/// Get `Shimcache` entries for all `ControlSets`. Then parse the `Shimcache` data
fn parse_shimcache(drive: &char) -> Result<Vec<ShimcacheEntry>, ShimcacheError> {
    let results = get_shimcache_data(drive)?;
    let mut shimcache_entries = Vec::new();

    for entry in results {
        let mut entries = ShimcacheEntry::parse_shimdata(&entry.shim_data, &entry.key_path)?;
        shimcache_entries.append(&mut entries);
    }
    Ok(shimcache_entries)
}

#[cfg(test)]
mod tests {
    use crate::{
        artifacts::os::windows::shimcache::parser::{
            alt_drive_shimcache, default_shimcache, grab_shimcache, parse_shimcache,
        },
        structs::artifacts::os::windows::ShimcacheOptions,
    };

    #[test]
    fn test_default_shimcache() {
        let results = default_shimcache().unwrap();
        assert!(results.len() > 3);
    }

    #[test]
    fn test_alt_drive_shimcache() {
        let results = alt_drive_shimcache(&'C').unwrap();
        assert!(results.len() > 3);
    }

    #[test]
    fn test_parse_shimcache() {
        let results = parse_shimcache(&'C').unwrap();
        assert!(results.len() > 3);
    }

    #[test]
    fn test_grab_shimcache() {
        let options = ShimcacheOptions { alt_drive: None };

        let results = grab_shimcache(&options).unwrap();
        assert!(results.len() > 5);
    }
}
