// https://github.com/libyal/dtformats/blob/main/documentation/WMI%20repository%20file%20format.asciidoc#instance-object-record---version-22
// See https://github.com/libyal/dtformats/blob/main/documentation/WMI%20repository%20file%20format.asciidoc#instance_block

use super::class::{CimType, ClassInfo, Qualifier};
use crate::utils::{
    nom_helper::{
        nom_unsigned_eight_bytes, nom_unsigned_four_bytes, nom_unsigned_one_byte, Endian,
    },
    strings::extract_utf16_string,
};
use nom::bytes::complete::take;
use serde_json::Value;
use std::{collections::HashMap, mem::size_of};

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

pub(crate) fn parse_instance_record<'a>(data: &'a [u8]) -> nom::IResult<&'a [u8], InstanceRecord> {
    let hash_size: u8 = 128;
    let (input, hash_data) = take(hash_size)(data)?;
    let hash_name = extract_utf16_string(hash_data);
    //println!("{hash_name}");

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

pub(crate) fn parse_instances(
    classes: &HashMap<String, Vec<ClassInfo>>,
    instances: &[InstanceRecord],
) {
    for instance in instances {
        let hash_name = format!("CD_{}", instance.hash_name);
        let name_option = classes.get(&hash_name);
        if name_option.is_none() {
            continue;
        }
        let class_entries = name_option.unwrap();
        let mut prop_count = 0;
        // Now need the number of properties
        for class in class_entries {
            prop_count += class.properties.len();
        }
        println!("{class_entries:?}");
        println!(" prop count: {prop_count}");
        //let (remaining, prop_data) = parse_instance_props(&instance.data, &prop_count).unwrap();
        //println!("{prop_data:?}");
        panic!("what!!!");
    }
}

fn parse_instance_props<'a>(data: &'a [u8], prop_count: &usize) -> nom::IResult<&'a [u8], Vec<u8>> {
    let bits = 8;
    let size = prop_count * bits;
    // Must align bytes. https://github.com/libyal/dtformats/blob/main/documentation/WMI%20repository%20file%20format.asciidoc#2211-instance-block
    let align_size = (bits - size) % bits;

    let total_size = size + align_size;
    let (remaining, prop_data) = take(total_size)(data)?;
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
