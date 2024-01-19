use super::{
    class::ClassInfo,
    index::IndexBody,
    objects::{parse_objects, parse_record},
};
use nom::bytes::complete::take;
use std::collections::HashMap;

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

    let definition =
        String::from("CD_64659AB9F8F1C4B568DB6438BAE11B26EE8F93CB5F8195E21E8C383D6C44CC41");
    for entries in namespace_info {
        for keys in entries {
            if keys.contains(&definition) {
                let (_, class_info) = get_namespace_classes(keys, objects_data, pages)?;
                println!("{class_info:?}");
            }
        }
    }

    Ok((objects_data, names))
}

pub(crate) fn get_namespace_classes<'a>(
    class_hash: String,
    objects_data: &'a [u8],
    pages: &[u32],
) -> nom::IResult<&'a [u8], Vec<ClassInfo>> {
    let class_info: Vec<&str> = class_hash.split('.').collect();

    let logical_page_str = class_info.get(1).unwrap();
    let logical_page = logical_page_str.parse::<usize>().unwrap();

    let record_id_str = class_info.get(2).unwrap();
    let record_id = record_id_str.parse::<u32>().unwrap();

    let page_size = 8192;
    let page = pages.get(logical_page).unwrap();
    let (data, _) = take(page * page_size)(objects_data)?;

    let (_, object_info) = parse_objects(objects_data, pages)?;

    let mut classes = Vec::new();
    for object in object_info {
        if object.record_id == record_id {
            let class_result = parse_record(&object.object_data);
            let class = match class_result {
                Ok((_, result)) => result,
                Err(err) => {
                    println!("failed to parse record");
                    continue;
                }
            };
            classes.push(class);
        }
    }

    Ok((data, classes))
}
