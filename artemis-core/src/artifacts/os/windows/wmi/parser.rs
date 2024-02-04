use super::{
    error::WmiError,
    wmi::{get_wmi_persist, parse_wmi_repo},
};
use crate::{
    structs::artifacts::os::windows::WmiPersistOptions, utils::environment::get_systemdrive,
};
use common::windows::WmiPersist;
use log::error;

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
