/**
 * `Prefetch` data tracks execution of applications on Windows Workstations
 * `Prefetch` is disabled on Windows Servers and may be disabled on systems with SSDs
 *
 * Referencs:
 *  `https://github.com/libyal/libscca/blob/main/documentation/Windows%20Prefetch%20File%20(PF)%20format.asciidoc`
 *
 * Other Parsers:
 *  `https://github.com/Velocidex/velociraptor`
 *  `https://ericzimmerman.github.io/PECmd.zip`
 */
use crate::{
    artifacts::os::windows::prefetch::error::PrefetchError,
    filesystem::files::{file_extension, list_files, read_file},
    structs::artifacts::os::windows::PrefetchOptions,
    utils::environment::get_systemdrive,
};
use common::windows::Prefetch;
use log::error;

use super::pf::parse_prefetch;

/// Parse `Prefetch` based on `PrefetchOptions`
pub(crate) fn grab_prefetch(options: &PrefetchOptions) -> Result<Vec<Prefetch>, PrefetchError> {
    if let Some(path) = &options.alt_dir {
        return custom_prefetch_path(path);
    }

    let drive_result = get_systemdrive();
    let drive = match drive_result {
        Ok(result) => result,
        Err(err) => {
            error!("[prefetch] Could not determine systemdrive: {err:?}");
            return Err(PrefetchError::DriveLetter);
        }
    };
    let path = format!("{drive}:\\Windows\\Prefetch");
    read_directory(&path)
}

/// Read and parse prefetch files at a custom path
pub(crate) fn custom_prefetch_path(path: &str) -> Result<Vec<Prefetch>, PrefetchError> {
    read_directory(path)
}

/// Read all files at provided path
fn read_directory(path: &str) -> Result<Vec<Prefetch>, PrefetchError> {
    let dir_results = list_files(path);
    let read_dir = match dir_results {
        Ok(result) => result,
        Err(err) => {
            error!("[prefetch] Failed to get prefetch files {path}, error: {err:?}");
            return Err(PrefetchError::ReadDirectory);
        }
    };
    let mut prefetch_data: Vec<Prefetch> = Vec::new();

    for pf_file in read_dir {
        // Skip non-prefetch files
        if file_extension(&pf_file) != "pf" {
            continue;
        }

        let prefetch_results = read_prefetch(&pf_file);
        match prefetch_results {
            Ok(result) => prefetch_data.push(result),
            Err(err) => {
                error!("[prefetch] Failed to get prefetch for {pf_file}, error: {err:?}");
            }
        }
    }

    Ok(prefetch_data)
}

/// Read and parse the prefetch file
fn read_prefetch(path: &str) -> Result<Prefetch, PrefetchError> {
    let buffer_results = read_file(path);
    let buffer = match buffer_results {
        Ok(result) => result,
        Err(err) => {
            error!("[prefetch] Failed to read prefetch file {path}, error: {err:?}");
            return Err(PrefetchError::ReadFile);
        }
    };

    parse_prefetch(&buffer, path)
}

#[cfg(test)]
#[cfg(target_os = "windows")]
mod tests {
    use super::{custom_prefetch_path, grab_prefetch};
    use crate::{
        artifacts::os::windows::prefetch::parser::{read_directory, read_prefetch},
        structs::artifacts::os::windows::PrefetchOptions,
    };
    use std::path::PathBuf;

    #[test]
    fn test_grab_prefetch() {
        let options = PrefetchOptions { alt_dir: None };
        let _ = grab_prefetch(&options).unwrap();
    }

    #[test]
    fn test_custom_prefetch_path() {
        let mut test_location = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
        test_location.push("tests/test_data/windows/prefetch/win10");

        let results = custom_prefetch_path(&test_location.display().to_string()).unwrap();
        assert_eq!(results.len(), 272);

        assert_eq!(
            results[124]
                .path
                .contains("SHELLEXPERIENCEHOST.EXE-C83BCA53.pf"),
            true
        );
        assert_eq!(results[124].filename, "SHELLEXPERIENCEHOST.EXE");
        assert_eq!(results[124].hash, "C83BCA53");
        assert_eq!(results[124].last_run_time, "2021-05-10T01:39:55.000Z");
    }

    #[test]
    fn test_read_directory() {
        let mut test_location = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
        test_location.push("tests/test_data/windows/prefetch/win10");

        let results = read_directory(&test_location.display().to_string()).unwrap();
        assert_eq!(results.len(), 272);

        assert_eq!(
            results[124]
                .path
                .contains("SHELLEXPERIENCEHOST.EXE-C83BCA53.pf"),
            true
        );
        assert_eq!(results[124].filename, "SHELLEXPERIENCEHOST.EXE");
        assert_eq!(results[124].hash, "C83BCA53");
        assert_eq!(results[124].last_run_time, "2021-05-10T01:39:55.000Z");
    }

    #[test]
    fn test_custom_prefetch_path_win8() {
        let mut test_location = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
        test_location.push("tests/test_data/windows/prefetch/win81");

        let results = custom_prefetch_path(&test_location.display().to_string()).unwrap();
        assert_eq!(results.len(), 133);

        assert_eq!(
            results[124].path.contains("WINSDKSETUP.EXE-637164D5.pf"),
            true
        );
        assert_eq!(results[124].filename, "WINSDKSETUP.EXE");
        assert_eq!(results[124].hash, "637164D5");
        assert_eq!(results[124].last_run_time, "2019-12-17T03:21:04.000Z");
    }

