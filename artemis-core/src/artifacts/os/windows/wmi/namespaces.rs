use super::{
    class::ClassInfo,
    index::IndexBody,
    instance::InstanceRecord,
    objects::{parse_objects, parse_record},
};
use crate::artifacts::os::windows::wmi::instance::parse_instance_record;
use nom::bytes::complete::take;
use std::collections::HashMap;

struct ClassInstance {
    classes: HashMap<String, Vec<ClassInfo>>,
    instances: Vec<InstanceRecord>,
}

/// Get all namespaces in WMI repo
pub(crate) fn gather_namespaces<'a>(
    index: &HashMap<u32, IndexBody>,
    objects_data: &'a [u8],
    pages: &[u32],
) -> nom::IResult<&'a [u8], Vec<String>> {
    let names = Vec::new();
    let namespace =
        String::from("NS_FCBAF5A1255D45B1176570C0B63AA60199749700C79A11D5811D54A83A1F4EFD");
    let mut namespace_info = Vec::new();
    for entry in index.values() {
        if entry.value_data.contains(&namespace) {
            namespace_info.push(entry.value_data.clone());
        }
    }

    let mut tracker = HashMap::new();
    let definition =
        String::from("CD_64659AB9F8F1C4B568DB6438BAE11B26EE8F93CB5F8195E21E8C383D6C44CC41");
    for entries in namespace_info {
        for keys in entries {
            if keys.contains(&definition) {
                let (_, class_info) =
                    get_namespace_classes(keys, objects_data, pages, &mut tracker)?;
            }
        }
    }

    Ok((objects_data, names))
}

pub(crate) fn get_namespace_classes<'a>(
    class_hash: String,
    objects_data: &'a [u8],
    pages: &[u32],
    class_tracker: &mut HashMap<String, Vec<ClassInfo>>,
) -> nom::IResult<&'a [u8], Vec<InstanceRecord>> {
    let class_info: Vec<&str> = class_hash.split('.').collect();
    let class_def = class_info.get(0).unwrap();

    let logical_page_str = class_info.get(1).unwrap();
    let logical_page = logical_page_str.parse::<usize>().unwrap();

    let record_id_str = class_info.get(2).unwrap();
    let record_id = record_id_str.parse::<u32>().unwrap();

    let page_size = 8192;
    let page = pages.get(logical_page).unwrap();
    let (data, _) = take(page * page_size)(objects_data)?;

    let (_, object_info) = parse_objects(objects_data, pages)?;

    let mut classes = Vec::new();
    let mut instances = Vec::new();
    for object in object_info {
        if object.record_id == record_id {
            let class_result = parse_record(&object.object_data, &class_def);
            let class = match class_result {
                Ok((_, result)) => result,
                Err(_err) => {
                    // If we fail, it might be because we encountered an Instance record
                    let instance_result = parse_instance_record(&object.object_data);
                    if instance_result.is_ok() {
                        let (_, instance) = instance_result.unwrap();
                        instances.push(instance);
                        continue;
                    }

                    println!("[wmi] Failed to parse record or instance");
                    continue;
                }
            };
            classes.push(class);
        }
    }

    class_tracker.insert(class_def.to_string(), classes.clone());

    Ok((data, instances))
}
