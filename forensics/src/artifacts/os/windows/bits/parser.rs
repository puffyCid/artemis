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
    background::{legacy_bits, parse_ese_bits},
    error::BitsError,
};
use crate::{
    accessor::{access::Accessor, entry::handle::EntryKind},
    structs::artifacts::os::windows::BitsOptions,
    utils::environment::get_systemdrive,
};
use common::windows::BitsInfo;
use tracing::error;

/**
 * Grab the `BITS` data from the default path(s) or an alternative path  
 * The associated `BITS` file(s) is locked if the `BITS` service is running so we read the raw file to bypass the lock
 */
pub(crate) fn grab_bits(options: &BitsOptions) -> Result<Vec<BitsInfo>, BitsError> {
    let pattern = if let Some(file) = &options.alt_file {
        file.clone()
    } else {
        let drive = match get_systemdrive() {
            Ok(result) => result,
            Err(err) => {
                error!("Could not get systemdrive: {err:?}");
                return Err(BitsError::Systemdrive);
            }
        };
        format!("ntfs:{drive}:\\ProgramData\\Microsoft\\Network\\Downloader\\qmgr*")
    };

    let mut accessor = Accessor::with_defaults();
    let paths = match accessor.globfs(&pattern) {
        Ok(results) => results,
        Err(err) => {
            error!("Could not glob BITs files {pattern}: {err:?}");
            return Err(BitsError::ReadFile);
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

        // Modern versions of BITS use ESE db
        if handle.filename() == "qmgr.db" {
            let mut value = parse_ese_bits(handle, options.carve)?;
            values.append(&mut value);
            continue;
        }

        if handle.filename() == "qmgr0.dat" || handle.filename() == "qmgr1.dat" {
            let mut value = legacy_bits(handle, options.carve)?;
            values.append(&mut value);
        }
    }

    Ok(values)
}

#[cfg(test)]
#[cfg(target_os = "windows")]
mod tests {
    use crate::{
        artifacts::os::windows::bits::parser::grab_bits,
        structs::artifacts::os::windows::BitsOptions,
    };

    #[test]
    fn test_grab_bits() {
        let options = BitsOptions {
            alt_file: None,
            carve: true,
        };
        let _ = grab_bits(&options).unwrap();
    }
}
