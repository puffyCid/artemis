use super::{
    class::ClassInfo,
    instance::{ClassValues, InstanceRecord},
    objects::{parse_objects, parse_record},
    windows_management::hash_name,
};
use crate::artifacts::os::windows::wmi::instance::{parse_instance_record, parse_instances};
use log::{error, warn};
use nom::error::ErrorKind;
use std::collections::HashMap;

/// Extract Properties, Classes, and Instances from a Namespace
pub(crate) fn extract_namespace_data(
    namespace_vec: &Vec<Vec<String>>,
    objects: &[u8],
    pages: &[u32],
    classes: &[String],
) -> Vec<ClassValues> {
    let mut classes_info = Vec::new();
    let mut instances_info = Vec::new();
    let mut class_hashes = Vec::new();

    // loop to parse all namespaces of WMI repo and get the classes
    for entries in namespace_vec {
        // First get all the class data associated with namespace
        for class_entry in entries {
            if class_entry.starts_with("CD_") || class_entry.starts_with("IL_") {
                let class_info_result = get_classes(class_entry, objects, pages, classes);
                let classes_result = match class_info_result {
                    Ok((_, result)) => result,
                    Err(_err) => {
                        warn!("[wmi] Failed to get class info for: {class_entry}");
                        continue;
                    }
                };
                if classes_result.is_empty() {
                    continue;
                }
                for class in classes_result.values() {
                    class_hashes.push(hash_name(&class.super_class_name));
                    class_hashes.push(class.class_hash.clone());
                }
                classes_info.push(classes_result);
            }
        }
    }
    class_hashes.sort();
    class_hashes.dedup();

    // loop to parse all namespaces of WMI repo and get the instances
    for entries in namespace_vec {
        // First get all the class data associated with namespace
        for class_entry in entries {
            if class_entry.starts_with("CD_") || class_entry.starts_with("IL_") {
                let instances_result = get_instances(class_entry, objects, pages, &class_hashes);
                let mut instance = match instances_result {
                    Ok((_, result)) => result,
                    Err(_err) => {
                        warn!("[wmi] Failed to get instance info for: {class_entry}");
                        continue;
                    }
                };
                if instance.is_empty() {
                    continue;
                }

                instances_info.append(&mut instance);
            }
        }
    }
    let lookup_parents = classes_info.clone();

    let values_result = parse_instances(&mut classes_info, &instances_info, &lookup_parents);
    match values_result {
        Ok((_, result)) => result,
        Err(_err) => {
            error!("[wmi] Failed to get WMI data for properties and instances");
            Vec::new()
        }
    }
}

/// Get classes associated with a namespace
pub(crate) fn get_classes<'a>(
    class_hash: &str,
    objects_data: &'a [u8],
    pages: &[u32],
    filter_classes: &[String],
) -> nom::IResult<&'a [u8], HashMap<String, ClassInfo>> {
    let hash_result = extract_hash_info(class_hash);

    let (_hash, record_id) = if let Some(result) = hash_result {
        result
    } else {
        error!("[wmi] Could not split WMI index hash for classes");
        return Err(nom::Err::Failure(nom::error::Error::new(
            &[],
            ErrorKind::Fail,
        )));
    };
    let (_, object_info) = parse_objects(objects_data, pages)?;

    let mut classes = HashMap::new();
    for object in &object_info {
        if object.record_id != record_id {
            continue;
        }
        let class_result = parse_record(&object.object_data);
        let class = match class_result {
            Ok((_, result)) => result,
            Err(_err) => {
                continue;
            }
        };
        if !filter_classes.contains(&class.class_hash)
            && !filter_classes.contains(&hash_name(&class.super_class_name))
        {
            continue;
        }
        classes.insert(class.class_hash.clone(), class);
    }

    Ok((&[], classes))
}

/// Get instances associated with a namespace
fn get_instances<'a>(
    class_hash: &str,
    objects_data: &'a [u8],
    pages: &[u32],
    filter_instances: &[String],
) -> nom::IResult<&'a [u8], Vec<InstanceRecord>> {
    let hash_result = extract_hash_info(class_hash);

    let (_hash, record_id) = if let Some(result) = hash_result {
        result
    } else {
        error!("[wmi] Could not split WMI index hashes for instances");
        return Err(nom::Err::Failure(nom::error::Error::new(
            &[],
            ErrorKind::Fail,
        )));
    };
    let (_, object_info) = parse_objects(objects_data, pages)?;

    let mut instances = Vec::new();
    for object in object_info {
        if object.record_id != record_id {
            continue;
        }
        let instance_result = parse_instance_record(&object.object_data);
        let instance = match instance_result {
            Ok((_, result)) => result,
            Err(_err) => {
                continue;
            }
        };

        if !filter_instances.contains(&instance.hash_name) {
            continue;
        }
        instances.push(instance);
    }
    Ok((&[], instances))
}

/// Get Hash and record ID from hash string
fn extract_hash_info(hash: &str) -> Option<(String, u32)> {
    let class_info: Vec<&str> = hash.split('.').collect();
    let mut class_def = (*class_info.first()?).to_string();
    class_def = class_def.replace("CD_", "").replace("IL_", "");

    let record_id_str = class_info.get(2)?;
    let record_id_result = record_id_str.parse::<u32>();
    let record_id = match record_id_result {
        Ok(result) => result,
        Err(err) => {
            error!("[wmi] Could not parse record id number: {err:?}");
            return None;
        }
    };

    Some((class_def, record_id))
}