    #[test]
    fn test_custom_prefetch_path_win7() {
        let mut test_location = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
        test_location.push("tests/test_data/windows/prefetch/win7");

        let results = custom_prefetch_path(&test_location.display().to_string()).unwrap();
        assert_eq!(results.len(), 23);

        assert_eq!(results[1].path.contains("DLLHOST.EXE-5E46FA0D.pf"), true);
        assert_eq!(results[1].filename, "DLLHOST.EXE");
        assert_eq!(results[1].hash, "5E46FA0D");
        assert_eq!(results[1].last_run_time, "2022-10-31T02:39:50.000Z");
    }

    #[test]
    fn test_custom_prefetch_path_win11() {
        let mut test_location = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
        test_location.push("tests/test_data/windows/prefetch/win11");

        let results = custom_prefetch_path(&test_location.display().to_string()).unwrap();
        assert_eq!(results.len(), 257);

        assert_eq!(results[1].path.contains("7ZFM.EXE-44040917.pf"), true);
        assert_eq!(results[1].filename, "7ZFM.EXE");
        assert_eq!(results[1].hash, "44040917");
        assert_eq!(results[1].last_run_time, "2022-10-08T00:38:51.000Z");

        for result in results {
            if result.path.contains("SVCHOST.EXE-576FFE64.pf") {
                assert_eq!(result.path.contains("SVCHOST.EXE-576FFE64.pf"), true);
                assert_eq!(result.filename, "SVCHOST.EXE");
                assert_eq!(result.hash, "576FFE64");
                assert_eq!(result.last_run_time, "2022-10-21T01:52:22.000Z");
                assert_eq!(
                    result.all_run_times,
                    vec![
                        "2022-10-21T01:52:22.000Z",
                        "2022-10-13T05:52:24.000Z",
                        "2022-10-10T00:57:22.000Z"
                    ]
                );
                assert_eq!(
                    result.volume_serial,
                    vec!["8C8FB0D4", "4290933E", "34F4146B"]
                );
                assert_eq!(
                    result.volume_creation,
                    vec![
                        "2020-09-04T06:13:52.000Z",
                        "2020-09-04T06:13:53.000Z",
                        "2022-01-03T23:23:45.000Z"
                    ]
                );
                assert_eq!(
                    result.volume_path,
                    vec![
                        "\\VOLUME{01d682828fb0b754-8c8fb0d4}",
                        "\\VOLUME{01d6828290579d13-4290933e}",
                        "\\VOLUME{01d800f8f40d0d56-34f4146b}"
                    ]
                );
            }
        }
    }

    #[test]
    fn test_read_prefetch() {
        let mut test_location = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
        test_location.push("tests/test_data/windows/prefetch/win81/CMD.EXE-AC113AA8.pf");

        let results = read_prefetch(&test_location.display().to_string()).unwrap();

        assert_eq!(results.path.contains("CMD.EXE-AC113AA8.pf"), true);
        assert_eq!(results.filename, "CMD.EXE");
        assert_eq!(results.hash, "AC113AA8");
        assert_eq!(results.last_run_time, "2020-05-24T01:31:21.000Z");
        assert_eq!(
            results.all_run_times,
            vec![
                "2020-05-24T01:31:21.000Z",
                "2020-05-24T01:29:15.000Z",
                "2020-05-24T01:25:43.000Z",
                "2020-05-24T01:18:10.000Z",
                "2020-05-24T00:24:17.000Z",
                "2020-05-24T00:20:32.000Z",
                "2020-05-24T00:12:48.000Z",
                "2020-05-23T23:50:02.000Z"
            ]
        );
        assert_eq!(results.run_count, 80);
        assert_eq!(results.size, 14130);
        assert_eq!(results.volume_serial, vec!["7ADCE687"]);
        assert_eq!(results.volume_creation, vec!["2019-12-17T04:53:01.000Z"]);
        assert_eq!(results.volume_path, vec!["\\DEVICE\\HARDDISKVOLUME2"]);
        assert_eq!(results.accessed_files_count, 28);
        assert_eq!(results.accessed_directories_count, 5);
        assert_eq!(results.accessed_files.len(), 28);
        assert_eq!(results.accessed_directories.len(), 5);

        assert_eq!(
            results.accessed_files[3],
            "\\DEVICE\\HARDDISKVOLUME2\\WINDOWS\\SYSTEM32\\WOW64CPU.DLL"
        );
        assert_eq!(
            results.accessed_files[12],
            "\\DEVICE\\HARDDISKVOLUME2\\WINDOWS\\SYSWOW64\\CMDEXT.DLL"
        );
        assert_eq!(
            results.accessed_files[27],
            "\\DEVICE\\HARDDISKVOLUME2\\USERS\\BOB\\APPDATA\\LOCAL\\TEMP\\TMP832F744F467240578F4610EC7E1C7547.EXEC.CMD"
        );

        assert_eq!(
            results.accessed_directories[2],
            "\\DEVICE\\HARDDISKVOLUME2\\WINDOWS\\GLOBALIZATION\\SORTING"
        );
        assert_eq!(
            results.accessed_directories[3],
            "\\DEVICE\\HARDDISKVOLUME2\\WINDOWS\\SYSTEM32"
        );
        assert_eq!(
            results.accessed_directories[4],
            "\\DEVICE\\HARDDISKVOLUME2\\WINDOWS\\SYSWOW64"
        );
    }

    #[test]
    #[should_panic(expected = "Version")]
    fn test_read_bad_prefetch() {
        let mut test_location = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
        test_location.push("tests/test_data/windows/prefetch/bad data/malformed.pf");

        let _ = read_prefetch(&test_location.display().to_string()).unwrap();
    }
}
