use super::{
    class::{parse_class, ClassInfo},
    error::WmiError,
    map::parse_map,
    objects::parse_objects,
};
use crate::{
    artifacts::os::windows::wmi::{index::parse_index, objects::parse_record},
    filesystem::{files::read_file, metadata::glob_paths},
};
use nom::bytes::complete::take;

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
        for namespace in namespaces {
            if entry.value_data.contains(&namespace) {
                namespace_info.push(entry.value_data.clone());
            }
        }
    }
    println!("{namespace_info:?}");
    println!("{:?}", pages[2425]);

    for entries in namespace_info {
        for class_entry in entries {
            if class_entry.starts_with("CD_") {
                let (_, class_info) = get_namespace_classes(class_entry, &objects, &pages).unwrap();
                println!("{class_info:?}");
            }
        }
    }

    Ok(())
}

fn get_namespace_classes<'a>(
    class_hash: String,
    objects_data: &'a [u8],
    pages: &[u32],
) -> nom::IResult<&'a [u8], ()> {
    let class_info: Vec<&str> = class_hash.split('.').collect();

    let logical_page_str = class_info.get(1).unwrap();
    let logical_page = logical_page_str.parse::<usize>().unwrap();

    let record_id_str = class_info.get(2).unwrap();
    let record_id = record_id_str.parse::<u32>().unwrap();

    let page_size = 8192;
    let page = pages.get(logical_page).unwrap();
    let (data, _) = take(page * page_size)(objects_data)?;

    let (_, object_info) = parse_objects(data)?;

    for object in object_info {
        if object.record_id == record_id {
            let (_, class) = parse_record(&object.object_data).unwrap();
            println!("class: {class:?}");
        }
    }

    Ok((data, ()))
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
