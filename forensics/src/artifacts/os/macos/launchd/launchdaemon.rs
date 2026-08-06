/**
 * macOS launchd (Daemons and Agents) can be used as persistence
 * They exist system wide and per user
 *
 * References:
 *   `https://www.sentinelone.com/blog/how-malware-persists-on-macos/`
 */
use crate::{
    accessor::{
        access::Accessor,
        entry::handle::{EntryKind, GlobMatch},
    },
    artifacts::os::macos::plist::property_list::parse_plist_file_handle,
    filesystem::metadata::get_timestamps,
    structs::artifacts::os::macos::LaunchdOptions,
};
use common::macos::LaunchdPlist;
use tracing::warn;

/// Grab `LaunchDaemons` and `LaunchAgents`
pub(crate) fn grab_launchd(options: &LaunchdOptions) -> Vec<LaunchdPlist> {
    let paths = if let Some(alt_file) = &options.alt_file {
        vec![alt_file.as_str()]
    } else {
        vec![
            "/Users/*/Library/LaunchDaemons/*",
            "/Library/LaunchDaemons/*",
            "/System/Library/LaunchDaemons/*",
            "/Library/Apple/System/Library/LaunchDaemons/*",
            "/Users/*/Library/LaunchAgents/*",
            "/Library/LaunchAgents/*",
            "/System/Library/LaunchAgents/*",
            "/Library/Apple/System/Library/LaunchAgents/*",
        ]
    };

    let mut accessor = Accessor::with_defaults();
    let mut launchd = Vec::new();
    for path in paths {
        let plist_files = match accessor.globfs(path) {
            Ok(result) => result,
            Err(err) => {
                warn!("Failed to glob '{path}': {err:?}");
                continue;
            }
        };

        launchd.append(&mut extract_launchd_data(plist_files, &mut accessor));
    }

    launchd
}

/// Extract the plist data
fn extract_launchd_data(paths: Vec<GlobMatch>, accessor: &mut Accessor) -> Vec<LaunchdPlist> {
    let mut values = Vec::new();
    for entry in paths {
        if entry.meta.kind != EntryKind::File {
            continue;
        }

        let Some(file_handle) = entry.handle.as_file() else {
            continue;
        };

        let plist_value = match parse_plist_file_handle(accessor, file_handle) {
            Ok(result) => result,
            Err(err) => {
                warn!(
                    "Could not parse file '{}': {err:?}",
                    file_handle.display_path()
                );
                continue;
            }
        };

        let Some(plist_data) = plist_value.into_dictionary() else {
            continue;
        };

        let mut launchd_data = LaunchdPlist {
            launchd_data: plist_data,
            evidence: file_handle.display_path(),
            created: String::from("1601-01-01T00:00:00Z"),
            modified: String::from("1601-01-01T00:00:00Z"),
            accessed: String::from("1601-01-01T00:00:00Z"),
            changed: String::from("1601-01-01T00:00:00Z"),
        };

        let meta_result = get_timestamps(&file_handle.display_path());
        if let Ok(result) = meta_result {
            launchd_data.created = result.created;
            launchd_data.modified = result.modified;
            launchd_data.changed = result.changed;
            launchd_data.accessed = result.accessed;
        }

        values.push(launchd_data);
    }

    values
}

#[cfg(test)]
#[cfg(target_os = "macos")]
mod tests {
    use super::grab_launchd;
    use crate::{
        accessor::{
            access::Accessor,
            entry::handle::{DirHandle, EntryKind, EntryMeta, GlobMatch, ItemHandle},
        },
        artifacts::os::macos::launchd::launchdaemon::extract_launchd_data,
        structs::artifacts::os::macos::LaunchdOptions,
    };
    use std::path::PathBuf;

    #[test]
    fn test_grab_launchd() {
        let results = grab_launchd(&LaunchdOptions { alt_file: None });
        assert!(results.len() > 5);
    }

    #[test]
    fn test_extract_launchd_data() {
        let tests = vec![GlobMatch {
            handle: ItemHandle::Directory(DirHandle::host(PathBuf::new())),
            meta: EntryMeta {
                kind: EntryKind::Directory,
                size: 0,
                display_path: String::new(),
            },
        }];

        let result = extract_launchd_data(tests, &mut Accessor::with_defaults());
        assert!(result.is_empty());
    }
}
