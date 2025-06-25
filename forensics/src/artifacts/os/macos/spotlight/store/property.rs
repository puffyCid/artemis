use crate::{
    artifacts::os::macos::spotlight::{
        dbstr::meta::SpotlightMeta,
        store::properties::{
            binary::extract_binary,
            byte::{extract_bool, extract_byte},
            date::extract_dates,
            float::{extract_float32, extract_float64},
            list::extract_list,
            multivalue::extract_multivalue,
            string::extract_string,
        },
    },
    utils::{
        compression::decompress::decompress_lz4,
        nom_helper::{Endian, nom_unsigned_four_bytes, nom_unsigned_one_byte},
        time::unixepoch_to_iso,
    },
};
use common::macos::{DataAttribute, SpotlightEntries, SpotlightValue};
use log::{error, warn};
use nom::{bytes::complete::take, error::ErrorKind};
use serde_json::{Value, json};
use std::collections::HashMap;

/// Parse and extract all properties associated with Spotlight data
pub(crate) fn parse_property<'a>(
    data: &'a [u8],
    meta: &SpotlightMeta,
    uncompressed_size: &u32,
    dir: &str,
) -> nom::IResult<&'a [u8], Vec<SpotlightEntries>> {
    let mut compressed_input = data;
    let mut decom_data = Vec::new();
    loop {
        let (input, compress_sig) = nom_unsigned_four_bytes(compressed_input, Endian::Le)?;
        let lz4_sig = 0x31347662;
        // Sometimes spotlight data is already decompressed
        let decom_sig = 0x2d347662;

        if compress_sig == decom_sig {
            let (input, decompress_size) = nom_unsigned_four_bytes(input, Endian::Le)?;
            let (input, decom) = take(decompress_size)(input)?;
            decom_data.append(&mut decom.to_vec());
            let (_, check_sig) = nom_unsigned_four_bytes(input, Endian::Le)?;
            // There may be more compressed data.
            if check_sig != lz4_sig && check_sig != decom_sig {
                break;
            }
            compressed_input = input;
            continue;
        }

        if compress_sig != lz4_sig {
            error!("[spotlight] Did not get LZ4 compression signature");
            break;
        }

        let (input, decompress_size) = nom_unsigned_four_bytes(input, Endian::Le)?;
        let (input, compress_size) = nom_unsigned_four_bytes(input, Endian::Le)?;
        let (input, compressed_data) = take(compress_size)(input)?;

        let decompress_result =
            decompress_lz4(compressed_data, decompress_size as usize, &decom_data);
        let mut decom = match decompress_result {
            Ok(result) => result,
            Err(err) => {
                error!("[spotlight] Failed to decompress lz4 compression: {err:?}");
                return Ok((input, Vec::new()));
            }
        };
        decom_data.append(&mut decom);

        let (_, check_sig) = nom_unsigned_four_bytes(input, Endian::Le)?;
        // There may be more compressed data.
        if check_sig != lz4_sig && check_sig != decom_sig {
            break;
        }

        compressed_input = input;
    }

    if decom_data.len() != (uncompressed_size - 20) as usize {
        warn!(
            "[spotlight] Decompressed size ({}) did not matched expected size: {}. Parsing will likely fail or be incopmlete",
            decom_data.len(),
            (uncompressed_size - 20)
        );
    }

    let entries_result = parse_all_records(&decom_data, meta, dir);
    let entries = match entries_result {
        Ok((_, result)) => result,
        Err(_err) => {
            error!("[spotlight] Failed to parse all spotlight entries");
            Vec::new()
        }
    };

    Ok((&[], entries))
}

pub(crate) struct PropertyHeader {
    _sig: u32,
    pub(crate) page_size: u32,
    _used_size: u32,
    _property_types: Vec<PropertyType>,
    pub(crate) uncompressed_size: u32,
}

/// Extract property header info
pub(crate) fn property_header(data: &[u8]) -> nom::IResult<&[u8], PropertyHeader> {
    let (input, sig) = nom_unsigned_four_bytes(data, Endian::Le)?;
    let (input, page_size) = nom_unsigned_four_bytes(input, Endian::Le)?;
    let (input, used_size) = nom_unsigned_four_bytes(input, Endian::Le)?;

    // Also referred to as block_type
    let (input, property_type_data) = nom_unsigned_four_bytes(input, Endian::Le)?;
    let property_types = get_property_types(property_type_data);

    if !property_types.contains(&PropertyType::Lz4Compressed) {
        warn!("[spotlight] Got non-lz4 compressed data. This is unsupported!");
        return Err(nom::Err::Failure(nom::error::Error::new(
            &[],
            ErrorKind::Fail,
        )));
    }

    // Total uncompressed size (including the 16 previous bytes above)
    // If the value is zero the page/property is not compressed!
    let (compressed_input, uncompressed_size) = nom_unsigned_four_bytes(input, Endian::Le)?;

    let header = PropertyHeader {
        _sig: sig,
        page_size,
        _used_size: used_size,
        _property_types: property_types,
        uncompressed_size,
    };

    Ok((compressed_input, header))
}

