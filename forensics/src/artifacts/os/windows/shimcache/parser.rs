/**
 * Windows `Shimcache` (also called: `AppCompatCache`, `Application Compatibility Cache`, `AppCompat`) are Registry entries that track application execution.
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
    accessor::{access::Accessor, entry::handle::EntryKind},
    structs::artifacts::os::windows::ShimcacheOptions,
    utils::environment::get_systemdrive,
};
use common::windows::ShimcacheEntry;
use tracing::{debug, error};

pub(crate) fn grab_shimcache(
    options: &ShimcacheOptions,
) -> Result<Vec<ShimcacheEntry>, ShimcacheError> {
    let pattern = if let Some(file) = &options.alt_file {
        file.clone()
    } else {
        let drive_result = get_systemdrive();
        let drive = match drive_result {
            Ok(result) => result,
            Err(err) => {
                error!("Could not determine system drive: {err:?}");
                return Err(ShimcacheError::Drive);
            }
        };
        format!("ntfs:{drive}:\\Windows\\System32\\config\\SYSTEM")
    };

    parse_shimcache(&pattern)
}

/// Get `Shimcache` entries for all `ControlSets`. Then parse the `Shimcache` data
fn parse_shimcache(pattern: &str) -> Result<Vec<ShimcacheEntry>, ShimcacheError> {
    let mut accessor = Accessor::with_defaults();
    let paths = match accessor.globfs(pattern) {
        Ok(result) => result,
        Err(err) => {
            error!("Could not glob pattern {pattern}: {err:?}");
            return Err(ShimcacheError::RegistryFile);
        }
    };
    let mut shimcache_entries = Vec::new();

    for entry in paths {
        if entry.meta.kind != EntryKind::File {
            continue;
        }

        let Some(handle) = entry.handle.as_file() else {
            continue;
        };

        let results = get_shimcache_data(handle)?;
        for entry in results {
            let mut entries =
                parse_shimdata(&entry.shim_data, &entry.key_path, &handle.display_path())?;
            shimcache_entries.append(&mut entries);
        }
    }

    debug!("Got {} total Shimcache entries", shimcache_entries.len());
    Ok(shimcache_entries)
}

#[cfg(test)]
#[cfg(target_os = "windows")]
mod tests {
    use crate::{
        artifacts::os::windows::shimcache::parser::{grab_shimcache, parse_shimcache},
        structs::artifacts::os::windows::ShimcacheOptions,
    };

    #[test]
    fn test_parse_shimcache() {
        let results = parse_shimcache("ntfs:C:\\Windows\\System32\\config\\SYSTEM").unwrap();
        assert!(results.len() > 3);
    }

    #[test]
    fn test_grab_shimcache() {
        let options = ShimcacheOptions { alt_file: None };

        let results = grab_shimcache(&options).unwrap();
        assert!(results.len() > 5);
    }
}
