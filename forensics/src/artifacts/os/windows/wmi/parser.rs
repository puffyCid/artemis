/**
 * Windows Management Instrumentation (WMI) is a collections of tools that allow users to manage the system.  
 * This parser parses the WMI Repository database typically found at C:\\Windows\\System32\\wbem\\Repository.
 * Malware can use WMI to achieve persistence on a system
 *
 * References:
 * `https://docs.velociraptor.app/blog/2022/2022-01-12-wmi-eventing`
 * `https://redcanary.com/threat-detection-report/techniques/windows-management-instrumentation`
 * `https://github.com/libyal/dtformats/blob/main/documentation/WMI%20repository%20file%20format.asciidoc`
 *
 * Other Parsers:
 * `https://github.com/Velocidex/velociraptor`
 * `https://github.com/fox-it/dissect.cim`
 */
use super::{error::WmiError, windows_management::get_wmi_persist};
use crate::{
    accessor::{
        access::Accessor,
        entry::handle::{EntryKind, GlobMatch},
    },
    artifacts::os::windows::wmi::windows_management::extract_wmi,
    structs::artifacts::os::windows::WmiPersistOptions,
    utils::environment::get_systemdrive,
};
use common::windows::WmiPersist;
use tracing::{error, warn};

/// Get WMI persist data based on provided options
pub(crate) fn grab_wmi_persist(options: &WmiPersistOptions) -> Result<Vec<WmiPersist>, WmiError> {
    let pattern = if let Some(dir) = &options.alt_dir {
        dir.clone()
    } else {
        let drive = match get_systemdrive() {
            Ok(result) => result,
            Err(err) => {
                error!("Could not get drive letter: {err:?}");
                return Err(WmiError::DriveLetter);
            }
        };
        format!("{drive}:\\Windows\\System32\\wbem\\Repository")
    };
    let mut accessor = Accessor::with_defaults();
    let paths = match accessor.globfs(&pattern) {
        Ok(results) => results,
        Err(err) => {
            error!("Could not glob for WMI {pattern}: {err:?}");
            return Err(WmiError::Glob);
        }
    };

    parse_wmi_persist(paths)
}

/// Parse WMI files at provided path
fn parse_wmi_persist(paths: Vec<GlobMatch>) -> Result<Vec<WmiPersist>, WmiError> {
    let mut accessor = Accessor::with_defaults();
    let mut persist = Vec::new();

    for entry in paths {
        if entry.meta.kind != EntryKind::Directory {
            continue;
        }

        let Some(handle) = entry.handle.as_directory() else {
            continue;
        };

        let wmi_files = match accessor.read_dir_handle(handle) {
            Ok(results) => results,
            Err(err) => {
                warn!(
                    "Could not read directory {}: {err:?}",
                    handle.display_path()
                );
                continue;
            }
        };

        let wmi_data = extract_wmi(wmi_files)?;
        let mut wmi_persist = get_wmi_persist(&wmi_data, &handle.display_path())?;
        persist.append(&mut wmi_persist);
    }

    Ok(persist)
}

#[cfg(test)]
#[cfg(target_os = "windows")]
mod tests {
    use super::{grab_wmi_persist, parse_wmi_persist};
    use crate::{
        accessor::access::Accessor, structs::artifacts::os::windows::WmiPersistOptions,
        utils::environment::get_systemdrive,
    };

    #[test]
    fn test_grab_wmi_persist() {
        let options = WmiPersistOptions { alt_dir: None };

        let _ = grab_wmi_persist(&options).unwrap();
    }

    #[test]
    fn test_parse_wmi_persist() {
        let drive = get_systemdrive().unwrap();
        let mut accessor = Accessor::with_defaults();
        let paths = accessor
            .globfs(&format!("{drive}:\\Windows\\System32\\wbem\\Repository"))
            .unwrap();

        let _ = parse_wmi_persist(paths).unwrap();
    }
}
