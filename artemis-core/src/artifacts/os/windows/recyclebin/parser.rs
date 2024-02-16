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
    artifacts::os::windows::recyclebin::error::RecycleBinError,
    filesystem::{
        files::{get_filename, read_file},
        metadata::glob_paths,
    },
    structs::artifacts::os::windows::RecycleBinOptions,
    utils::environment::get_systemdrive,
};
use common::windows::RecycleBin;
use log::error;
use std::path::Path;

use super::recycle::parse_recycle_bin;

/// Grab data in the Windows `Recycle Bin` based on options
pub(crate) fn grab_recycle_bin(
    options: &RecycleBinOptions,
) -> Result<Vec<RecycleBin>, RecycleBinError> {
    if let Some(file) = &options.alt_file {
        let result = grab_recycle_bin_path(file)?;
        return Ok(vec![result]);
    }
    let systemdrive_result = get_systemdrive();
    let drive = match systemdrive_result {
        Ok(result) => result,
        Err(err) => {
            error!("[recyclebin] Could not get systemdrive: {err:?}");
            return Err(RecycleBinError::Systemdrive);
        }
    };

    let path = format!("{drive}:\\$RECYCLE.BIN\\*\\$I*");
    let glob_results = glob_paths(&path);
    let glob_paths = match glob_results {
        Ok(result) => result,
        Err(err) => {
            error!("[recyclebin] Could not glob recycle bin path {path}: {err:?}");
            return Err(RecycleBinError::ReadFile);
        }
    };

    let mut recycle = Vec::new();
    for entry in glob_paths {
        let bin_result = grab_recycle_bin_path(&entry.full_path);
        let bin = match bin_result {
            Ok(result) => result,
            Err(_err) => continue,
        };

        recycle.push(bin);
    }

    Ok(recycle)
}

/// Grab data from the provided Windows `Recycle Bin` path
pub(crate) fn grab_recycle_bin_path(path: &str) -> Result<RecycleBin, RecycleBinError> {
    let read_result = read_file(path);
    let data = match read_result {
        Ok(result) => result,
        Err(err) => {
            error!("[recyclebin] Failed to read recycle bing file {path}: {err:?}");
            return Err(RecycleBinError::ReadFile);
        }
    };
    let bin_result = parse_recycle_bin(&data);
    let mut bin = match bin_result {
        Ok((_, result)) => result,
        Err(_err) => {
            error!("[recyclebin] Failed to parse recycle bin file {path}");
            return Err(RecycleBinError::ParseFile);
        }
    };

    bin.recycle_path = path.to_string();
    let dir = Path::new(&path).parent();

    if let Some(path) = dir {
        bin.sid = get_filename(path.to_str().unwrap_or_default());
    }
    Ok(bin)
}

#[cfg(test)]
mod tests {
    use super::{grab_recycle_bin, grab_recycle_bin_path};
    use crate::structs::artifacts::os::windows::RecycleBinOptions;
    use std::path::PathBuf;

    #[test]
    fn test_grab_recycle_bin() {
        let options = RecycleBinOptions { alt_file: None };
        let _ = grab_recycle_bin(&options).unwrap();
    }

    #[test]
    fn test_grab_recycle_bin_path() {
        let mut test_location = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
        test_location.push("tests/test_data/windows/recyclebin/win10/$IWHBX3J");

        let result = grab_recycle_bin_path(&test_location.display().to_string()).unwrap();

        assert_eq!(result.deleted, 1631147228);
        assert_eq!(result.size, 0);
        assert_eq!(result.filename, "ns_osquery_utils_system_systemutils");
        assert_eq!(
            result.full_path,
            "C:\\Users\\bob\\Projects\\osquery\\build\\ns_osquery_utils_system_systemutils"
        );
        assert_eq!(result.directory, "C:\\Users\\bob\\Projects\\osquery\\build");
        assert_eq!(result.sid, "win10");
    }
}
