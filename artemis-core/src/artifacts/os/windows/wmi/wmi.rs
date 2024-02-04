use super::{
    error::WmiError,
    index::IndexBody,
    instance::ClassValues,
    map::parse_map,
    namespaces::{extract_namespace_data, NamespaceData},
};
use crate::{
    artifacts::os::windows::{securitydescriptor::sid::grab_sid, wmi::index::parse_index},
    filesystem::{files::read_file, metadata::glob_paths},
};
use log::warn;
use serde_json::Value;
use sha2::{Digest, Sha256};
use std::collections::{BTreeMap, HashMap};

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

    let mut namespace_info = Vec::new();
    for entry in index_info.values() {
        for hash in &entry.value_data {
            if hash.starts_with(&String::from("CD_")) || hash.starts_with(&String::from("IL_")) {
                namespace_info.push(entry.value_data.clone());
                break;
            }
        }
    }

    let classes = vec![
        "__EventConsumer",
        "__EventFilter",
        "__FilterToConsumerBinding",
    ];

    let mut hash_classes = Vec::new();
    for class in classes {
        hash_classes.push(hash_name(class));
    }

    let namespace_data = extract_namespace_data(&namespace_info, &objects, &pages, &hash_classes);

    // remove after done
    // return namespace_data to caller and caller can get wmi_persist if they want too
    get_wmi_persist(&namespace_data);

    Ok(())
}

#[derive(Debug, PartialEq, Eq)]
pub(crate) struct WmiPersist {
    class: String,
    values: BTreeMap<String, Value>,
    query: String,
    sid: String,
    filter: String,
    consumer: String,
    consumer_name: String,
}

/*
 * After parsing WMI repo, extract persistence data
 */
pub(crate) fn get_wmi_persist(namespace_data: &[ClassValues]) -> Result<(), WmiError> {
    let mut persist_vec = Vec::new();
    for event_consumer in namespace_data {
        if event_consumer.super_class_name != "__EventConsumer" {
            continue;
        }

        for filter_consumer in namespace_data {
            if filter_consumer.class_name != "__FilterToConsumerBinding" {
                continue;
            }
            for event_filter in namespace_data {
                if event_filter.class_name != "__EventFilter" {
                    continue;
                }
                let mut persist = WmiPersist {
                    class: String::new(),
                    values: BTreeMap::new(),
                    query: String::new(),
                    sid: String::new(),
                    filter: String::new(),
                    consumer: String::new(),
                    consumer_name: String::new(),
                };
                assemble_wmi_persist(event_consumer, filter_consumer, event_filter, &mut persist);
                if !persist.class.is_empty() {
                    persist_vec.push(persist);
                    break;
                }
            }
        }
    }

    println!("len pre dedup: {}", persist_vec.len());
    persist_vec.dedup();
    println!("len post dedup: {}", persist_vec.len());

    Ok(())
}

fn assemble_wmi_persist(
    consumer: &ClassValues,
    filter_consumer: &ClassValues,
    event_filter: &ClassValues,
    persist: &mut WmiPersist,
) {
    let consumer_name_opt = consumer.values.get("Name");
    let consumer_value = match consumer_name_opt {
        Some(result) => result,
        None => return,
    };
    let consumer_name = consumer_value.to_string().replace("\"", "");

    let filter_consumer_opt = filter_consumer.values.get("Consumer");
    let filter_consumer_value = match filter_consumer_opt {
        Some(result) => result,
        None => return,
    };

    let filter_consumer_name = filter_consumer_value
        .to_string()
        .replace("\"", "")
        .replace("\\", "");
    if format!("{}.Name={consumer_name}", consumer.class_name) != filter_consumer_name {
        return;
    }

    let filter_consumer_filter_opt = filter_consumer.values.get("Filter");
    let filter_consumer_filter_value = match filter_consumer_filter_opt {
        Some(result) => result,
        None => return,
    };

    let filter_consumer_filter = filter_consumer_filter_value
        .to_string()
        .to_string()
        .replace("\"", "")
        .replace("\\", "");
    let event_filter_opt = event_filter.values.get("Name");
    let event_filter_value = match event_filter_opt {
        Some(result) => result,
        None => return,
    };

    let event_filter_name = event_filter_value.to_string().to_string().replace("\"", "");
    if format!("__EventFilter.Name={event_filter_name}") != filter_consumer_filter {
        return;
    }

    let event_filter_query_opt = event_filter.values.get("Query");
    let event_filter_query_value = match event_filter_query_opt {
        Some(result) => result,
        None => return,
    };

    let event_filter_query = event_filter_query_value.to_string();

    let event_filter_sid_opt = event_filter.values.get("CreatorSID");
    let event_filter_sid_value = match event_filter_sid_opt {
        Some(result) => result,
        None => return,
    };

    let default = Vec::new();
    let sid_data_value = event_filter_sid_value.as_array().unwrap_or(&default);
    let mut sid_data = Vec::new();
    for value in sid_data_value {
        sid_data.push(value.as_u64().unwrap_or(0) as u8);
    }

    if !sid_data.is_empty() {
        let sid_result = grab_sid(&sid_data);
        match sid_result {
            Ok((_, result)) => persist.sid = result,
            Err(_err) => {
                warn!("[wmi-persist] Could not extract SID info");
            }
        }
    }

    persist.consumer = filter_consumer_name;
    persist.values = consumer.values.clone();
    persist.class = consumer.class_name.clone();
    persist.filter = filter_consumer_filter;
    persist.consumer_name = consumer_name;
    persist.query = event_filter_query;
}

/// Hash name for WMI lookups
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
    let hash = format!("{hash_name:x}").to_uppercase();
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
