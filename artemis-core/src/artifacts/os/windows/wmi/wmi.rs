use super::{
    class::get_namespace_from_class,
    error::WmiError,
    index::IndexBody,
    map::parse_map,
    namespaces::{extract_namespace_data, gather_namespaces, NamespaceData},
};
use crate::{
    artifacts::os::windows::wmi::index::parse_index,
    filesystem::{files::read_file, metadata::glob_paths},
};
use serde_json::Value;
use sha2::{Digest, Sha256};
use std::collections::HashMap;

pub(crate) fn parse_wmi_repo(namespaces: &[String], drive: &char) -> Result<(), WmiError> {
    let map_paths = format!("{drive}:\\Windows\\System32\\wbem\\Repository\\MAPPING*.MAP");
    let objects_path = format!("{drive}:\\Windows\\System32\\wbem\\Repository\\OBJECTS.DATA");
    let index_path = format!("{drive}:\\Windows\\System32\\wbem\\Repository\\INDEX.BTR");

    let maps = glob_paths(&map_paths).unwrap();
    let objects = read_file(&objects_path).unwrap();
    let index = read_file(&index_path).unwrap();

    let mut seq = 0;
    let mut pages = Vec::new();

    for map in maps {
        let map_data = read_file(&map.full_path).unwrap();

        let (_, map_info) = parse_map(&map_data).unwrap();
        if map_info.seq_number2 > seq {
            seq = map_info.seq_number2;
            pages = map_info.mappings;
        }
    }

    let (_, index_info) = parse_index(&index).unwrap();
    let (_, spaces) = gather_namespaces(&index_info, &objects, &pages).unwrap();

    let mut namespace_info = Vec::new();
    for entry in index_info.values() {
        for namespace in namespaces {
            if entry.value_data.contains(&namespace) {
                namespace_info.push(entry.value_data.clone());
            }
        }
    }

    let namespace_data = extract_namespace_data(&namespace_info, &objects, &pages, &index_info);

    // remove after done
    // return namespace_data to caller and caller can get wmi_persist if they want too
    get_wmi_persist(&namespace_data, &index_info, &objects, &pages);

    Ok(())
}

pub(crate) struct WmiPersist {
    class: String,
    values: HashMap<String, Value>,
    query: String,
    sid: String,
}

/*
 * After parsing WMI repo, extract persistence data
 */
pub(crate) fn get_wmi_persist(
    namespace_data: &[NamespaceData],
    indexes: &HashMap<u32, IndexBody>,
    objects_data: &[u8],
    pages: &[u32],
) -> Result<(), WmiError> {
    let filter_consumer_hash = hash_name("__FilterToConsumerBinding");
    let filter_hash = hash_name("__EventFilter");

    let filter_namespace = get_namespace_from_class(&filter_hash, indexes);
    let filter_data = extract_namespace_data(&vec![filter_namespace], objects_data, pages, indexes);
    println!("filter: {:?}", filter_data);
    let filter_consumer_namespace = get_namespace_from_class(&filter_consumer_hash, indexes);
    let filter_consumer_data = extract_namespace_data(
        &vec![filter_consumer_namespace],
        objects_data,
        pages,
        indexes,
    );
    for data in namespace_data {
        for class in &data.values {
            if class.super_class_name == "__EventConsumer" {
                println!("{class:?}");
                for filter_consumer in &filter_consumer_data {
                    for entry in &filter_consumer.values {
                        println!("{entry:?}");
                    }
                }
            }
        }
    }

    Ok(())
}

/// Hash the class name for WMI lookups
pub(crate) fn hash_name(name: &str) -> String {
    let class = name.to_uppercase().as_bytes().to_vec();
    let mut hash = Sha256::new();
    let mut class_data = Vec::new();
    // Needs to be UTF-16 (wide char)
    for bytes in class {
        class_data.push(bytes);
        class_data.push(0);
    }
    hash.update(class_data);
    let hash_name = hash.finalize();
    let hash = format!("{hash_name:x}");
    hash
}

#[cfg(test)]
mod tests {
    use super::parse_wmi_repo;

    #[test]
    fn test_parse_wmi_repo() {
        let namespaces = [
            String::from("NS_892f8db69c4edfbc68165c91087b7a08323f6ce5b5ef342c0f93e02a0590bfc4")
                .to_uppercase(),
            // String::from("NS_e1dd43413ed9fd9c458d2051f082d1d739399b29035b455f09073926e5ed9870")
            //    .to_uppercase(),
        ];
        let drive = 'C';

        let results = parse_wmi_repo(&namespaces, &drive).unwrap();
    }
}
