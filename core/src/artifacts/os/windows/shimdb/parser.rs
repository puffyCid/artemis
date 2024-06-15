/**
 * Windows Shimdatabase (`ShimDB`) can be used by Windows applications to provided compatability between Windows versions.  
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
    filesystem::files::{file_extension, list_files, read_file_custom},
    structs::artifacts::os::windows::ShimdbOptions,
    utils::environment::get_systemdrive,
};
use common::windows::ShimData;
use log::error;

/// Parse `Shimdb` based on `ShimdbOptions`
pub(crate) fn grab_shimdb(options: &ShimdbOptions) -> Result<Vec<ShimData>, ShimdbError> {
    if let Some(file) = &options.alt_file {
        let result = custom_shimdb_path(file)?;
        return Ok(vec![result]);
    }
    let drive_result = get_systemdrive();
    let drive = match drive_result {
        Ok(result) => result,
        Err(err) => {
            error!("[shimdb] Could not determine systemdrive: {err:?}");
            return Err(ShimdbError::DriveLetter);
        }
    };

    drive_shimdb(&drive)
}

/// SDB files can technically exist anywhere and do not have to end in `.sdb`. Parse any custom paths provided
pub(crate) fn custom_shimdb_path(path: &str) -> Result<ShimData, ShimdbError> {
    parse_sdb_file(path)
}

/// Parse the default sdb paths on an provide drive letter
fn drive_shimdb(drive: &char) -> Result<Vec<ShimData>, ShimdbError> {
    let path = format!("{drive}:\\Windows\\apppatch\\sysmain.sdb");

    let custom32_bit_path = format!("{drive}:\\Windows\\apppatch\\Custom");
    let sdb_files_result = list_files(&custom32_bit_path);
    let mut sdb_files = match sdb_files_result {
        Ok(results) => results,
        Err(err) => {
            error!(
                "[shimdb] Failed to list custom 32 bit sdb files at: {custom32_bit_path}, error: {err:?}",
            );
            return Err(ShimdbError::ReadDirectory);
        }
    };
    let custom64_bit_path = format!("{drive}:\\Windows\\apppatch\\Custom\\Custom64");
    let sdb_files_result = list_files(&custom64_bit_path);
    match sdb_files_result {
        Ok(mut results) => sdb_files.append(&mut results),
        Err(err) => {
            error!(
                "[shimdb] Failed to list custom 64 bit sdb files at: {custom64_bit_path}, error: {err:?}",
            );
            return Err(ShimdbError::ReadDirectory);
        }
    };
    sdb_files.push(path);

    let mut shimdb_vec: Vec<ShimData> = Vec::new();
    for file in sdb_files {
        if file_extension(&file) != "sdb" {
            continue;
        }
        let shim_data_result = parse_sdb_file(&file);
        match shim_data_result {
            Ok(result) => shimdb_vec.push(result),
            Err(_) => continue,
        }
    }
    Ok(shimdb_vec)
}

/// Read and parse a sdb file
fn parse_sdb_file(path: &str) -> Result<ShimData, ShimdbError> {
    let max_size = 10485760; // 10MB
                             // Custom SDB files are very small 1-5KB
                             // The builtin sdb file (sysmain.sdb) is ~4MB
    let buffer_result = read_file_custom(path, &max_size);
    let buffer = match buffer_result {
        Ok(result) => result,
        Err(err) => {
            error!("[shimdb] Failed to read sdb file at: {path}, error: {err:?}");
            return Err(ShimdbError::ReadFile);
        }
    };
    let shimdb_result = parse_shimdb(&buffer);
    let mut shim_results = match shimdb_result {
        Ok((_, result)) => result,
        Err(err) => {
            error!("[shimdb] Failed to parse sdb file at: {path}, error: {err:?}");
            return Err(ShimdbError::ParseSdb);
        }
    };
    shim_results.sdb_path = path.to_string();
    Ok(shim_results)
}

#[cfg(test)]
#[cfg(target_os = "windows")]
mod tests {
    use super::{custom_shimdb_path, drive_shimdb, grab_shimdb, parse_sdb_file};
    use crate::structs::artifacts::os::windows::ShimdbOptions;
    use std::path::PathBuf;

    #[test]
    fn test_grab_shimdb() {
        let options = ShimdbOptions { alt_file: None };

        let results = grab_shimdb(&options).unwrap();
        assert!(results.len() >= 1)
    }

    #[test]
    fn test_drive_shimdb() {
        let result = drive_shimdb(&'C').unwrap();
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
            assert_eq!(result.db_data.name.is_empty(), false)
        }
    }

    #[test]
    fn test_parse_sdb_file() {
        let mut test_location = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
        test_location.push("tests/test_data/windows/shimdb/win10/sysmain.sdb");

        let result = parse_sdb_file(&test_location.display().to_string()).unwrap();
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
        assert_eq!(result.db_data.list_data[13580].data.get("TAG_NAME").unwrap(), "TARGETPATH:{7C5A40EF-A0FB-4BFC-874A-C0F2E0B9FA8E}\\Microsoft Office\\Office15\\FIRSTRUN.EXE");
        assert_eq!(
            result.db_data.list_data[13580]
                .data
                .get("TAG_APP_NAME")
                .unwrap(),
            "AUMID ShellLink Color Overrides For Desktop Tiles"
        );
    }
}
