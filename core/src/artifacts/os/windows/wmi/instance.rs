use super::class::{CimType, ClassInfo, Property};
use crate::{
    artifacts::os::windows::wmi::{
        class::{extract_cim_data, parse_qualifier},
        windows_management::hash_name,
    },
    utils::{
        nom_helper::{
            Endian, nom_unsigned_eight_bytes, nom_unsigned_four_bytes, nom_unsigned_one_byte,
        },
        strings::extract_utf16_string,
    },
};
use log::warn;
use nom::{bytes::complete::take, error::ErrorKind};
use serde_json::Value;
use std::collections::{BTreeMap, HashMap, HashSet};

#[derive(Debug, Clone)]
pub(crate) struct InstanceRecord {
    pub(crate) hash_name: String,
    pub(crate) _unknown_filetime: u64,
    pub(crate) _unknown_filetime2: u64,
    pub(crate) data: Vec<u8>,
    pub(crate) _class_name_offset: u32,
}

/// Get Instance record info from data
pub(crate) fn parse_instance_record(data: &[u8]) -> nom::IResult<&[u8], InstanceRecord> {
    let hash_size: u8 = 128;
    let (input, hash_data) = take(hash_size)(data)?;
    let hash_name = extract_utf16_string(hash_data);
    if hash_name.len() < 10 {
        // Not instance record
        return Err(nom::Err::Failure(nom::error::Error::new(
            &[],
            ErrorKind::Fail,
        )));
    }

    let (input, unknown_filetime) = nom_unsigned_eight_bytes(input, Endian::Le)?;
    let (input, unknown_filetime2) = nom_unsigned_eight_bytes(input, Endian::Le)?;

    let (input, block_size) = nom_unsigned_four_bytes(input, Endian::Le)?;

    let adjust_block = 4;
    // Size includes block size itself. Which has already been nom'd
    let (input, block_data) = take(block_size - adjust_block)(input)?;

    let (remaining, class_name_offset) = nom_unsigned_four_bytes(block_data, Endian::Le)?;
    let (remaining, _unknown) = nom_unsigned_one_byte(remaining, Endian::Le)?;

    let instance = InstanceRecord {
        hash_name,
        _unknown_filetime: unknown_filetime,
        _unknown_filetime2: unknown_filetime2,
        data: remaining.to_vec(),
        _class_name_offset: class_name_offset,
    };

    Ok((input, instance))
}

#[derive(Debug)]
pub(crate) struct ClassValues {
    pub(crate) class_name: String,
    pub(crate) _class_hash: String,
    pub(crate) super_class_name: String,
    pub(crate) values: BTreeMap<String, Value>,
}

/// Using both classes and instances match the instances data with the correct class
pub(crate) fn parse_instances<'a>(
    classes: &mut [HashMap<String, ClassInfo>],
    instances: &'a [InstanceRecord],
    lookup_parents: &[HashMap<String, ClassInfo>],
) -> nom::IResult<&'a [u8], Vec<ClassValues>> {
    let mut class_values = Vec::new();

    let mut empty_instances = HashSet::new();
    for instance in instances {
        if empty_instances.contains(&instance.hash_name) {
            continue;
        }
        for class in &mut *classes {
            if let Some(class_value) = class.get_mut(&instance.hash_name) {
                let value_result = grab_instance_data(instance, class_value, lookup_parents);
                let class_value = match value_result {
                    Ok((_, result)) => result,
                    Err(_err) => {
                        warn!(
                            "[wmi] Could not grab instance data for class {}. There may not be any data",
                            class_value.class_name
                        );
                        empty_instances.insert(instance.hash_name.clone());
                        continue;
                    }
                };
                class_values.push(class_value);
                break;
            }
        }
    }

    Ok((&[], class_values))
}

