/**
 * `UsnJrnl` is a sparse Windows binary file that tracks changes to files and directories.
 * Located at the alternative data stream (ADS) \<drive\>:\$Extend\$UsnJrnl:$J
 * Parsing this data can sometimes show files that have been deleted. However, depending on the file activity
 * on the system entries in the `UsnJrnl` may get overwritten quickly
 *
 * References:
 * `https://github.com/libyal/libfsntfs/blob/main/documentation/New%20Technologies%20File%20System%20(NTFS).asciidoc#usn_change_journal`
 *
 * Other Parsers:
 * `https://f001.backblazeb2.com/file/EricZimmermanTools/MFTECmd.zip`
 * `https://github.com/Velocidex/velociraptor`
 */
use super::{
    error::UsnJrnlError,
    ntfs::{get_usnjrnl_path, parse_usnjrnl_data},
};
use crate::{
    artifacts::os::windows::usnjrnl::ntfs::{get_usnjrnl_alt_path, get_usnjrnl_path_stream},
    structs::{artifacts::os::windows::UsnJrnlOptions, toml::Output},
    utils::{environment::get_systemdrive, time::time_now},
};
use common::windows::UsnJrnlEntry;
use log::error;

/// Parse `UsnJrnl` data and return list of entries
pub(crate) fn grab_usnjrnl(
    options: &UsnJrnlOptions,
    output: &mut Output,
    filter: bool,
) -> Result<(), UsnJrnlError> {
    let start_time = time_now();

    if let Some(alt) = options.alt_drive {
        return parse_usnjrnl_data(alt, &format!("{alt}:\\$MFT"), output, filter, start_time);
    }
    if let Some(path) = &options.alt_path {
        return get_usnjrnl_path_stream(path, &options.alt_mft, output, filter, start_time);
    }
    let systemdrive_result = get_systemdrive();
    let systemdrive = match systemdrive_result {
        Ok(result) => result,
        Err(err) => {
            error!("[usnjrnl] Could not get systemdrive: {err:?}");
            return Err(UsnJrnlError::SystemDrive);
        }
    };

    parse_usnjrnl_data(
        systemdrive,
        &format!("{systemdrive}:\\$MFT"),
        output,
        filter,
        start_time,
    )
}

/// Get `UsnJrnl` data at provided path
pub(crate) fn grab_usnjrnl_path(
    options: &UsnJrnlOptions,
) -> Result<Vec<UsnJrnlEntry>, UsnJrnlError> {
    if let Some(alt) = options.alt_drive {
        return get_usnjrnl_path(alt, &format!("{alt}:\\$MFT"));
    }
    if let Some(path) = &options.alt_path {
        return get_usnjrnl_alt_path(path, &options.alt_mft);
    }
    let systemdrive_result = get_systemdrive();
    let systemdrive = match systemdrive_result {
        Ok(result) => result,
        Err(err) => {
            error!("[usnjrnl] Could not get systemdrive: {err:?}");
            return Err(UsnJrnlError::SystemDrive);
        }
    };
    get_usnjrnl_path(systemdrive, &format!("{systemdrive}:\\$MFT"))
}

#[cfg(test)]
#[cfg(target_os = "windows")]
mod tests {
    use super::{grab_usnjrnl, grab_usnjrnl_path};
    use crate::structs::{artifacts::os::windows::UsnJrnlOptions, toml::Output};
    use std::path::PathBuf;

    fn output_options(name: &str, output: &str, directory: &str, compress: bool) -> Output {
        Output {
            name: name.to_string(),
            directory: directory.to_string(),
            format: String::from("jsonl"),
            compress,
            endpoint_id: String::from("abcd"),
            output: output.to_string(),
            ..Default::default()
        }
    }

    #[test]
    fn test_grab_usnjrnl() {
        let params = UsnJrnlOptions {
            alt_drive: None,
            alt_path: None,
            alt_mft: None,
        };
        let mut output = output_options("usnjrnl_temp", "local", "./tmp", false);

        grab_usnjrnl(&params, &mut output, false).unwrap();
    }

    #[test]
    fn test_grab_usnjrnl_path() {
        let mut test_location = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
        test_location.push("tests\\test_data\\windows\\usnjrnl\\win11\\usnjrnl.raw");
        let params = UsnJrnlOptions {
            alt_drive: None,
            alt_path: Some(test_location.display().to_string()),
            alt_mft: None,
        };
        let results = grab_usnjrnl_path(&params).unwrap();
        assert_eq!(results.len(), 1);
    }
}
