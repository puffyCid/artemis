use tracing::error;

/**
 * MFT (Master File Table) is part of the Windows NTFS filesystem.
 * It keeps track of all files and directories on a system.
 *
 * By just parsing the $MFT file it is possible to create a filelisting without needing to iterate through a live system.
 * Other parsers:
 *   `https://github.com/Velocidex/velociraptor`
 */
use super::error::MftError;
use crate::{
    accessor::access::Accessor, artifacts::os::windows::mft::master::parse_mft,
    output::manager::OutputManager, structs::artifacts::os::windows::MftOptions,
    utils::environment::get_systemdrive,
};

/// Try create a filelisting from provided MFT file
pub(crate) fn grab_mft(options: &MftOptions, manager: &mut OutputManager) -> Result<(), MftError> {
    let mut drive = String::new();
    let pattern = if let Some(file) = &options.alt_file {
        file.clone()
    } else {
        // Check if alternative drive letter provided
        if let Some(alt_drive) = &options.alt_drive {
            drive = alt_drive.to_string();
            format!("{alt_drive}:\\$MFT")
        } else {
            // Otherwise try to get the SystemDrive
            drive = get_systemdrive().unwrap_or('C').to_string();
            format!("ntfs:{drive}:\\$MFT")
        }
    };

    let mut accessor = Accessor::with_defaults();
    let paths = match accessor.globfs(&pattern) {
        Ok(results) => results,
        Err(err) => {
            error!("Could not glob for MFT {pattern}: {err:?}");
            return Err(MftError::ReadFile);
        }
    };

    parse_mft(paths, manager, options, &drive)
}

#[cfg(test)]
#[cfg(target_os = "windows")]
mod tests {
    use super::grab_mft;
    use crate::structs::toml::{OutputConfig, OutputDestination, OutputFormat};
    use crate::{output::manager::OutputManager, structs::artifacts::os::windows::MftOptions};
    use std::path::PathBuf;

    fn output_options(name: &str, directory: &str, compress: bool) -> OutputManager {
        let config = OutputConfig {
            name: name.to_string(),
            directory: PathBuf::from(directory),
            format: OutputFormat::Jsonl,
            compress,
            endpoint_id: String::from("abcd"),
            destination: OutputDestination::Local,
            ..Default::default()
        };
        OutputManager::new(config).unwrap()
    }

    #[test]
    fn test_grab_mft() {
        let options = MftOptions {
            alt_drive: None,
            alt_file: None,
        };
        let mut output = output_options("mft_temp", "./tmp", false);
        grab_mft(&options, &mut output).unwrap();
    }
}
