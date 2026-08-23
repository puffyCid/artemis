use super::{error::WmiError, instance::ClassValues, namespaces::extract_namespace_data};
use crate::{
    accessor::{access::Accessor, entry::handle::DirEntry},
    artifacts::os::windows::{
        securitydescriptor::sid::grab_sid,
        wmi::{index::parse_index, map::parse_map},
    },
};
use base16ct::upper::encode_str;
use common::windows::WmiPersist;
use md5::Md5;
use sha2::{Digest, Sha256};
use std::collections::HashSet;
use tracing::{error, warn};

/// Extract WMI class data from WMI repo directory
pub(crate) fn extract_wmi(wmi_files: Vec<DirEntry>) -> Result<Vec<ClassValues>, WmiError> {
    let mut objects = Vec::new();
    let mut pages = Vec::new();
    let mut index = Vec::new();
    let mut page_seq = 0;

    let mut accessor = Accessor::with_defaults();

    // Get data we need for WMI parsing
    for entry in wmi_files {
        if entry.is_file()
            && entry.name.to_lowercase() == "objects.data"
            && let Some(handle) = entry.handle.as_file()
        {
            objects = match accessor.read_file_handle(handle) {
                Ok(results) => results,
                Err(err) => {
                    error!(
                        "Could not read objects file {}: {err:?}",
                        handle.display_path()
                    );
                    return Err(WmiError::ReadObjects);
                }
            };
        }

        if entry.is_file()
            && entry.name.to_lowercase() == "index.btr"
            && let Some(handle) = entry.handle.as_file()
        {
            let bytes = match accessor.read_file_handle(handle) {
                Ok(results) => results,
                Err(err) => {
                    error!(
                        "Could not read index file {}: {err:?}",
                        handle.display_path()
                    );
                    return Err(WmiError::ReadIndex);
                }
            };

            index = match parse_index(&bytes) {
                Ok((_, result)) => result,
                Err(err) => {
                    error!(
                        "Could not parse index file {}: {err:?}",
                        handle.display_path()
                    );
                    return Err(WmiError::ParseIndex);
                }
            };
        }
        if entry.is_file()
            && entry.name.to_lowercase().contains("mapping")
            && let Some(handle) = entry.handle.as_file()
        {
            let bytes = match accessor.read_file_handle(handle) {
                Ok(results) => results,
                Err(err) => {
                    error!("Could not read map file {}: {err:?}", handle.display_path());
                    return Err(WmiError::ReadMaps);
                }
            };
            let map_info = match parse_map(&bytes) {
                Ok((_, results)) => results,
                Err(err) => {
                    error!("Could not parse WMI map {}: {err:?}", handle.display_path());
                    return Err(WmiError::ParseMap);
                }
            };

            // Need to use the map file with the highest sequence
            if map_info.seq_number2 > page_seq {
                page_seq = map_info.seq_number2;
                pages = map_info.mappings;
            }
        }
    }

    let mut namespace_info = Vec::new();
    for entry in index {
        for hash in &entry.value_data {
            if hash.starts_with("CD_") || hash.starts_with("IL_") {
                namespace_info.push(entry.value_data.clone());
                break;
            }
        }
    }

    let class_data = extract_namespace_data(&namespace_info, &objects, &pages);

    Ok(class_data)
}

/*
 * After parsing WMI repo, extract persistence data
 */
pub(crate) fn get_wmi_persist(
    namespace_data: &[ClassValues],
    evidence: &str,
) -> Result<Vec<WmiPersist>, WmiError> {
    let mut persist_vec = Vec::new();
    // Small tracker when looping through the data
    let mut hits = HashSet::new();
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
                    evidence: evidence.to_string(),
                    ..Default::default()
                };
                assemble_wmi_persist(event_consumer, filter_consumer, event_filter, &mut persist);
                let mut md5 = Md5::new();
                let bytes = serde_json::to_vec(&persist).unwrap_or_default();
                md5.update(&bytes);
                let hash = md5.finalize();
                let mut buf = [0u8; 32];
                let md5_string = encode_str(&hash, &mut buf).unwrap_or_default().to_string();

                if !persist.class.is_empty() && !hits.contains(&md5_string) {
                    persist_vec.push(persist);
                    hits.insert(md5_string);
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

    let filter_consumer_string = filter_consumer_value.to_string();
    let filter_consumer_name = if filter_consumer_string.contains(':') {
        let (_, name) = filter_consumer_string.split_once(':').unwrap_or((
            filter_consumer_string.as_str(),
            filter_consumer_string.as_str(),
        ));
        name.to_string().replace(['"', '\\'], "")
    } else {
        filter_consumer_string.replace(['"', '\\'], "")
    };
    if format!("{}.Name={consumer_name}", consumer.class_name) != filter_consumer_name {
        return;
    }

    let filter_consumer_filter_opt = filter_consumer.values.get("Filter");
    let filter_consumer_filter_value = match filter_consumer_filter_opt {
        Some(result) => result,
        None => return,
    };

    let filter_consumer_filter_string = filter_consumer_filter_value.to_string();
    let filter_consumer_filter = if filter_consumer_filter_string.contains(':') {
        let (_, name) = filter_consumer_filter_string.split_once(':').unwrap_or((
            filter_consumer_filter_string.as_str(),
            filter_consumer_filter_string.as_str(),
        ));
        name.to_string().replace(['"', '\\'], "")
    } else {
        filter_consumer_filter_string.replace(['"', '\\'], "")
    };

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
    persist.class.clone_from(&consumer.class_name);
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
    let mut buf = [0u8; 64];
    encode_str(&hash_name, &mut buf)
        .unwrap_or_default()
        .to_string()
}

#[cfg(test)]
#[cfg(target_os = "windows")]
mod tests {
    use super::{assemble_wmi_persist, extract_wmi, get_wmi_persist, hash_name};
    use crate::accessor::access::Accessor;
    use common::windows::WmiPersist;

    #[test]
    fn test_extract_wmi() {
        let files = Accessor::with_defaults()
            .read_dir("C:\\Windows\\System32\\wbem\\Repository")
            .unwrap();

        let results = extract_wmi(files).unwrap();

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
    fn test_get_wmi_persist() {
        let files = Accessor::with_defaults()
            .read_dir("C:\\Windows\\System32\\wbem\\Repository")
            .unwrap();

        let results = extract_wmi(files).unwrap();

        let _ = get_wmi_persist(&results, "repo").unwrap();
    }

    #[test]
    fn test_assemble_wmi_persist() {
        let files = Accessor::with_defaults()
            .read_dir("C:\\Windows\\System32\\wbem\\Repository")
            .unwrap();

        let results = extract_wmi(files).unwrap();

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
                    let mut persist = WmiPersist::default();
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
