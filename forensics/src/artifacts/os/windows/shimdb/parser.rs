/**
 * Windows Shimdatabase (`ShimDB`) can be used by Windows applications to provided compatibility between Windows versions.  
 * It does this via `shims` that are inserted into the application that modifies function calls.
 * Malicious custom shims can be created as a form of persistence.
 *
 * References:
 *  `https://www.geoffchappell.com/studies/windows/win32/apphelp/sdb/index.htm`
 *  `https://www.mandiant.com/resources/blog/fin7-shim-databases-persistence`
 *
 * Other Parsers:
 *  `https://ericzimmerman.github.io/SDBExplorer.zip`
 */
use super::{error::ShimdbError, shims::parse_shimdb};
use crate::{
    accessor::{
        access::Accessor,
        config::{AccessMode, AccessorConfig},
        entry::handle::{EntryKind, FileHandle},
    },
    structs::artifacts::os::windows::ShimdbOptions,
    utils::environment::get_systemdrive,
};
use common::windows::ShimData;
use tracing::error;

/// Parse `Shimdb` based on `ShimdbOptions`
pub(crate) fn grab_shimdb(options: &ShimdbOptions) -> Result<Vec<ShimData>, ShimdbError> {
    if let Some(file) = &options.alt_file {
        return custom_shimdb_path(file);
    }
    let drive_result = get_systemdrive();
    let drive = match drive_result {
        Ok(result) => result,
        Err(err) => {
            error!("Could not determine systemdrive: {err:?}");
            return Err(ShimdbError::DriveLetter);
        }
    };

    drive_shimdb(drive)
}

/// SDB files can technically exist anywhere and do not have to end in `.sdb`. Parse any custom paths provided
fn custom_shimdb_path(path: &str) -> Result<Vec<ShimData>, ShimdbError> {
    let mut accessor = Accessor::with_defaults();
    let paths = match accessor.globfs(path) {
        Ok(result) => result,
        Err(err) => {
            error!("Could not glob shimdb files {path}: {err:?}");
            return Err(ShimdbError::ReadFile);
        }
    };

    let mut shim_values = Vec::new();
    for path in paths {
        if path.meta.kind != EntryKind::File {
            continue;
        }

        let Some(handle) = path.handle.as_file() else {
            continue;
        };

        if let Ok(value) = parse_sdb_file("", &mut accessor, Some(handle)) {
            shim_values.push(value);
        }
    }

    Ok(shim_values)
}

/// Parse the default sdb paths on an provided drive letter
fn drive_shimdb(drive: char) -> Result<Vec<ShimData>, ShimdbError> {
    let mut sdb_files = vec![format!("{drive}:\\Windows\\apppatch\\sysmain.sdb")];

    let custom32_bit_path = format!("{drive}:\\Windows\\apppatch\\Custom");
    let mut accessor = Accessor::new(AccessorConfig {
        access_mode: AccessMode::Auto,
        // 10MB
        max_read_size: Some(10485760),
    });

    if let Ok(files) = accessor.read_dir(&custom32_bit_path) {
        for file in files {
            if !file.is_file() {
                continue;
            }

            let Some(handle) = file.handle.as_file() else {
                continue;
            };

            sdb_files.push(handle.display_path());
        }
    }

    let custom64_bit_path = format!("{drive}:\\Windows\\apppatch\\Custom\\Custom64");
    if let Ok(files) = accessor.read_dir(&custom64_bit_path) {
        for file in files {
            if !file.is_file() {
                continue;
            }

            let Some(handle) = file.handle.as_file() else {
                continue;
            };

            sdb_files.push(handle.display_path());
        }
    }

    let mut shimdb_vec: Vec<ShimData> = Vec::new();
    for file in sdb_files {
        if let Ok(result) = parse_sdb_file(&file, &mut accessor, None) {
            shimdb_vec.push(result);
        }
    }
    Ok(shimdb_vec)
}

