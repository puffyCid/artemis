use super::{
    error::WmiError, instance::ClassValues, map::parse_map, namespaces::extract_namespace_data,
};
use crate::{
    artifacts::os::windows::{securitydescriptor::sid::grab_sid, wmi::index::parse_index},
    filesystem::{files::read_file, metadata::glob_paths},
};
use common::windows::WmiPersist;
use log::{error, warn};
use sha2::{Digest, Sha256};
use std::collections::BTreeMap;

/// Parse the WMI repository and return data associated with provided classes
pub(crate) fn parse_wmi_repo(
    classes: &[&str],
    map_paths: &str,
    objects_path: &str,
    index_path: &str,
) -> Result<Vec<ClassValues>, WmiError> {
    let maps_result = glob_paths(map_paths);
    let maps = match maps_result {
        Ok(result) => result,
        Err(err) => {
            error!("[wmi] Could not glob maps path {map_paths}: {err:?}");
            return Err(WmiError::GlobMaps);
        }
    };

    let objects_result = read_file(objects_path);
    let objects = match objects_result {
        Ok(result) => result,
        Err(err) => {
            error!("[wmi] Could not read objects file {objects_path}: {err:?}");
            return Err(WmiError::ReadObjects);
        }
    };

    let index_result = read_file(index_path);
    let index = match index_result {
        Ok(result) => result,
        Err(err) => {
            error!("[wmi] Could not read index file {index_path}: {err:?}");
            return Err(WmiError::ReadIndex);
        }
    };

    let mut seq = 0;
    let mut pages = Vec::new();

    for map in maps {
        let map_data_result = read_file(&map.full_path);
        let map_data = match map_data_result {
            Ok(result) => result,
            Err(err) => {
                error!("[wmi] Could not read map file {}: {err:?}", map.full_path);
                return Err(WmiError::ReadMaps);
            }
        };

        let map_info_result = parse_map(&map_data);
        let map_info = match map_info_result {
            Ok((_, result)) => result,
            Err(err) => {
                error!("[wmi] Could not parse map file {}: {err:?}", map.full_path);
                return Err(WmiError::ParseMap);
            }
        };

        // Need to use the map file with the highest sequence
        if map_info.seq_number2 > seq {
            seq = map_info.seq_number2;
            pages = map_info.mappings;
        }
    }

    let index_info_result = parse_index(&index);
    let index_info = match index_info_result {
        Ok((_, result)) => result,
        Err(err) => {
            error!("[wmi] Could not parse index file {index_path}: {err:?}");
            return Err(WmiError::ParseIndex);
        }
    };

    let mut namespace_info = Vec::new();
    for entry in index_info.values() {
        for hash in &entry.value_data {
            if hash.starts_with(&String::from("CD_")) || hash.starts_with(&String::from("IL_")) {
                namespace_info.push(entry.value_data.clone());
                break;
            }
        }
    }

    let mut hash_classes = Vec::new();
    for class in classes {
        hash_classes.push(hash_name(class));
    }

    let class_data = extract_namespace_data(&namespace_info, &objects, &pages, &hash_classes);

    Ok(class_data)
}

/*
 * After parsing WMI repo, extract persistence data
 */
pub(crate) fn get_wmi_persist(namespace_data: &[ClassValues]) -> Result<Vec<WmiPersist>, WmiError> {
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

    persist_vec.dedup();
    Ok(persist_vec)
}

/// Combine all classes related to WMI persistence data
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
    let consumer_name = consumer_value.to_string().replace('"', "");

    let filter_consumer_opt = filter_consumer.values.get("Consumer");
    let filter_consumer_value = match filter_consumer_opt {
        Some(result) => result,
        None => return,
    };

    let filter_consumer_name = filter_consumer_value.to_string().replace(['"', '\\'], "");
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
        .replace(['\"', '\\'], "");
    let event_filter_opt = event_filter.values.get("Name");
    let event_filter_value = match event_filter_opt {
        Some(result) => result,
        None => return,
    };

    let event_filter_name = event_filter_value.to_string().replace('"', "");
    if format!("__EventFilter.Name={event_filter_name}") != filter_consumer_filter {
        return;
    }

    let event_filter_query_opt = event_filter.values.get("Query");
    let event_filter_query_value = match event_filter_query_opt {
        Some(result) => result,
        None => return,
    };

    let event_filter_query = event_filter_query_value.to_string().replace('"', "");

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
    format!("{hash_name:x}").to_uppercase()
}

#[cfg(test)]
#[cfg(target_os = "windows")]
mod tests {
    use super::{assemble_wmi_persist, get_wmi_persist, hash_name, parse_wmi_repo};
    use common::windows::WmiPersist;
    use std::collections::BTreeMap;

    #[test]
    #[cfg(target_os = "windows")]
    #[ignore = "Takes time to run"]
    fn test_parse_wmi_repo() {
        let classes = vec!["__NAMESPACE"];
        let drive = 'C';

        let map_paths = format!("{drive}:\\Windows\\System32\\wbem\\Repository\\MAPPING*.MAP");
        let objects_path = format!("{drive}:\\Windows\\System32\\wbem\\Repository\\OBJECTS.DATA");
        let index_path = format!("{drive}:\\Windows\\System32\\wbem\\Repository\\INDEX.BTR");
        let results = parse_wmi_repo(&classes, &map_paths, &objects_path, &index_path).unwrap();

        assert!(results.len() > 3);
    }

    #[test]
    fn test_hash_name() {
        let name = "name";
        let result = hash_name(name);
        assert_eq!(
            result,
            "5F7920B75914FA9869AC87CF44262E78C0A9B5751CCB3610B2392617F72D95CD"
        );
    }

    #[test]
    #[ignore = "Takes time to run"]
    fn test_get_wmi_persist() {
        let classes = vec!["__EventConsumer"];
        let drive = 'C';

        let map_paths = format!("{drive}:\\Windows\\System32\\wbem\\Repository\\MAPPING*.MAP");
        let objects_path = format!("{drive}:\\Windows\\System32\\wbem\\Repository\\OBJECTS.DATA");
        let index_path = format!("{drive}:\\Windows\\System32\\wbem\\Repository\\INDEX.BTR");
        let results = parse_wmi_repo(&classes, &map_paths, &objects_path, &index_path).unwrap();

        let _ = get_wmi_persist(&results).unwrap();
    }

    #[test]
    #[ignore = "Takes time to run"]
    fn test_assemble_wmi_persist() {
        let classes = vec!["__EventConsumer"];
        let drive = 'C';

        let map_paths = format!("{drive}:\\Windows\\System32\\wbem\\Repository\\MAPPING*.MAP");
        let objects_path = format!("{drive}:\\Windows\\System32\\wbem\\Repository\\OBJECTS.DATA");
        let index_path = format!("{drive}:\\Windows\\System32\\wbem\\Repository\\INDEX.BTR");
        let results = parse_wmi_repo(&classes, &map_paths, &objects_path, &index_path).unwrap();

        let mut persist_vec = Vec::new();
        for event_consumer in &results {
            if event_consumer.super_class_name != "__EventConsumer" {
                continue;
            }

            for filter_consumer in &results {
                if filter_consumer.class_name != "__FilterToConsumerBinding" {
                    continue;
                }
                for event_filter in &results {
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
                    assemble_wmi_persist(
                        event_consumer,
                        filter_consumer,
                        event_filter,
                        &mut persist,
                    );
                    if !persist.class.is_empty() {
                        persist_vec.push(persist);
                        break;
                    }
                }
            }
        }

        persist_vec.dedup();
    }
}
