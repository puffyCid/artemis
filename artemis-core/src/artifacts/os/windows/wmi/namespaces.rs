use super::{
    class::ClassInfo,
    instance::{ClassValues, InstanceRecord},
    objects::{parse_objects, parse_record},
    wmi::hash_name,
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
    // loop to parse all namespaces of WMI repo
    for entries in namespace_vec {
        // First get all the class data associated with namespace
        for class_entry in entries {
            if class_entry.starts_with("CD_") || class_entry.starts_with("IL_") {
                let class_info_result = get_namespace_data(class_entry, objects, pages, classes);
                let (classes_result, mut instances_result) = match class_info_result {
                    Ok((_, result)) => result,
                    Err(_err) => {
                        warn!("failed to get class info for: {class_entry}");
                        continue;
                    }
                };
                if !classes_result.is_empty() {
                    classes_info.push(classes_result);
                }

                instances_info.append(&mut instances_result);
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

pub(crate) fn get_namespace_data<'a>(
    class_hash: &str,
    objects_data: &'a [u8],
    pages: &[u32],
    filter_classes: &[String],
) -> nom::IResult<&'a [u8], (HashMap<String, ClassInfo>, Vec<InstanceRecord>)> {
    let hash_result = extract_hash_info(class_hash);

    let (_hash, record_id) = if let Some(result) = hash_result {
        result
    } else {
        error!("[wmi] Could not split WMI index hashs.");
        return Err(nom::Err::Failure(nom::error::Error::new(
            &[],
            ErrorKind::Fail,
        )));
    };

    let (_, object_info) = parse_objects(objects_data, pages)?;

    let mut classes = HashMap::new();
    let mut instances = Vec::new();
    for object in object_info {
        if object.record_id != record_id {
            continue;
        }
        let class_result = parse_record(&object.object_data);
        let class = match class_result {
            Ok((_, result)) => result,
            Err(_err) => {
                // If we fail, it might be because we encountered an Instance record
                let instance_result = parse_instance_record(&object.object_data);
                if instance_result.is_ok() {
                    // Unwrap safe since we check above
                    let (_, instance) = instance_result.unwrap();
                    // if !filter_classes.contains(&instance.hash_name) {
                    //    continue;
                    // }
                    instances.push(instance);
                    continue;
                }

                warn!("[wmi] Failed to parse record or instance");
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
    Ok((&[], (classes, instances)))
}

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
