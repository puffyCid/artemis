use super::class::{CimType, ClassInfo, Property, Qualifier};
use crate::{
    artifacts::os::windows::wmi::{
        class::{extract_cim_data, parse_qualifier},
        wmi::hash_name,
    },
    utils::{
        nom_helper::{
            nom_unsigned_eight_bytes, nom_unsigned_four_bytes, nom_unsigned_one_byte, Endian,
        },
        strings::extract_utf16_string,
    },
};
use log::warn;
use nom::bytes::complete::take;
use serde_json::Value;
use std::collections::{BTreeMap, HashMap};

#[derive(Debug, Clone)]
pub(crate) struct InstanceRecord {
    pub(crate) hash_name: String,
    pub(crate) unknown_filetime: u64,
    pub(crate) unknown_filetime2: u64,
    pub(crate) property_values: Vec<u8>,
    pub(crate) qualifier: Qualifier,
    pub(crate) block_type: u8,
    pub(crate) dynamic_properties: Vec<u8>,
    pub(crate) values: Vec<u8>,
    pub(crate) data: Vec<u8>,
    pub(crate) class_name_offset: u32,
}

pub(crate) fn parse_instance_record(data: &[u8]) -> nom::IResult<&[u8], InstanceRecord> {
    let hash_size: u8 = 128;
    let (input, hash_data) = take(hash_size)(data)?;
    let hash_name = extract_utf16_string(hash_data);

    let (input, unknown_filetime) = nom_unsigned_eight_bytes(input, Endian::Le)?;
    let (input, unknown_filetime2) = nom_unsigned_eight_bytes(input, Endian::Le)?;

    let (input, block_size) = nom_unsigned_four_bytes(input, Endian::Le)?;

    let adjust_block = 4;
    // Size includes block size itself. Which has already been nom'd
    let (input, block_data) = take(block_size - adjust_block)(input)?;

    let (remaining, class_name_offset) = nom_unsigned_four_bytes(block_data, Endian::Le)?;
    let (remaining, _unknown) = nom_unsigned_one_byte(remaining, Endian::Le)?;

    let mut instance = InstanceRecord {
        hash_name,
        unknown_filetime,
        unknown_filetime2,
        property_values: Vec::new(),
        qualifier: Qualifier {
            name: String::new(),
            value_data_type: CimType::Unknown,
            data: Value::Null,
        },
        block_type: 0,
        dynamic_properties: Vec::new(),
        values: Vec::new(),
        data: remaining.to_vec(),
        class_name_offset,
    };

    Ok((input, instance))
}

#[derive(Debug)]
pub(crate) struct ClassValues {
    pub(crate) class_name: String,
    pub(crate) class_hash: String,
    pub(crate) super_class_name: String,
    pub(crate) values: BTreeMap<String, Value>,
}

pub(crate) fn parse_instances<'a>(
    classes: &mut [HashMap<String, ClassInfo>],
    instances: &'a [InstanceRecord],
    lookup_parents: &[HashMap<String, ClassInfo>],
) -> nom::IResult<&'a [u8], Vec<ClassValues>> {
    let mut class_values = Vec::new();

    for instance in instances {
        for class in &mut *classes {
            for (class_key, class_value) in class.iter_mut() {
                if class_value.class_hash != instance.hash_name {
                    continue;
                }

                let value_result = grab_instance_data(instance, class_value, lookup_parents);
                let class_value = match value_result {
                    Ok((_, result)) => result,
                    Err(_err) => {
                        warn!("[wmi] Could not grab instance data for class {}. There may not be any data", class_value.class_name);
                        continue;
                    }
                };
                class_values.push(class_value);
                break;
            }
        }
    }

    println!("values len: {}", class_values.len());

    Ok((&[], class_values))
}