/// Read and parse a sdb file
fn parse_sdb_file(
    path: &str,
    accessor: &mut Accessor,
    file_handle: Option<&FileHandle>,
) -> Result<ShimData, ShimdbError> {
    let bytes = if let Some(handle) = file_handle {
        match accessor.read_file_handle(handle) {
            Ok(result) => result,
            Err(err) => {
                error!(
                    "Failed to read sdb file handle: {}, error: {err:?}",
                    handle.display_path()
                );
                return Err(ShimdbError::ReadFile);
            }
        }
    } else {
        match accessor.read_file(path) {
            Ok(result) => result,
            Err(err) => {
                error!("Failed to read sdb file at: {path}, error: {err:?}");
                return Err(ShimdbError::ReadFile);
            }
        }
    };

    let shimdb_result = parse_shimdb(&bytes);
    let mut shim_results = match shimdb_result {
        Ok((_, result)) => result,
        Err(err) => {
            error!("Failed to parse sdb file at: {path}, error: {err:?}");
            return Err(ShimdbError::ParseSdb);
        }
    };
    shim_results.evidence = path.to_string();

    Ok(shim_results)
}

#[cfg(test)]
#[cfg(target_os = "windows")]
mod tests {
    use super::{custom_shimdb_path, drive_shimdb, grab_shimdb, parse_sdb_file};
    use crate::{accessor::access::Accessor, structs::artifacts::os::windows::ShimdbOptions};
    use std::path::PathBuf;

    #[test]
    fn test_grab_shimdb() {
        let options = ShimdbOptions { alt_file: None };

        let results = grab_shimdb(&options).unwrap();
        assert!(results.len() >= 1)
    }

    #[test]
    fn test_drive_shimdb() {
        let result = drive_shimdb('C').unwrap();
        assert!(result.len() >= 1)
    }

    #[test]
    fn test_custom_shimdb_path() {
        let mut test_location = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
        test_location.push("tests/test_data/windows/shimdb/win10/sysmain.sdb");

        let mut tests = vec![test_location.display().to_string()];
        test_location.pop();
        test_location.pop();

        test_location.push("AtomicShimx86.sdb");
        tests.push(test_location.display().to_string());
        test_location.pop();

        test_location.push("T1546.011CompatDatabase.sdb");
        tests.push(test_location.display().to_string());

        for path in tests {
            let result = custom_shimdb_path(&path).unwrap();
            assert_eq!(result[0].db_data.name.is_empty(), false)
        }
    }

    #[test]
    fn test_parse_sdb_file() {
        let mut test_location = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
        test_location.push("tests/test_data/windows/shimdb/win10/sysmain.sdb");

        let mut accessor = Accessor::with_defaults();

        let result =
            parse_sdb_file(&test_location.display().to_string(), &mut accessor, None).unwrap();
        assert_eq!(result.db_data.additional_metadata.len(), 0);
        assert_eq!(result.db_data.compile_time, "2016-01-01T00:00:00.000Z");
        assert_eq!(result.db_data.platform, 6);
        assert_eq!(result.db_data.compiler_version, "3.0.0.9");
        assert_eq!(
            result.db_data.name,
            "Microsoft Windows Application Compatibility Fix Database"
        );
        assert_eq!(result.db_data.sdb_version, "3.0");
        assert_eq!(
            result.db_data.database_id,
            "11111111-1111-1111-1111-111111111111"
        );
        assert_eq!(result.db_data.list_data.len(), 13581);

        assert_eq!(
            result.db_data.list_data[0].list_data[0]
                .get("TAG_MODULE")
                .unwrap(),
            "FWCWSP64.dll"
        );
        assert_eq!(
            result.db_data.list_data[13580]
                .data
                .get("TAG_NAME")
                .unwrap(),
            "TARGETPATH:{7C5A40EF-A0FB-4BFC-874A-C0F2E0B9FA8E}\\Microsoft Office\\Office15\\FIRSTRUN.EXE"
        );
        assert_eq!(
            result.db_data.list_data[13580]
                .data
                .get("TAG_APP_NAME")
                .unwrap(),
            "AUMID ShellLink Color Overrides For Desktop Tiles"
        );
    }
}
