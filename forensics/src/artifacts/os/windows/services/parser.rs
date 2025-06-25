/**
 * Windows `Services` are a common form of persistence and privilege escalation on Windows systems. Service data is stored in the SYSTEM Registry file.  
 * `Services` run with SYSTEM level privileges.
 *
 * References:  
 * `https://forensafe.com/blogs/windowsservices.html`
 * `https://github.com/Velocidex/velociraptor/blob/master/artifacts/definitions/Windows/System/Services.yaml`
 * `https://winreg-kb.readthedocs.io/en/latest/sources/system-keys/Services-and-drivers.html`
 *
 * Other Parsers:
 * Any tool that can read the Registry
 * `https://github.com/Velocidex/velociraptor`
 */
use super::{error::ServicesError, service::parse_services};
use crate::{
    structs::artifacts::os::windows::ServicesOptions, utils::environment::get_systemdrive,
};
use common::windows::ServicesData;
use log::error;

/// Parse `Services` based on `ServicesOptions`
pub(crate) fn grab_services(options: &ServicesOptions) -> Result<Vec<ServicesData>, ServicesError> {
    if let Some(file) = &options.alt_file {
        return grab_service_file(file);
    }
    default_services()
}

/// Grab and parse SYSTEM file at custom path
fn grab_service_file(path: &str) -> Result<Vec<ServicesData>, ServicesError> {
    parse_services(path)
}

/// Get `Services` entries using default system drive
fn default_services() -> Result<Vec<ServicesData>, ServicesError> {
    let drive_result = get_systemdrive();
    let drive = match drive_result {
        Ok(result) => result,
        Err(err) => {
            error!("[services] Could not determine systemdrive: {err:?}");
            return Err(ServicesError::DriveLetter);
        }
    };
    alt_drive_services(drive)
}

/// Get `Services` entries on a different system drive
fn alt_drive_services(drive: char) -> Result<Vec<ServicesData>, ServicesError> {
    let path = format!("{drive}:\\Windows\\System32\\config\\SYSTEM");
    parse_services(&path)
}

#[cfg(test)]
#[cfg(target_os = "windows")]
mod tests {
    use crate::artifacts::os::windows::services::parser::{
        alt_drive_services, default_services, grab_service_file, grab_services,
    };
    use crate::structs::artifacts::os::windows::ServicesOptions;
    use crate::utils::environment::get_systemdrive;

    #[test]
    fn test_grab_services() {
        let options = ServicesOptions { alt_file: None };

        let result = grab_services(&options).unwrap();
        assert!(result.len() > 10);
    }

    #[test]
    fn test_default_services() {
        let result = default_services().unwrap();
        assert!(result.len() > 10);
    }

    #[test]
    fn test_alt_drive_services() {
        let result = alt_drive_services(&'C').unwrap();
        assert!(result.len() > 10);
    }

    #[test]
    fn test_grab_service_file() {
        let drive = get_systemdrive().unwrap();
        let path = format!("{drive}:\\Windows\\System32\\config\\SYSTEM");

        let _ = grab_service_file(&path).unwrap();
    }
}