/// Extract all of records associated with the property
fn parse_all_records<'a>(
    data: &'a [u8],
    meta: &SpotlightMeta,
    dir: &str,
) -> nom::IResult<&'a [u8], Vec<SpotlightEntries>> {
    let mut decom_data = data;
    let min_size = 4;
    let mut entries = Vec::new();
    while decom_data.len() > min_size {
        let (input, record_size) = nom_unsigned_four_bytes(decom_data, Endian::Le)?;
        let (input, record_data) = take(record_size)(input)?;

        decom_data = input;

        let entry_result = parse_record(record_data, meta, dir);
        let entry = match entry_result {
            Ok((_, result)) => result,
            Err(_err) => {
                error!("[spotlight] Failed to parse spotlight entry");
                continue;
            }
        };
        entries.push(entry);
    }

    Ok((decom_data, entries))
}

/// Parse each individual record
fn parse_record<'a>(
    data: &'a [u8],
    meta: &SpotlightMeta,
    dir: &str,
) -> nom::IResult<&'a [u8], SpotlightEntries> {
    let (input, inode) = parse_variable_size(data)?;

    let (input, flags) = nom_unsigned_one_byte(input, Endian::Le)?;
    let (input, store_id) = parse_variable_size(input)?;
    let (input, parent_inode) = parse_variable_size(input)?;

    let (mut remaining_input, last_updated) = parse_variable_size(input)?;
    let mut meta_prop_index = 0;

    let mut values = HashMap::new();
    while !remaining_input.is_empty() {
        let (input, index) = parse_variable_size(remaining_input)?;

        meta_prop_index += index;
        if meta_prop_index > meta.props.len() {
            break;
        }
        let prop_opt = meta.props.get(&meta_prop_index);
        let props = if let Some(result) = prop_opt {
            result
        } else {
            error!("[spotlight] No properties for {index}");
            break;
        };

        let mut spot_value = SpotlightValue {
            attribute: props.attribute.clone(),
            value: Value::Null,
        };

        if spot_value.attribute == DataAttribute::AttrVariableSizeIntMultiValue {
            let (input, multi_values) = extract_multivalue(input, &props.prop_type)?;
            spot_value.value = multi_values;

            values.insert(props.name.clone(), spot_value);
            remaining_input = input;
            continue;
        }

        if spot_value.attribute == DataAttribute::AttrFloat32 {
            let (input, floats) = extract_float32(input, &props.prop_type)?;
            spot_value.value = floats;

            values.insert(props.name.clone(), spot_value);
            remaining_input = input;
            continue;
        }

        if spot_value.attribute == DataAttribute::AttrFloat64 {
            let (input, floats) = extract_float64(input, &props.prop_type)?;
            spot_value.value = floats;

            values.insert(props.name.clone(), spot_value);
            remaining_input = input;
            continue;
        }

        if spot_value.attribute == DataAttribute::AttrDate {
            let (input, dates) = extract_dates(input, &props.prop_type)?;
            spot_value.value = dates;

            values.insert(props.name.clone(), spot_value);
            remaining_input = input;
            continue;
        }

        if spot_value.attribute == DataAttribute::AttrByte
            && props.name != "kMDStoreAccumulatedSizes"
        {
            let (input, value) = extract_byte(input)?;
            spot_value.value = value;

            values.insert(props.name.clone(), spot_value);
            remaining_input = input;
            continue;
        }

        if spot_value.attribute == DataAttribute::AttrBool {
            let (input, bool) = extract_bool(input)?;
            spot_value.value = bool;

            values.insert(props.name.clone(), spot_value);
            remaining_input = input;
            continue;
        }

        let (var_input, prop_value_size) = parse_variable_size(input)?;

        if spot_value.attribute == DataAttribute::AttrString {
            let (input, string) = extract_string(var_input, &prop_value_size)?;
            spot_value.value = string;
            values.insert(props.name.clone(), spot_value);
            remaining_input = input;
            continue;
        }

        if spot_value.attribute == DataAttribute::AttrBinary {
            let (input, binary) = extract_binary(var_input, &prop_value_size, &props.name)?;
            spot_value.value = binary;
            values.insert(props.name.clone(), spot_value);
            remaining_input = input;
            continue;
        }

        // Get lists associated with Attribute
        if spot_value.attribute == DataAttribute::AttrList {
            spot_value.value = extract_list(
                &meta.categories,
                &meta.indexes1,
                &meta.indexes2,
                &prop_value_size,
                &props.prop_type,
            );

            values.insert(props.name.clone(), spot_value);
            remaining_input = var_input;

            continue;
        }

        // All other property attributes are variable data
        spot_value.value = json!(prop_value_size);
        values.insert(props.name.clone(), spot_value);
        remaining_input = var_input;
    }

    let entry = SpotlightEntries {
        inode,
        parent_inode,
        flags,
        store_id,
        last_updated: unixepoch_to_iso(last_updated as i64),
        values,
        directory: dir.to_string(),
    };

    Ok((data, entry))
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

