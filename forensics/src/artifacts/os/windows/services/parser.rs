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
use tracing::error;

/// Parse `Services` based on `ServicesOptions`
pub(crate) fn grab_services(options: &ServicesOptions) -> Result<Vec<ServicesData>, ServicesError> {
    let pattern = if let Some(file) = &options.alt_file {
        file.clone()
    } else {
        let drive_result = get_systemdrive();
        let drive = match drive_result {
            Ok(result) => result,
            Err(err) => {
                error!("Could not determine systemdrive: {err:?}");
                return Err(ServicesError::DriveLetter);
            }
        };
        format!("ntfs:{drive}:\\Windows\\System32\\config\\SYSTEM")
    };

    parse_services(&pattern)
}

#[cfg(test)]
#[cfg(target_os = "windows")]
mod tests {
    use crate::artifacts::os::windows::services::parser::grab_services;
    use crate::structs::artifacts::os::windows::ServicesOptions;

    #[test]
    fn test_grab_services() {
        let options = ServicesOptions { alt_file: None };

        let result = grab_services(&options).unwrap();
        assert!(result.len() > 10);
    }
}
