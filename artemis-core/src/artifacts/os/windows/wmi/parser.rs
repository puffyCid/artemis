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
use super::{
    error::WmiError,
    windows_management::{get_wmi_persist, parse_wmi_repo},
};
use crate::{
    structs::artifacts::os::windows::WmiPersistOptions, utils::environment::get_systemdrive,
};
use common::windows::WmiPersist;
use log::error;

/// Get WMI persist data based on provided options
pub(crate) fn grab_wmi_persist(options: &WmiPersistOptions) -> Result<Vec<WmiPersist>, WmiError> {
    if let Some(drive) = options.alt_drive {
        let map_paths = format!("{drive}:\\Windows\\System32\\wbem\\Repository\\MAPPING*.MAP");
        let objects_path = format!("{drive}:\\Windows\\System32\\wbem\\Repository\\OBJECTS.DATA");
        let index_path = format!("{drive}:\\Windows\\System32\\wbem\\Repository\\INDEX.BTR");

        return parse_wmi_persist(&map_paths, &objects_path, &index_path);
    } else if let Some(alt_dir) = &options.alt_dir {
        let mut correct_dir = alt_dir.to_string();
        if let Some(verify_dir) = correct_dir.strip_suffix('\\') {
            correct_dir = verify_dir.to_string();
        }
        let map_paths = format!("{correct_dir}\\MAPPING*.MAP");
        let objects_path = format!("{correct_dir}\\OBJECTS.DATA");
        let index_path = format!("{correct_dir}\\INDEX.BTR");

        return parse_wmi_persist(&map_paths, &objects_path, &index_path);
    }

    let default_drive_result = get_systemdrive();
    let default_drive = match default_drive_result {
        Ok(result) => result,
        Err(err) => {
            error!("[wmi] Could not get drive letter: {err:?}");
            return Err(WmiError::DriveLetter);
        }
    };

    let map_paths = format!("{default_drive}:\\Windows\\System32\\wbem\\Repository\\MAPPING*.MAP");
    let objects_path =
        format!("{default_drive}:\\Windows\\System32\\wbem\\Repository\\OBJECTS.DATA");
    let index_path = format!("{default_drive}:\\Windows\\System32\\wbem\\Repository\\INDEX.BTR");

    parse_wmi_persist(&map_paths, &objects_path, &index_path)
}

/// Parse WMI files at provided path
pub(crate) fn parse_wmi_persist(
    map_paths: &str,
    objects_path: &str,
    index_path: &str,
) -> Result<Vec<WmiPersist>, WmiError> {
    let classes = vec![
        "__EventConsumer",
        "__EventFilter",
        "__FilterToConsumerBinding",
    ];
    let wmi_data = parse_wmi_repo(&classes, map_paths, objects_path, index_path)?;

    get_wmi_persist(&wmi_data)
}

#[cfg(test)]
mod tests {
    use super::{grab_wmi_persist, parse_wmi_persist};
    use crate::{
        structs::artifacts::os::windows::WmiPersistOptions, utils::environment::get_systemdrive,
    };

    #[test]
    fn test_grab_wmi_persist() {
        let options = WmiPersistOptions {
            alt_drive: None,
            alt_dir: None,
        };

        let _ = grab_wmi_persist(&options).unwrap();
    }

    #[test]
    #[ignore = "Takes time to run"]
    fn test_parse_wmi_persist() {
        let default_drive = get_systemdrive().unwrap();

        let map_paths =
            format!("{default_drive}:\\Windows\\System32\\wbem\\Repository\\MAPPING*.MAP");
        let objects_path =
            format!("{default_drive}:\\Windows\\System32\\wbem\\Repository\\OBJECTS.DATA");
        let index_path =
            format!("{default_drive}:\\Windows\\System32\\wbem\\Repository\\INDEX.BTR");

        let _ = parse_wmi_persist(&map_paths, &objects_path, &index_path).unwrap();
    }
}
