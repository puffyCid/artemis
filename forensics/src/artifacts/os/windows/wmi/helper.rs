use super::{
    class::ClassInfo, error::WmiError, index::IndexBody, map::parse_map,
    namespaces::extract_namespace_data, windows_management::hash_name,
};
use crate::{
    artifacts::os::windows::wmi::{namespaces::extract_classes, objects::parse_objects},
    filesystem::{files::read_file, metadata::glob_paths},
};
use log::error;
use serde::Serialize;
use serde_json::Value;
use std::collections::{HashMap, HashSet};

#[derive(Debug, Serialize)]
pub(crate) struct Namespaces {
    class: String,
    class_hash: String,
    name: String,
    path: String,
    super_class: String,
}

/// List WMI namespaces
pub(crate) fn list_namespaces(
    namespace: &str,
    indexes: &[IndexBody],
    object_data: &[u8],
    pages: &[u32],
) -> Result<Vec<Namespaces>, WmiError> {
    let namespace_hash = hash_name(namespace);
    let namespace_class = hash_name("__NAMESPACE");
    let mut namespace_info = Vec::new();

    for entry in indexes {
        for hash in &entry.value_data {
            if hash.starts_with(&format!("NS_{namespace_hash}"))
                || hash.starts_with(&format!("CD_{namespace_class}"))
                || hash.starts_with(&format!("IL_{namespace_class}"))
            {
                namespace_info.push(entry.value_data.clone());
            }
        }
    }
    let info = extract_namespace_data(&namespace_info, object_data, pages);

    let mut spaces = Vec::new();
    for entry in info {
        if entry.class_name != "__NAMESPACE" {
            continue;
        }

        let default = Value::String(String::new());
        let name = entry
            .values
            .get("Name")
            .unwrap_or(&default)
            .as_str()
            .unwrap_or_default();
        let space = Namespaces {
            class: entry.class_name,
            class_hash: entry._class_hash,
            path: format!("{namespace}\\{name}"),
            name: name.to_string(),
            super_class: entry.super_class_name,
        };

        spaces.push(space);
    }

    Ok(spaces)
}

/// List classes in a namespace
pub(crate) fn list_classes(
    namespace: &str,
    indexes: &[IndexBody],
    object_data: &[u8],
    pages: &[u32],
) -> Result<Vec<HashMap<String, ClassInfo>>, WmiError> {
    let namespace_hash = hash_name(namespace);

    let mut namespace_info = Vec::new();

    for entry in indexes {
        for hash in &entry.value_data {
            if hash.starts_with(&format!("NS_{namespace_hash}")) {
                namespace_info.push(entry.value_data.clone());
            }
        }
    }
    let object_info = match parse_objects(object_data, pages) {
        Ok((_, result)) => result,
        Err(err) => {
            error!("[wmi] Could not parse objects for namespace {namespace}:{err:?}");
            return Err(WmiError::ReadObjects);
        }
    };
    let mut classes = extract_classes(&namespace_info, &object_info);
    let parents = classes.clone();

    // Keep tracker to avoid recursive parent children. (This should not happen)
    let mut tracker = HashSet::new();
    // Classes inherit properties from their parents
    // We now need to backfill them
    for entry in classes.iter_mut() {
        for value in entry.values_mut() {
            // Empty super class means there are no parents
            if value.super_class_name.is_empty() {
                continue;
            }

            get_parent_class(
                &hash_name(&value.super_class_name),
                &parents,
                &mut tracker,
                value,
            );
            value.includes_parent_props = true;
            tracker = HashSet::new();
        }
    }

    Ok(classes)
}

/// Get optional descriptions for a class
pub(crate) fn class_description(
    namespace: &str,
    locale: u32,
    class_name: &str,
    indexes: &[IndexBody],
    object_data: &[u8],
    pages: &[u32],
) -> Result<ClassInfo, WmiError> {
    let lang_code = format!("{locale:x}");
    let namespace_hash = hash_name(&format!("{namespace}\\ms_{lang_code}"));
    let class = hash_name(class_name);

    let mut namespace_info = Vec::new();

    for entry in indexes {
        for hash in &entry.value_data {
            if hash.starts_with(&format!("NS_{namespace_hash}")) {
                namespace_info.push(entry.value_data.clone());
            }
        }
    }
    let object_info = match parse_objects(object_data, pages) {
        Ok((_, result)) => result,
        Err(err) => {
            error!("[wmi] Could not parse objects for namespace {namespace}:{err:?}");
            return Err(WmiError::ReadObjects);
        }
    };
    let mut classes = extract_classes(&namespace_info, &object_info);
    let parents = classes.clone();

    // Keep tracker to avoid recursive parent children. (This should not happen)
    let mut tracker = HashSet::new();
    // Classes inherit properties from their parents
    // We now need to backfill them
    for entry in classes.iter_mut() {
        if let Some(value) = entry.get_mut(&class) {
            // Empty super class means there are no parents
            if value.super_class_name.is_empty() {
                continue;
            }

            get_parent_class(
                &hash_name(&value.super_class_name),
                &parents,
                &mut tracker,
                value,
            );
            value.includes_parent_props = true;

            // We only backfill the class we are interested in
            return Ok(value.clone());
        }
    }

    error!("[wmi] Could not get class descriptions for {namespace} class {class_name}");
    Err(WmiError::ClassDescriptions)
}