/// Determine Property type
fn get_property_types(data: u32) -> Vec<PropertyType> {
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

/// Extract variable sized property data
pub(crate) fn parse_variable_size(data: &[u8]) -> nom::IResult<&[u8], usize> {
    let (mut input, mut value) = nom_unsigned_one_byte(data, Endian::Le)?;
    let mut lower_nibble = true;
    let mut extra_bytes: usize = 0;

    if value == 0 {
        return Ok((input, value as usize));
    }

    if (value & 0xf0) == 0xf0 {
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
            new_value += (value as usize) << (extra_bytes * 8);
        }
        return Ok((input, new_value));
    }

    Ok((input, value as usize))
}

#[cfg(test)]
mod tests {
    use super::{get_property_types, parse_variable_size, property_header};
    use crate::{
        artifacts::os::macos::spotlight::{
            dbstr::meta::get_spotlight_meta,
            store::property::{PropertyType, parse_all_records, parse_property, parse_record},
        },
        filesystem::{files::read_file, metadata::glob_paths},
    };
    use std::path::PathBuf;

    #[test]
    fn test_parse_property() {
        let mut test_location = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
        test_location.push("tests/test_data/macos/spotlight/bigsur/property.raw");

        let data = read_file(test_location.to_str().unwrap()).unwrap();
        test_location.pop();
        test_location.push("*.header");
        let paths = glob_paths(test_location.to_str().unwrap()).unwrap();

        let meta = get_spotlight_meta(&paths).unwrap();
        let (input, header) = property_header(&data).unwrap();

        let (_, results) = parse_property(input, &meta, &header.uncompressed_size, "test").unwrap();
        assert_eq!(results.len(), 195);
    }

    #[test]
    fn test_parse_all_records() {
        let mut test_location = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
        test_location.push("tests/test_data/macos/spotlight/bigsur/decom.raw");

        let data = read_file(test_location.to_str().unwrap()).unwrap();
        test_location.pop();
        test_location.push("*.header");
        let paths = glob_paths(test_location.to_str().unwrap()).unwrap();

        let meta = get_spotlight_meta(&paths).unwrap();

        let (_, results) = parse_all_records(&data, &meta, "test").unwrap();
        assert_eq!(results.len(), 195);
    }

    #[test]
    fn test_parse_record() {
        let mut test_location = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
        test_location.push("*.header");

        let paths = glob_paths(test_location.to_str().unwrap()).unwrap();
        let meta = get_spotlight_meta(&paths).unwrap();

        let data = [
            243, 0, 0, 19, 144, 32, 128, 255, 243, 0, 0, 19, 143, 254, 5, 213, 170, 86, 232, 77,
            92, 7, 2, 1, 3, 2, 35, 1, 9, 4, 0, 2, 15, 95, 67, 111, 100, 101, 83, 105, 103, 110, 97,
            116, 117, 114, 101, 0, 18, 7, 2, 0, 0, 0, 192, 203, 201, 195, 65, 2, 0, 1, 0, 1, 4, 3,
            0, 0, 0, 64, 12, 222, 193, 65, 1, 0, 0, 0, 128, 68, 222, 193, 65, 1, 6, 191, 93, 88,
            225, 201, 195, 65, 1, 0, 1, 0, 3, 0, 1, 0, 1, 0, 0, 0, 128, 68, 222, 193, 65, 2, 0, 1,
            0, 0, 0, 64, 12, 222, 193, 65, 2, 0, 0, 0, 128, 68, 222, 193, 65, 2, 0, 0, 0, 128, 68,
            222, 193, 65, 1, 0, 0, 0, 128, 68, 222, 193, 65, 1, 0, 0, 0, 64, 12, 222, 193, 65, 4,
            17, 95, 67, 111, 100, 101, 83, 105, 103, 110, 97, 116, 117, 114, 101, 22, 2, 0, 1, 17,
            95, 67, 111, 100, 101, 83, 105, 103, 110, 97, 116, 117, 114, 101, 22, 2, 0,
        ];

        let (_, results) = parse_record(&data, &meta, "test").unwrap();
        assert_eq!(results.inode, 12884906896);
    }

    #[test]
    fn test_property_header() {
        let mut test_location = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
        test_location.push("tests/test_data/macos/spotlight/bigsur/property.raw");

        let data = read_file(test_location.to_str().unwrap()).unwrap();
        let (_, header) = property_header(&data).unwrap();

        assert_eq!(header.uncompressed_size, 67070);
        assert_eq!(header.page_size, 16384);
    }

    #[test]
    fn test_get_property_types() {
        let data = 9;
        let result = get_property_types(data);
        assert_eq!(result[0], PropertyType::ZlibDeflateRecords);
    }

    #[test]
    fn test_parse_variable_size() {
        let data = [1, 0, 0, 0, 0];
        let (_, result) = parse_variable_size(&data).unwrap();
        assert_eq!(result, 1);
    }
}
