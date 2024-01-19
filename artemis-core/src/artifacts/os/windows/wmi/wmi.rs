use super::{error::WmiError, map::parse_map, namespaces::gather_namespaces};
use crate::{
    artifacts::os::windows::wmi::{index::parse_index, namespaces::get_namespace_classes},
    filesystem::{files::read_file, metadata::glob_paths},
};

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

    for entries in namespace_info {
        for class_entry in entries {
            if class_entry.starts_with("CD_") {
                let class_result = get_namespace_classes(class_entry, &objects, &pages);
                let class_info = match class_result {
                    Ok((_, result)) => result,
                    Err(err) => {
                        println!("failed to get namespace");
                        continue;
                    }
                };
                println!("{class_info:?}");
            }
        }
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::parse_wmi_repo;

    #[test]
    fn test_parse_wmi_repo() {
        let namespaces = [
            String::from("NS_892f8db69c4edfbc68165c91087b7a08323f6ce5b5ef342c0f93e02a0590bfc4")
                .to_uppercase(),
            String::from("NS_e1dd43413ed9fd9c458d2051f082d1d739399b29035b455f09073926e5ed9870")
                .to_uppercase(),
        ];
        let drive = 'C';

        let results = parse_wmi_repo(&namespaces, &drive).unwrap();
    }
}
