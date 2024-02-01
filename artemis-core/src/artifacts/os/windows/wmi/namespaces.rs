use super::{
    class::{ClassInfo, Property},
    index::IndexBody,
    instance::{ClassValues, InstanceRecord},
    objects::{parse_objects, parse_record},
};
use crate::artifacts::os::windows::wmi::instance::{parse_instance_record, parse_instances};
use std::collections::HashMap;

#[derive(Debug)]
pub(crate) struct NamespaceData {
    pub(crate) classes: Vec<HashMap<String, Vec<ClassInfo>>>,
    pub(crate) instances: Vec<InstanceRecord>,
    pub(crate) values: Vec<ClassValues>,
}

/// Get all namespaces in WMI repo
pub(crate) fn gather_namespaces<'a>(
    index: &HashMap<u32, IndexBody>,
    objects_data: &'a [u8],
    pages: &[u32],
) -> nom::IResult<&'a [u8], Vec<String>> {
    let names = Vec::new();
    let namespace_root =
        String::from("NS_E8C4F9926E52E9240C37C4E59745CEB61A67A77C9F6692EA4295A97E0AF583C5");
    let namespace =
        String::from("NS_FCBAF5A1255D45B1176570C0B63AA60199749700C79A11D5811D54A83A1F4EFD");
    let mut namespace_info = Vec::new();
    for entry in index.values() {
        if entry.value_data.contains(&namespace) || entry.value_data.contains(&namespace_root) {
            namespace_info.push(entry.value_data.clone());
        }
    }
    println!("{namespace_info:?}");
    let mut cache_props = HashMap::new();

    let data = extract_namespace_data(&namespace_info, objects_data, pages, index, &mut cache_props);
    for entries in data {
        for value in entries.values {
            println!("{value:?}")
        }
    }

    Ok((objects_data, names))
}

/// Extract Properties, Classes, and Instances from a Namespace
pub(crate) fn extract_namespace_data(
    namespace_vec: &Vec<Vec<String>>,
    objects: &[u8],
    pages: &[u32],
    index_info: &HashMap<u32, IndexBody>,
    prop_cache_tracker: &mut HashMap<String, Vec<Property>>
) -> Vec<NamespaceData> {
    let mut spaces = Vec::new();
    let mut full_tracker = Vec::new();
    let mut instances_vec = Vec::new();

    // loop to parse all namespaces of WMI repo
    for entries in namespace_vec {
        let mut tracker = HashMap::new();
        // First get all the class data associated with namespace
        println!("entries len: {}", entries.len());
        for class_entry in entries {
            if class_entry.starts_with("CD_") || class_entry.starts_with("IL_") {
                let instance_result =
                    get_namespace_classes(&class_entry, objects, pages, &mut tracker);
                let mut instances = match instance_result {
                    Ok((_, result)) => result,
                    Err(_err) => {
                        println!("failed to get namespace");
                        continue;
                    }
                };
                instances_vec.append(&mut instances);
            } 
        }

        for classes in tracker.values() {
            for class in classes {
                prop_cache_tracker.insert(class.class_name.clone(), class.properties.clone());
            }
        }
        full_tracker.push(tracker.clone());
    }

    // Now parse the Class instances associated with namespace
    let (_, result) = parse_instances(
        &mut full_tracker,
        &instances_vec,
        &index_info,
        &objects,
        &pages,
        prop_cache_tracker
    )
    .unwrap();
    let value = NamespaceData {
        classes: full_tracker,
        instances: instances_vec,
        values: result,
    };
    spaces.push(value);
    spaces
}

/// Get a single Namespace. The namespace should be SHA256 hashed without "NS_" prefix
pub(crate) fn get_namespace(namespace: &str, index_info: &HashMap<u32, IndexBody>) -> Vec<String> {
    let mut namespace_info = Vec::new();
    for entry in index_info.values() {
        if entry
            .value_data
            .contains(&format!("NS_{namespace}").to_uppercase())
        {
            namespace_info.append(&mut entry.value_data.clone());
        }
    }
    namespace_info
}

pub(crate) fn get_namespace_classes<'a>(
    class_hash: &str,
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
    //let (data, _) = take(page * page_size)(objects_data)?;

    let (_, object_info) = parse_objects(objects_data, pages, page)?;

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

    Ok((objects_data, instances))
}