fn grab_instance_data<'a>(
    instance: &'a InstanceRecord,
    class_value: &mut ClassInfo,
    lookup_parents: &[HashMap<String, ClassInfo>],
) -> nom::IResult<&'a [u8], ClassValues> {
    println!("class: {}", class_value.class_name);

    if !class_value.super_class_name.is_empty() && !class_value.includes_parent_props {
        for parent_class in lookup_parents {
            if let Some(parent_class) = parent_class.get(&hash_name(&class_value.super_class_name))
            {
                println!("parent props len: {}", parent_class.properties.len());
                println!("original class info: {:?}", class_value);
                println!("parent props: {:?}", parent_class.properties);
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
    let (remaining, prop_bit_data) = parse_instance_props(&instance.data, &prop_count)?;

    let adjust_size = 4;
    // Calculate the total property data containing offsets size
    let prop_data_size = get_prop_data_size(&class_value.properties);
    let (remaining, prop_data_offsets) = take(prop_data_size)(remaining)?;
    let (remaining, qualifier_size) = nom_unsigned_four_bytes(remaining, Endian::Le)?;
    let (remaining, qual_data) = take(qualifier_size as usize - adjust_size)(remaining)?;
    let (_, qualifiers) = parse_qualifier(qual_data, remaining)?;

    let (mut remaining, dynamic_prop) = nom_unsigned_one_byte(remaining, Endian::Le)?;

    let has_dynamic = 2;
    if dynamic_prop == has_dynamic {
        let (remaining_input, _) = parse_dynamic_props(remaining)?;
        remaining = remaining_input;
    }

    let (remaining, values_size) = nom_unsigned_four_bytes(remaining, Endian::Le)?;
    let adjust_value_size = 0x7FFFFFFF;
    let size = values_size & adjust_value_size;
    let (remaining, value_data) = take(size)(remaining)?;

    /*
     * Right now we only support parsing WMI persistence data. Its mostly Strings, integers, and bytes
     * In order to parse objects we need to parse the Class specified in the Property Qualifier
     * Ex: In order to parse the "Connection" property in the "MSFT_CliAlias" class we need to parse the "MSFT_CliConnection" class
     *     The "Connection" property points to MSFT_CliConnection via the Qualifier
     * class_name: "MSFT_CliAlias", qualifiers: [], properties: [Property { name: "Connection", property_data_type: Object, property_index: 0, data_offset: 0, class_level: 0, qualifiers: [Qualifier { name: "type", value_data_type: String, data: String("object:MSFT_CliConnection") }], instance_value: Initialized, value: Null]
     */
    let unsupported_types = vec![CimType::Object, CimType::ArrayObject];

    // Now get the values for each instance property
    let mut prop_value = BTreeMap::new();
    for prop in class_value.properties.iter_mut() {
        if unsupported_types.contains(&prop.property_data_type) {
            continue;
        }
        let (start, _) = take(prop.data_offset)(prop_data_offsets)?;
        let (_, result) = extract_cim_data(&prop.property_data_type, start, value_data)?;

        prop_value.insert(prop.name.clone(), result);
    }
    let class_value = ClassValues {
        class_name: class_value.class_name.clone(),
        class_hash: class_value.class_hash.clone(),
        super_class_name: class_value.super_class_name.clone(),
        values: prop_value,
    };
    println!("value: {class_value:?}");

    Ok((&[], class_value))
}

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
    return Ok((input, ()));
}

/// Get the size in bytes of the property data offsets
fn get_prop_data_size(props: &[Property]) -> u32 {
    let mut total_size = 0;
    let eight_bytes = vec![CimType::Uint64, CimType::Sint64, CimType::Real64];
    let two_bytes = vec![CimType::Uint16, CimType::Sint16, CimType::Bool];
    let one_byte = vec![CimType::Uint8, CimType::Sint8];

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

fn parse_instance_props<'a>(data: &'a [u8], prop_count: &usize) -> nom::IResult<&'a [u8], Vec<u8>> {
    let mut bit_size = prop_count * 2;
    let align = 3;
    // Align to next byte
    bit_size = ((bit_size + 7) >> align) << align;
    let (remaining, prop_data) = take(bit_size / 8)(data)?;

    Ok((remaining, prop_data.to_vec()))
}

#[cfg(test)]
mod tests {
    use super::parse_instance_record;
    use crate::filesystem::files::read_file;
    use std::path::PathBuf;

    #[test]
    fn test_parse_objects() {
        let mut test_location = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
        test_location.push("tests/test_data/windows/wmi/instance.raw");

        let data = read_file(test_location.to_str().unwrap()).unwrap();
        let (_, results) = parse_instance_record(&data).unwrap();
    }
}
