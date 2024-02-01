use super::{
    class::get_namespace_from_class,
    error::WmiError,
    index::IndexBody,
    instance::ClassValues,
    map::parse_map,
    namespaces::{extract_namespace_data, gather_namespaces, NamespaceData},
};
use crate::{
    artifacts::os::windows::{securitydescriptor::sid::grab_sid, wmi::index::parse_index},
    filesystem::{files::read_file, metadata::glob_paths},
};
use log::warn;
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
   //let (_, spaces) = gather_namespaces(&index_info, &objects, &pages).unwrap();

    let mut namespace_info = Vec::new();
    for entry in index_info.values() {
        for namespace in namespaces {
            if entry.value_data.contains(&namespace) {
                namespace_info.push(entry.value_data.clone());
            }
        }
    }

    let mut cache_props = HashMap::new();

    let namespace_data = extract_namespace_data(&namespace_info, &objects, &pages, &index_info, &mut cache_props);

    // remove after done
    // return namespace_data to caller and caller can get wmi_persist if they want too
    get_wmi_persist(&namespace_data, &index_info, &objects, &pages);

    Ok(())
}

#[derive(Debug)]
pub(crate) struct WmiPersist {
    class: String,
    values: HashMap<String, Value>,
    query: String,
    sid: String,
    filter: String,
    consumer: String,
    consumer_name: String,
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

    let mut cache_props = HashMap::new();

    let filter_namespace = get_namespace_from_class(&filter_hash, indexes);
    let filter_data = extract_namespace_data(&vec![filter_namespace], objects_data, pages, indexes, &mut cache_props);
   
    let filter_consumer_namespace = get_namespace_from_class(&filter_consumer_hash, indexes);
    let filter_consumer_data = extract_namespace_data(
        &vec![filter_consumer_namespace],
        objects_data,
        pages,
        indexes,
        &mut cache_props
    );

    for data in namespace_data {
        for class in &data.values {
            if class.super_class_name == "__EventConsumer" {
                //println!("{class:?}");
                for filter_consumer in &filter_consumer_data {
                    for entry in &filter_consumer.values {
                        if entry.class_name == "__FilterToConsumerBinding" {
                            for event_filter in &filter_data {
                                for event_filter_entry in &event_filter.values {
                                    if event_filter_entry.class_name == "__EventFilter" {
                                        let mut persist = WmiPersist {
                                            class: String::new(),
                                            values: HashMap::new(),
                                            query: String::new(),
                                            sid: String::new(),
                                            filter: String::new(),
                                            consumer: String::new(),
                                            consumer_name: String::new(),
                                        };
                                        assemble_wmi_persist(
                                            class,
                                            entry,
                                            event_filter_entry,
                                            &mut persist,
                                        );
                                        if !persist.class.is_empty() {
                                            println!("{persist:?}");
                                            break;
                                        }
                                    }
                                }
                            }
                        }
                    }
                }
            }
        }
    }

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

    let filter_consumer_name = filter_consumer_value.to_string().replace("\"", "").replace("\\", "");
    if format!("{}.Name={consumer_name}", consumer.class_name) != filter_consumer_name {
        return;
    }

    let filter_consumer_filter_opt = filter_consumer.values.get("Filter");
    let filter_consumer_filter_value = match filter_consumer_filter_opt {
        Some(result) => result,
        None => return,
    };

    let filter_consumer_filter = filter_consumer_filter_value.to_string().to_string().replace("\"", "").replace("\\", "");
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
