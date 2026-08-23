use super::recycle::parse_recycle_bin;
/**
 * Windows `Recycle Bin` files contain metadata about "deleted" files
 * Currently artemis parses the `$I Recycle Bin` files using the std API
 *
 * References:
 * `https://github.com/libyal/dtformats/blob/main/documentation/Windows%20Recycle.Bin%20file%20formats.asciidoc`
 * `https://cybersecurity.att.com/blogs/security-essentials/digital-dumpster-diving-exploring-the-intricacies-of-recycle-bin-forensics`
 *
 * Other parsers:
 * `https://ericzimmerman.github.io/#!index.md`
 * `https://github.com/Velocidex/velociraptor`
 */
use crate::{
    accessor::{access::Accessor, entry::handle::EntryKind},
    artifacts::os::windows::recyclebin::error::RecycleBinError,
    filesystem::{directory::get_parent_directory, files::get_filename},
    structs::artifacts::os::windows::RecycleBinOptions,
    utils::environment::get_systemdrive,
};
use common::windows::RecycleBin;
use tracing::error;

/// Grab data in the Windows `Recycle Bin` based on options
pub(crate) fn grab_recycle_bin(
    options: &RecycleBinOptions,
) -> Result<Vec<RecycleBin>, RecycleBinError> {
    let pattern = if let Some(file) = &options.alt_file {
        file.clone()
    } else {
        let systemdrive_result = get_systemdrive();
        let drive = match systemdrive_result {
            Ok(result) => result,
            Err(err) => {
                error!("Could not get system drive: {err:?}");
                return Err(RecycleBinError::Systemdrive);
            }
        };

        format!("{drive}:\\$RECYCLE.BIN\\*\\$I*")
    };

    recycle_bin_data(&pattern)
}

/// Grab data from the provided Windows `Recycle Bin` path
fn recycle_bin_data(pattern: &str) -> Result<Vec<RecycleBin>, RecycleBinError> {
    let mut accessor = Accessor::with_defaults();
    let paths = match accessor.globfs(pattern) {
        Ok(results) => results,
        Err(err) => {
            error!("Failed to glob {pattern} for RecycleBin files: {err:?}");
            return Err(RecycleBinError::ReadFile);
        }
    };

    let mut values = Vec::new();
    for entry in paths {
        if entry.meta.kind != EntryKind::File {
            continue;
        }

        let Some(handle) = entry.handle.as_file() else {
            continue;
        };

        let bytes = match accessor.read_file_handle(handle) {
            Ok(results) => results,
            Err(err) => {
                error!(
                    "Failed to read recycle bing file {}: {err:?}",
                    handle.display_path()
                );
                continue;
            }
        };

        let mut bin_value = match parse_recycle_bin(&bytes) {
            Ok((_, result)) => result,
            Err(err) => {
                error!(
                    "Failed to parse recycle bin file {}: {err:?},",
                    handle.display_path()
                );
                continue;
            }
        };

        bin_value.evidence = handle.display_path();
        bin_value.sid = get_filename(&get_parent_directory(&bin_value.evidence));

        values.push(bin_value);
    }

    Ok(values)
}

#[cfg(test)]
#[cfg(target_os = "windows")]
mod tests {
    use crate::{
        artifacts::os::windows::recyclebin::parser::{grab_recycle_bin, recycle_bin_data},
        structs::artifacts::os::windows::RecycleBinOptions,
    };
    use std::path::PathBuf;

    #[test]
    fn test_grab_recycle_bin() {
        let options = RecycleBinOptions { alt_file: None };
        let _ = grab_recycle_bin(&options).unwrap();
    }

    #[test]
    fn test_recycle_bin_data() {
        let mut test_location = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
        test_location.push("tests\\test_data\\windows\\recyclebin\\win10\\$IWHBX3J");

        let result = recycle_bin_data(&test_location.display().to_string()).unwrap();

        assert_eq!(result[0].deleted, "2021-09-09T00:27:08.015Z");
        assert_eq!(result[0].size, 0);
        assert_eq!(result[0].filename, "ns_osquery_utils_system_systemutils");
        assert_eq!(
            result[0].full_path,
            "C:\\Users\\bob\\Projects\\osquery\\build\\ns_osquery_utils_system_systemutils"
        );
        assert_eq!(
            result[0].directory,
            "C:\\Users\\bob\\Projects\\osquery\\build"
        );
        assert_eq!(result[0].sid, "win10");
    }
}
