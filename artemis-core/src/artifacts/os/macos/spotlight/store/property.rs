use crate::{
    artifacts::os::macos::spotlight::dbstr::{data::DataAttribute, meta::SpotlightMeta},
    utils::{
        compression::decompress_lz4,
        nom_helper::{
            nom_unsigned_eight_bytes, nom_unsigned_four_bytes, nom_unsigned_one_byte, Endian,
        },
        strings::extract_utf8_string,
    },
};
use byteorder::{LittleEndian, ReadBytesExt};
use log::error;
use nom::{bytes::complete::take, number::complete::le_u64};
use serde::Serialize;
use serde_json::Value;
use std::collections::HashMap;

#[derive(Debug, Serialize)]
pub(crate) struct SpotlightEntries {
    inode: u64,
    parent_inode: u64,
    flags: u64,
    store_id: u64,
    last_updated: u64,
    values: HashMap<String, SpotlightValue>,
}

#[derive(Debug, Serialize)]
pub(crate) struct SpotlightValue {
    attribute: DataAttribute,
    value: Value,
}

pub(crate) fn parse_property<'a>(
    data: &'a [u8],
    meta: &SpotlightMeta,
) -> nom::IResult<&'a [u8], Vec<SpotlightEntries>> {
    let (input, sig) = nom_unsigned_four_bytes(data, Endian::Le)?;
    let (input, page_size) = nom_unsigned_four_bytes(input, Endian::Le)?;
    let (input, used_size) = nom_unsigned_four_bytes(input, Endian::Le)?;

    // Also referred to as block_type
    let (input, property_type_data) = nom_unsigned_four_bytes(input, Endian::Le)?;
    println!("{property_type_data}");

    let property_type = get_property_types(&property_type_data);
    println!("{property_type:?}");

    if !property_type.contains(&PropertyType::Lz4Compressed) {
        panic!("hmm: {property_type:?}");
    }

    // Total uncompressed size (including the 16 previous bytes above)
    // If the value is zero the page/property is not compressed!
    let (input, uncompressed_size) = nom_unsigned_four_bytes(input, Endian::Le)?;

    let (input, compress_sig) = nom_unsigned_four_bytes(input, Endian::Le)?;
    println!("{compress_sig}");

    let (input, decompress_size) = nom_unsigned_four_bytes(input, Endian::Le)?;
    let (input, compress_size) = nom_unsigned_four_bytes(input, Endian::Le)?;
    let (input, compressed_data) = take(compress_size)(input)?;

    let decompress_result = decompress_lz4(compressed_data, decompress_size as usize);
    let mut decom_data = match decompress_result {
        Ok(result) => result,
        Err(err) => {
            error!("[spotlight] Failed to decompress lz4 compression: {err:?}");
            panic!("hmm :)");
        }
    };

    //println!("{decom_data:?}");
    let entries = parse_all_records(&decom_data, meta).unwrap();

    Ok((input, Vec::new()))
}

fn parse_all_records<'a>(
    data: &'a [u8],
    meta: &SpotlightMeta,
) -> nom::IResult<&'a [u8], Vec<SpotlightEntries>> {
    let mut decom_data = data;
    let min_size = 4;
    while decom_data.len() > min_size {
        let (input, record_size) = nom_unsigned_four_bytes(decom_data, Endian::Le)?;
        let (input, record_data) = take(record_size)(input)?;

        decom_data = input;

        let (_, entry) = parse_record(record_data, meta)?;
        break;
    }

    Ok((decom_data, Vec::new()))
}

fn parse_record<'a>(data: &'a [u8], meta: &SpotlightMeta) -> nom::IResult<&'a [u8], ()> {
    let entry = SpotlightEntries {
        inode: 0,
        parent_inode: 0,
        flags: 0,
        store_id: 0,
        last_updated: 0,
        values: HashMap::new(),
    };
    let (input, inode) = parse_variable_size(data)?;
    println!("inode: {inode}");

    let (input, flags) = nom_unsigned_one_byte(input, Endian::Le)?;
    println!("flags: {flags}");

    let (input, store_id) = parse_variable_size(input)?;
    println!("store id: {store_id}");

    let (input, parent_inode) = parse_variable_size(input)?;
    println!("parent inode: {parent_inode}");

    let (mut remaining_input, last_updated) = parse_variable_size(input)?;
    println!("update time: {last_updated}");

    println!("{:?}", meta.props);
    while !remaining_input.is_empty() {
        let (input, index) = parse_variable_size(remaining_input)?;
        println!("index: {index}");
        let prop_opt = meta.props.get(&index);
        let props = match prop_opt {
            Some(result) => result,
            None => {
                panic!("what do i do :( (no properties for {index})");
            }
        };

        println!("{props:?}");
        let (input, prop_value_size) = parse_variable_size(input)?;
        println!("prop value size: {prop_value_size}");

        if prop_value_size > remaining_input.len() {
            break;
        }

        let (input, value_data) = take(prop_value_size)(input)?;
        //let value = extract_utf8_string(value_data);
        //println!("{value}");

        remaining_input = input;
    }

    Ok((data, ()))
}

