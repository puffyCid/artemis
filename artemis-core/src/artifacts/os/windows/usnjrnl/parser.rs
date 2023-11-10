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
use super::{error::UsnJrnlError, ntfs::parse_usnjrnl_data};
use crate::{structs::artifacts::os::windows::UsnJrnlOptions, utils::environment::get_systemdrive};
use common::windows::UsnJrnlEntry;
use log::error;

/// Parse `UsnJrnl` data and return list of entries
pub(crate) fn grab_usnjrnl(options: &UsnJrnlOptions) -> Result<Vec<UsnJrnlEntry>, UsnJrnlError> {
    if let Some(alt) = options.alt_drive {
        return parse_usnjrnl_data(&alt);
    }
    let systemdrive_result = get_systemdrive();
    let systemdrive = match systemdrive_result {
        Ok(result) => result,
        Err(err) => {
            error!("[usnjrnl] Could not get systemdrive: {err:?}");
            return Err(UsnJrnlError::SystemDrive);
        }
    };

    parse_usnjrnl_data(&systemdrive)
}

#[cfg(test)]
mod tests {
    use super::grab_usnjrnl;
    use crate::structs::artifacts::os::windows::UsnJrnlOptions;

    #[test]
    #[ignore = "Takes a long time"]
    fn test_grab_usnjrnl() {
        let params = UsnJrnlOptions { alt_drive: None };
        let results = grab_usnjrnl(&params).unwrap();
        assert!(results.len() > 10);
    }
}