/// Match instance data with the correct class
fn grab_instance_data<'a>(
    instance: &'a InstanceRecord,
    class_value: &mut ClassInfo,
    lookup_parents: &[HashMap<String, ClassInfo>],
) -> nom::IResult<&'a [u8], ClassValues> {
    if !class_value.super_class_name.is_empty() && !class_value.includes_parent_props {
        for parent_class in lookup_parents {
            if let Some(parent_class) = parent_class.get(&hash_name(&class_value.super_class_name))
            {
                for parent in &parent_class.properties {
                    let mut has_parent = false;
                    for child_prop in &class_value.properties {
                        if child_prop.name == parent.name {
                            has_parent = true;
                            break;
                        }
                    }
                    if !has_parent {
                        class_value.properties.push(parent.clone());
                    }
                }
                class_value.includes_parent_props = true;
            }
        }
    }

    let prop_count = class_value.properties.len();
    let (remaining, _prop_bit_data) = parse_instance_props(&instance.data, &prop_count)?;

    // Calculate the total property data containing offsets size
    let prop_data_size = get_prop_data_size(&class_value.properties);
    let (remaining, prop_data_offsets) = take(prop_data_size)(remaining)?;
    let (mut remaining, mut qualifier_size) = nom_unsigned_four_bytes(remaining, Endian::Le)?;

    let adjust_size = 4;
    // May be padding at end?
    if qualifier_size < adjust_size {
        let (qual_remaining, size) = nom_unsigned_four_bytes(remaining, Endian::Le)?;
        qualifier_size = size;
        remaining = qual_remaining;
    }

    let (remaining, qual_data) = take(qualifier_size - adjust_size)(remaining)?;
    let (_, _qualifiers) = parse_qualifier(qual_data, remaining)?;
    let (mut remaining, dynamic_prop) = nom_unsigned_one_byte(remaining, Endian::Le)?;

    let has_dynamic = 2;
    if dynamic_prop == has_dynamic {
        let (remaining_input, _) = parse_dynamic_props(remaining)?;
        remaining = remaining_input;
    }

    let (remaining, values_size) = nom_unsigned_four_bytes(remaining, Endian::Le)?;
    let adjust_value_size = 0x7FFFFFFF;
    let size = values_size & adjust_value_size;
    let (_, value_data) = take(size)(remaining)?;

    /*
     * Right now we only support parsing WMI persistence data. Its mostly Strings, integers, and bytes
     * In order to parse objects we need to parse the Class specified in the Property Qualifier
     * Ex: In order to parse the "Connection" property in the "MSFT_CliAlias" class we need to parse the "MSFT_CliConnection" class
     *     The "Connection" property points to MSFT_CliConnection via the Qualifier
     * class_name: "MSFT_CliAlias", qualifiers: [], properties: [Property { name: "Connection", property_data_type: Object, property_index: 0, data_offset: 0, class_level: 0, qualifiers: [Qualifier { name: "type", value_data_type: String, data: String("object:MSFT_CliConnection") }], instance_value: Initialized, value: Null]
     */
    let unsupported_types = [CimType::Object, CimType::ArrayObject];

    // Now get the values for each instance property
    let mut prop_value = BTreeMap::new();
    for prop in class_value.properties.iter_mut() {
        if unsupported_types.contains(&prop.property_data_type) {
            continue;
        }

        let (start, _) = take(prop.data_offset)(prop_data_offsets)?;
        let result = match extract_cim_data(&prop.property_data_type, start, value_data) {
            Ok((_, result)) => result,
            // CIM data can be null
            Err(_err) => Value::Null,
        };
        prop_value.insert(prop.name.clone(), result);
    }
    let class_value = ClassValues {
        class_name: class_value.class_name.clone(),
        _class_hash: class_value.class_hash.clone(),
        super_class_name: class_value.super_class_name.clone(),
        values: prop_value,
    };

    Ok((&[], class_value))
}

/// Parse dynamic property data if identified
fn parse_dynamic_props(data: &[u8]) -> nom::IResult<&[u8], ()> {
    let (mut input, number_instances) = nom_unsigned_four_bytes(data, Endian::Le)?;
    let mut count = 0;
    while count < number_instances {
        let (remaining, size) = nom_unsigned_four_bytes(input, Endian::Le)?;
        let adjust_size = 4;
        let (remaining, _data) = take(size - adjust_size)(remaining)?;
        input = remaining;
        count += 1;
    }
    Ok((input, ()))
}

/// Get the size in bytes of the property data offsets
fn get_prop_data_size(props: &[Property]) -> u32 {
    let mut total_size = 0;
    let eight_bytes = [CimType::Uint64, CimType::Sint64, CimType::Real64];
    let two_bytes = [CimType::Uint16, CimType::Sint16, CimType::Bool];
    let one_byte = [CimType::Uint8, CimType::Sint8];

    for prop in props {
        let size = if eight_bytes.contains(&prop.property_data_type) {
            8
        } else if two_bytes.contains(&prop.property_data_type) {
            2
        } else if one_byte.contains(&prop.property_data_type) {
            1
        } else {
            4
        };
        if (prop.data_offset + size) > total_size {
            total_size = prop.data_offset + size;
        }
    }

    total_size
}

/// Determine instance property data
fn parse_instance_props<'a>(data: &'a [u8], prop_count: &usize) -> nom::IResult<&'a [u8], Vec<u8>> {
    let mut bit_size = prop_count * 2;
    let align = 3;
    // Align to next byte
    bit_size = ((bit_size + 7) >> align) << align;
    let (remaining, prop_data) = take(bit_size / 8)(data)?;

    Ok((remaining, prop_data.to_vec()))
}

#[cfg(test)]
#[cfg(target_os = "windows")]
mod tests {
    use super::{
        InstanceRecord, get_prop_data_size, grab_instance_data, parse_dynamic_props,
        parse_instance_props, parse_instance_record, parse_instances,
    };
    use crate::{
        artifacts::os::windows::wmi::{
            class::{CimType, Property},
            index::parse_index,
            map::parse_map,
            namespaces::get_classes,
            objects::parse_objects,
        },
        filesystem::files::read_file,
    };
    use std::path::PathBuf;