#[cfg(test)]
mod tests {
    use crate::{
        artifacts::os::windows::wmi::{
            index::parse_index, map::parse_map, windows_management::hash_name,
        },
        filesystem::files::read_file,
    };

    use super::{extract_hash_info, extract_namespace_data, get_classes, get_instances};

    #[test]
    fn test_extract_hash_info() {
        let test = "CD_asdfasdfsadf.1234.12";
        let (hash, id) = extract_hash_info(test).unwrap();
        assert_eq!(hash, "asdfasdfsadf");
        assert_eq!(id, 12);
    }

    #[test]
    #[ignore = "Takes time to run"]
    fn test_get_classes() {
        let map_data = read_file("C:\\Windows\\System32\\wbem\\Repository\\MAPPING3.MAP").unwrap();
        let (_, results) = parse_map(&map_data).unwrap();
        let object_data =
            read_file("C:\\Windows\\System32\\wbem\\Repository\\OBJECTS.DATA").unwrap();
        let index_data = read_file("C:\\Windows\\System32\\wbem\\Repository\\INDEX.BTR").unwrap();

        let (_, index_info) = parse_index(&index_data).unwrap();

        let mut namespace_info = Vec::new();
        for entry in index_info.values() {
            for hash in &entry.value_data {
                if hash.starts_with(&String::from("CD_")) || hash.starts_with(&String::from("IL_"))
                {
                    namespace_info.push(entry.value_data.clone());
                    break;
                }
            }
        }

        let classes = vec![String::from("__NAMESPACE")];

        let mut hash_classes = Vec::new();
        for class in &classes {
            hash_classes.push(hash_name(class));
        }

        let mut classes_info = Vec::new();
        for entries in namespace_info {
            for class_entry in entries {
                if class_entry.starts_with("CD_") || class_entry.starts_with("IL_") {
                    let class_info_result =
                        get_classes(&class_entry, &object_data, &results.mappings, &hash_classes);
                    let classes_result = match class_info_result {
                        Ok((_, result)) => result,
                        Err(_err) => {
                            continue;
                        }
                    };
                    if !classes_result.is_empty() {
                        classes_info.push(classes_result);
                    }
                }
            }
        }
    }

    #[test]
    #[ignore = "Takes time to run"]
    fn test_get_instances() {
        let map_data = read_file("C:\\Windows\\System32\\wbem\\Repository\\MAPPING3.MAP").unwrap();
        let (_, results) = parse_map(&map_data).unwrap();
        let object_data =
            read_file("C:\\Windows\\System32\\wbem\\Repository\\OBJECTS.DATA").unwrap();
        let index_data = read_file("C:\\Windows\\System32\\wbem\\Repository\\INDEX.BTR").unwrap();

        let (_, index_info) = parse_index(&index_data).unwrap();

        let mut namespace_info = Vec::new();
        for entry in index_info.values() {
            for hash in &entry.value_data {
                if hash.starts_with(&String::from("CD_")) || hash.starts_with(&String::from("IL_"))
                {
                    namespace_info.push(entry.value_data.clone());
                    break;
                }
            }
        }

        let classes = vec![String::from("__NAMESPACE")];

        let mut hash_classes = Vec::new();
        for class in &classes {
            hash_classes.push(hash_name(class));
        }

        let mut classes_info = Vec::new();
        for entries in namespace_info {
            for class_entry in entries {
                if class_entry.starts_with("CD_") || class_entry.starts_with("IL_") {
                    let class_info_result =
                        get_instances(&class_entry, &object_data, &results.mappings, &hash_classes);
                    let classes_result = match class_info_result {
                        Ok((_, result)) => result,
                        Err(_err) => {
                            continue;
                        }
                    };
                    if !classes_result.is_empty() {
                        classes_info.push(classes_result);
                    }
                }
            }
        }
    }

    #[test]
    #[ignore = "Takes time to run"]
    fn test_extract_namespace_data() {
        let map_data = read_file("C:\\Windows\\System32\\wbem\\Repository\\MAPPING3.MAP").unwrap();
        let (_, results) = parse_map(&map_data).unwrap();
        let object_data =
            read_file("C:\\Windows\\System32\\wbem\\Repository\\OBJECTS.DATA").unwrap();
        let index_data = read_file("C:\\Windows\\System32\\wbem\\Repository\\INDEX.BTR").unwrap();

        let (_, index_info) = parse_index(&index_data).unwrap();

        let mut namespace_info = Vec::new();
        for entry in index_info.values() {
            for hash in &entry.value_data {
                if hash.starts_with(&String::from("CD_")) || hash.starts_with(&String::from("IL_"))
                {
                    namespace_info.push(entry.value_data.clone());
                    break;
                }
            }
        }

        let classes = vec![String::from("__NAMESPACE")];

        let mut hash_classes = Vec::new();
        for class in &classes {
            hash_classes.push(hash_name(class));
        }

        let _ = extract_namespace_data(&namespace_info, &object_data, &results.mappings, &classes);
    }
}