/// Check for parent classes for a provided class
fn get_parent_class(
    parent: &str,
    parents: &[HashMap<String, ClassInfo>],
    tracker: &mut HashSet<String>,
    class: &mut ClassInfo,
) {
    if parent.is_empty() || tracker.contains(parent) {
        return;
    }
    for entry in parents.iter() {
        if let Some(value) = entry.get(parent) {
            if !value.super_class_name.is_empty() && !value.includes_parent_props {
                tracker.insert(value.class_name.clone());
                tracker.insert(value.super_class_name.clone());

                // Keep looking up parents until we get to last parent
                get_parent_class(&hash_name(&value.super_class_name), parents, tracker, class);
            }

            for prop in &value.properties {
                let mut override_parent = false;
                for child_prop in &class.properties {
                    // Child properties override parents
                    if child_prop.name == prop.name {
                        override_parent = true;
                        break;
                    }
                }
                if !override_parent {
                    class.properties.push(prop.clone());
                }
            }
        }
    }
}

/// Get active pages for WMI repo
pub(crate) fn get_pages(map_paths: &str) -> Result<Vec<u32>, WmiError> {
    let maps_result = glob_paths(map_paths);
    let maps = match maps_result {
        Ok(result) => result,
        Err(err) => {
            error!("[wmi] Could not glob maps path {map_paths}: {err:?}");
            return Err(WmiError::GlobMaps);
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

    Ok(pages)
}

#[cfg(test)]
#[cfg(target_os = "windows")]
mod tests {
    use super::{class_description, get_pages, list_classes, list_namespaces};
    use crate::{artifacts::os::windows::wmi::index::parse_index, filesystem::files::read_file};

    #[test]
    fn test_get_pages() {
        let map_paths = "C:\\Windows\\System32\\wbem\\Repository\\MAPPING*.MAP";
        let pages = get_pages(map_paths).unwrap();
        assert!(pages.len() > 10);
    }

    #[test]
    fn test_list_namespaces() {
        let namespace = "root";
        let map_paths = "C:\\Windows\\System32\\wbem\\Repository\\MAPPING*.MAP";
        let index_path = "C:\\Windows\\System32\\wbem\\Repository\\INDEX.BTR";
        let object_path = "C:\\Windows\\System32\\wbem\\Repository\\OBJECTS.DATA";

        let pages = get_pages(map_paths).unwrap();
        let (_, index) = parse_index(&read_file(index_path).unwrap()).unwrap();
        let objects = read_file(object_path).unwrap();

        let result = list_namespaces(namespace, &index, &objects, &pages).unwrap();
        assert!(result.len() > 2);

        for entry in result {
            if entry.name.to_lowercase() == "cimv2" {
                return;
            }
        }

        panic!("did not find cimv2 namespace. This should always exist!");
    }

    #[test]
    fn test_list_classes() {
        let namespace = "root\\cimv2";
        let map_paths = "C:\\Windows\\System32\\wbem\\Repository\\MAPPING*.MAP";
        let index_path = "C:\\Windows\\System32\\wbem\\Repository\\INDEX.BTR";
        let object_path = "C:\\Windows\\System32\\wbem\\Repository\\OBJECTS.DATA";

        let pages = get_pages(map_paths).unwrap();
        let (_, index) = parse_index(&read_file(index_path).unwrap()).unwrap();
        let objects = read_file(object_path).unwrap();

        let result = list_classes(namespace, &index, &objects, &pages).unwrap();
        // There should be hundreds of classes under cimv2
        assert!(result.len() > 50);

        for entry in result {
            if let Some(value) =
                entry.get("9B3B36A993A462F50891454718B9E94854BEF6CDA6B45E39722B69B985CF4900")
            {
                assert_eq!(value.super_class_name, "CIM_BIOSElement");
                assert_eq!(value.class_name, "Win32_BIOS");
                assert!(value.qualifiers.len() > 3);
                assert!(value.properties.len() > 5);
                return;
            }
        }

        panic!("Failed to find Win32_BIOS class. This should exist?");
    }

    #[test]
    fn test_class_descriptions() {
        let namespace = "root\\cimv2";
        let map_paths = "C:\\Windows\\System32\\wbem\\Repository\\MAPPING*.MAP";
        let index_path = "C:\\Windows\\System32\\wbem\\Repository\\INDEX.BTR";
        let object_path = "C:\\Windows\\System32\\wbem\\Repository\\OBJECTS.DATA";

        let pages = get_pages(map_paths).unwrap();
        let (_, index) = parse_index(&read_file(index_path).unwrap()).unwrap();
        let objects = read_file(object_path).unwrap();

        let class_name = "Win32_BIOS";
        let locale = 1033;
        let info =
            class_description(namespace, locale, class_name, &index, &objects, &pages).unwrap();
        assert!(info.properties.len() > 10);
        assert_eq!(info.properties[0].name, "BiosCharacteristics");
        assert_eq!(info.properties[0].qualifiers[1].name, "Description");
        assert_eq!(
            info.properties[0].qualifiers[1].data.as_str().unwrap(),
            "The BiosCharacteristics property identifies the BIOS characteristics supported by the system as defined by the System Management BIOS Reference Specification"
        );
    }
}