    #[test]
    fn test_parse_instance_record() {
        let mut test_location = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
        test_location.push("tests/test_data/windows/wmi/instance.raw");

        let data = read_file(test_location.to_str().unwrap()).unwrap();
        let (_, results) = parse_instance_record(&data).unwrap();
        assert_eq!(
            results.hash_name,
            "FD3BBD4C42AC2F7FEF264340A9EB78A581FE5C28561CB63EDF780B329EAC2EE4"
        );
    }

    #[test]
    fn test_parse_dynamic_props() {
        let data = vec![1, 0, 0, 0, 5, 0, 0, 0, 0, 0, 0];
        let (results, _) = parse_dynamic_props(&data).unwrap();
        assert_eq!(results.len(), 2);
    }

    #[test]
    fn test_get_prop_data_size() {
        let prop = Property {
            name: String::from("test"),
            property_data_type: CimType::Uint8,
            _property_index: 0,
            data_offset: 0,
            _class_level: 0,
            _qualifiers: Vec::new(),
        };

        let result = get_prop_data_size(&vec![prop]);
        assert_eq!(result, 1);
    }

    #[test]
    fn test_parse_instance_props() {
        let data = vec![1, 0, 0, 0, 1, 0, 0, 0, 1];
        let count = 1;
        let (_, result) = parse_instance_props(&data, &count).unwrap();
        assert_eq!(result.len(), 1);
    }

    #[test]
    fn test_parse_instances() {
        let map_data = read_file("C:\\Windows\\System32\\wbem\\Repository\\MAPPING3.MAP").unwrap();
        let (_, results) = parse_map(&map_data).unwrap();
        let object_data =
            read_file("C:\\Windows\\System32\\wbem\\Repository\\OBJECTS.DATA").unwrap();
        let index_data = read_file("C:\\Windows\\System32\\wbem\\Repository\\INDEX.BTR").unwrap();

        let (_, index_info) = parse_index(&index_data).unwrap();

        let mut namespace_info = Vec::new();
        for entry in index_info {
            for hash in &entry.value_data {
                if hash.starts_with(&String::from("CD_")) || hash.starts_with(&String::from("IL_"))
                {
                    namespace_info.push(entry.value_data.clone());
                    break;
                }
            }
        }

        let mut classes_info = Vec::new();
        let instances_info = Vec::new();
        let (_, object_info) = parse_objects(&object_data, &results.mappings).unwrap();

        for entries in namespace_info {
            for class_entry in entries {
                if class_entry.starts_with("CD_") || class_entry.starts_with("IL_") {
                    let class_info_result = get_classes(&class_entry, &object_info);
                    let classes_result = match class_info_result {
                        Ok((_, result)) => result,
                        Err(_err) => {
                            continue;
                        }
                    };
                    if !classes_result.is_empty() {
                        classes_info.push(classes_result);
                    }
                }
            }
        }
        let lookup_parents = classes_info.clone();

        let _ = parse_instances(&mut classes_info, &instances_info, &lookup_parents).unwrap();
    }

    #[test]
    fn test_grab_instance_data() {
        let map_data = read_file("C:\\Windows\\System32\\wbem\\Repository\\MAPPING3.MAP").unwrap();
        let (_, results) = parse_map(&map_data).unwrap();
        let object_data =
            read_file("C:\\Windows\\System32\\wbem\\Repository\\OBJECTS.DATA").unwrap();
        let index_data = read_file("C:\\Windows\\System32\\wbem\\Repository\\INDEX.BTR").unwrap();

        let (_, index_info) = parse_index(&index_data).unwrap();

        let mut namespace_info = Vec::new();
        for entry in index_info {
            for hash in &entry.value_data {
                if hash.starts_with(&String::from("CD_")) || hash.starts_with(&String::from("IL_"))
                {
                    namespace_info.push(entry.value_data.clone());
                    break;
                }
            }
        }

        let mut classes_info = Vec::new();
        let instances_info: Vec<InstanceRecord> = Vec::new();
        let (_, object_info) = parse_objects(&object_data, &results.mappings).unwrap();

        for entries in namespace_info {
            for class_entry in entries {
                if class_entry.starts_with("CD_") || class_entry.starts_with("IL_") {
                    let class_info_result = get_classes(&class_entry, &object_info);
                    let classes_result = match class_info_result {
                        Ok((_, result)) => result,
                        Err(_err) => {
                            continue;
                        }
                    };
                    if !classes_result.is_empty() {
                        classes_info.push(classes_result);
                    }
                }
            }
        }
        let lookup_parents = classes_info.clone();

        let mut class_values = Vec::new();

        for instance in instances_info {
            for class in &mut *classes_info {
                for (_class_key, class_value) in class.iter_mut() {
                    if class_value.class_hash != instance.hash_name {
                        continue;
                    }

                    let value_result = grab_instance_data(&instance, class_value, &lookup_parents);
                    let class_value = match value_result {
                        Ok((_, result)) => result,
                        Err(_err) => {
                            continue;
                        }
                    };
                    class_values.push(class_value);
                    break;
                }
            }
        }
    }
}
