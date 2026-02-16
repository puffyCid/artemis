/**
 * MFT (Master File Table) is part of the Windows NTFS filesystem.
 * It keeps track of all files and directories on a system.
 *
 * By just parsing the $MFT file it is possible to create a filelisting without needing to iterate through a live system.
 * Other parsers:
 *   `https://github.com/Velocidex/velociraptor`
 */
use super::{error::MftError, master::parse_mft};
use crate::{
    structs::{artifacts::os::windows::MftOptions, toml::Output},
    utils::{environment::get_systemdrive, time::time_now},
};

/// Try create a filelisting from provided MFT file
pub(crate) fn grab_mft(
    options: &MftOptions,
    output: &mut Output,
    filter: bool,
) -> Result<(), MftError> {
    let start_time = time_now();

    let path = if let Some(file) = &options.alt_file {
        return parse_mft(file, output, filter, start_time);
    } else {
        // Check if alternative drive letter provided
        if let Some(drive) = &options.alt_drive {
            format!("{drive}:\\$MFT")
        } else {
            // Otherwise try to get the SystemDrive
            let drive = get_systemdrive().unwrap_or('C');
            format!("{drive}:\\$MFT")
        }
    };

    parse_mft(&path, output, filter, start_time)
}

#[cfg(test)]
#[cfg(target_os = "windows")]
mod tests {
    use super::grab_mft;
    use crate::structs::{artifacts::os::windows::MftOptions, toml::Output};

    fn output_options(name: &str, output: &str, directory: &str, compress: bool) -> Output {
        Output {
            name: name.to_string(),
            directory: directory.to_string(),
            format: String::from("json"),
            compress,
            endpoint_id: String::from("abcd"),
            output: output.to_string(),
            ..Default::default()
        }
    }

    #[test]
    fn test_grab_mft() {
        let options = MftOptions {
            alt_drive: None,
            alt_file: None,
        };
        let mut output = output_options("mft_temp", "local", "./tmp", false);
        grab_mft(&options, &mut output, false).unwrap();
    }
}