#[derive(Debug, PartialEq)]
enum PropertyType {
    ZlibDeflateRecords,
    MetaAttributeTypes,
    MetaAttributeValues,
    UnknownProperty,
    ListsStrings,
    Lz4Compressed,
    Unknown,
}

fn get_property_types(data: &u32) -> Vec<PropertyType> {
    let records = 0x9;
    let attr_types = 0x11;
    let attr_values = 0x21;
    let unknown_prop = 0x41;
    let strings = 0x81;
    let lz4_compressed = 0x1000;
    let unknown = 0x4000;

    let mut props = Vec::new();

    if (data & records) == records {
        props.push(PropertyType::ZlibDeflateRecords);
    }
    if (data & attr_types) == attr_types {
        props.push(PropertyType::MetaAttributeTypes);
    }
    if (data & attr_values) == attr_values {
        props.push(PropertyType::MetaAttributeValues);
    }
    if (data & unknown_prop) == unknown_prop {
        props.push(PropertyType::UnknownProperty);
    }
    if (data & strings) == strings {
        props.push(PropertyType::ListsStrings);
    }
    if (data & lz4_compressed) == lz4_compressed {
        props.push(PropertyType::Lz4Compressed);
    }
    if (data & unknown) == unknown {
        props.push(PropertyType::Unknown);
    }

    props
}

fn parse_variable_size(data: &[u8]) -> nom::IResult<&[u8], usize> {
    let (mut input, mut value) = nom_unsigned_one_byte(data, Endian::Le)?;
    println!("var value: {value}");
    let mut lower_nibble = true;
    let mut extra_bytes: usize = 0;
    if value == 0 {
        return Ok((input, value as usize));
    } else if (value & 0xf0) == 0xf0 {
        lower_nibble = false;
        if (value & 0xf) == 0xf {
            extra_bytes = 8;
        } else if (value & 0xe) == 0xe {
            extra_bytes = 7;
        } else if (value & 0xc) == 0xc {
            extra_bytes = 6;
        } else if (value & 0x8) == 0x8 {
            extra_bytes = 5;
        } else {
            lower_nibble = true;
            extra_bytes = 4;
            value -= 0xf0;
        }
    } else if (value & 0xe0) == 0xe0 {
        extra_bytes = 3;
        value -= 0xe0;
    } else if (value & 0xc0) == 0xc0 {
        extra_bytes = 2;
        value -= 0xc0;
    } else if (value & 0x80) == 0x80 {
        extra_bytes = 1;
        value -= 0x80;
    }

    if extra_bytes != 0 {
        let mut new_value: usize = 0;
        let mut count = 1;
        while count <= extra_bytes {
            let (remaining, value) = nom_unsigned_one_byte(input, Endian::Le)?;
            input = remaining;

            new_value += (value as usize) << ((extra_bytes - count) * 8);
            count += 1;
        }

        if lower_nibble {
            new_value = new_value + ((value as usize) << (extra_bytes * 8));
        }
        return Ok((input, new_value as usize));
    }

    Ok((input, value as usize))
}

#[cfg(test)]
mod tests {
    use crate::{
        artifacts::os::macos::spotlight::{
            dbstr::meta::get_spotlight_meta, store::property::parse_property,
        },
        filesystem::files::read_file,
    };
    use std::path::PathBuf;

    #[test]
    fn test_parse_property() {
        let mut test_location = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
        test_location.push("tests/test_data/macos/spotlight/bigsur/property.raw");

        let data = read_file(test_location.to_str().unwrap()).unwrap();
        test_location.pop();
        test_location.push("*.header");
        let meta = get_spotlight_meta(test_location.to_str().unwrap()).unwrap();

        let (_, results) = parse_property(&data, &meta).unwrap();
    }
}
