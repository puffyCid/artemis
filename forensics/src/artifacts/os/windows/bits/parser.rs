/**
 * Windows Background Intelligent Transfer Service (`BITS`) is a service that allows applications and users to register jobs to upload/download files  
 * It is commonly used by applications to download updates.  In addition, Windows Updates are downloaded through BITS
 * Starting on Windows 10 BITS data is stored in an ESE database  
 * Pre-Win10 it is stored in a proprietary binary format  
 *
 * References:  
 * `https://ss64.com/nt/bitsadmin.html`  
 * `https://en.wikipedia.org/wiki/Background_Intelligent_Transfer_Service`  
 * `https://www.mandiant.com/resources/blog/attacker-use-of-windows-background-intelligent-transfer-service`  
 *
 * Other Parsers:  
 * `https://github.com/fireeye/BitsParser`  
 * `https://github.com/ANSSI-FR/bits_parser` (only pre-win10 BITS files)
 */
use super::{
    background::{legacy_bits, parse_ese_bits, parse_legacy_bits},
    error::BitsError,
};
use crate::{
    filesystem::files::{file_extension, is_file},
    structs::artifacts::os::windows::BitsOptions,
    utils::environment::get_systemdrive,
};
use common::windows::WindowsBits;
use log::error;

/**
 * Grab the `BITS` data from the default path(s) or an alternative path  
 * The associated `BITS` file(s) is locked if the `BITS` service is running so we read the raw file to bypass the lock
 */
pub(crate) fn grab_bits(options: &BitsOptions) -> Result<WindowsBits, BitsError> {
    let path = if let Some(alt) = &options.alt_file {
        alt.clone()
    } else {
        let systemdrive_result = get_systemdrive();
        let systemdrive = match systemdrive_result {
            Ok(result) => result,
            Err(err) => {
                error!("[bits] Could not get systemdrive: {err:?}");
                return Err(BitsError::Systemdrive);
            }
        };
        let bits_path =
            format!("{systemdrive}:\\ProgramData\\Microsoft\\Network\\Downloader\\qmgr.db");
        // If qmbgr.db is not found this may be an older system that uses the older BITS format
        if !is_file(&bits_path) {
            return parse_legacy_bits(&systemdrive, options.carve);
        }
        bits_path
    };
    grab_bits_path(&path, options.carve)
}

/**
 * Grab the BITS data from file path
 */
fn grab_bits_path(path: &str, carve: bool) -> Result<WindowsBits, BitsError> {
    if file_extension(path) == "db" {
        return parse_ese_bits(path, carve);
    }
    legacy_bits(path, carve)
}

#[cfg(test)]
#[cfg(target_os = "windows")]
mod tests {
    use crate::{
        artifacts::os::windows::bits::parser::{grab_bits, grab_bits_path},
        structs::artifacts::os::windows::BitsOptions,
    };
    use std::path::PathBuf;

    #[test]
    fn test_grab_bits() {
        let options = BitsOptions {
            alt_file: None,
            carve: true,
        };
        let _ = grab_bits(&options).unwrap();
    }

    #[test]
    fn test_grab_bits_data() {
        let mut test_location = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
        test_location.push("tests\\test_data\\windows\\bits\\win81\\qmgr0.dat");
        let results = grab_bits_path(&test_location.to_str().unwrap(), false).unwrap();
        assert_eq!(results.bits.len(), 1);
    }
}
