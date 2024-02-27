use crate::{
    artifacts::os::macos::spotlight::store::property::parse_variable_size,
    utils::{
        nom_helper::{nom_unsigned_four_bytes, nom_unsigned_one_byte, Endian},
        strings::extract_utf8_string,
    },
};
use common::macos::DataAttribute;
use nom::bytes::complete::{take, take_while1};
use std::collections::HashMap;

#[derive(Debug)]
pub(crate) struct DataProperties {
    pub(crate) attribute: DataAttribute,
    pub(crate) prop_type: u8,
    pub(crate) name: String,
}

/// Parse property data associated with Dbstr-1.map
pub(crate) fn parse_properties_data<'a>(
    data: &'a [u8],
    offsets: &[u32],
) -> nom::IResult<&'a [u8], HashMap<usize, DataProperties>> {
    let deleted = 1;
    let empty = 0;
    let mut props = HashMap::new();
    for (index, offset) in offsets.iter().enumerate() {
        if offset == &deleted || offset == &empty {
            continue;
        }
        let (data_start, _) = take(*offset)(data)?;
        let (input, size) = nom_unsigned_one_byte(data_start, Endian::Le)?;
        let (_, input) = take(size)(input)?;

        let (input, attribute_data) = nom_unsigned_one_byte(input, Endian::Le)?;
        let (input, prop_type) = nom_unsigned_one_byte(input, Endian::Le)?;
        let (_, string_data) = take_while1(|b| b != 0)(input)?;
        let name = extract_utf8_string(string_data);

        let prop = DataProperties {
            attribute: get_attribute(&attribute_data),
            prop_type,
            name,
        };

        props.insert(index, prop);
    }

    Ok((data, props))
}

/// Parse category data associated with Dbstr-2.map
pub(crate) fn parse_categories_data<'a>(
    data: &'a [u8],
    offsets: &[u32],
) -> nom::IResult<&'a [u8], HashMap<usize, String>> {
    let deleted = 1;
    let empty = 0;
    let mut categories = HashMap::new();
    for (index, offset) in offsets.iter().enumerate() {
        if offset == &deleted || offset == &empty {
            continue;
        }
        let (data_start, _) = take(*offset)(data)?;
        let (input, size) = nom_unsigned_one_byte(data_start, Endian::Le)?;
        let (_, input) = take(size)(input)?;

        let (_, string_data) = take_while1(|b| b != 0)(input)?;
        let name = extract_utf8_string(string_data);

        categories.insert(index, name);
    }

    Ok((data, categories))
}

/// Parse data associated with Dbstr-4.map and Dbstr-5.map
pub(crate) fn parse_dbstr_data<'a>(
    data: &'a [u8],
    offsets: &[u32],
    has_extra: &bool,
) -> nom::IResult<&'a [u8], HashMap<usize, Vec<u32>>> {
    let deleted = 1;
    let empty = 0;
    let mut values = HashMap::new();
    for (index, offset) in offsets.iter().enumerate() {
        if offset == &deleted || offset == &empty {
            continue;
        }

        let (data_start, _) = take(*offset)(data)?;
        let (mut input, mut value) = nom_unsigned_one_byte(data_start, Endian::Le)?;

        while (value & 0x80) == 0x80 {
            let (remaining_input, extra_value) = nom_unsigned_one_byte(input, Endian::Le)?;
            input = remaining_input;
            value = extra_value;
        }

        let (mut input, mut index_size) = parse_variable_size(input)?;
        if *has_extra {
            let (remaining, _) = nom_unsigned_one_byte(input, Endian::Le)?;
            input = remaining;
        }

        index_size = 4 * (index_size / 4);
        let (_input, mut array_data) = take(index_size)(input)?;
        let min_size = 4;
        let mut value_vec = Vec::new();

        while !array_data.is_empty() && array_data.len() >= min_size {
            let (input, value) = nom_unsigned_four_bytes(array_data, Endian::Le)?;
            array_data = input;

            value_vec.push(value);
        }
        values.insert(index, value_vec);
    }

    Ok((data, values))
}

/// Get property attribute type
fn get_attribute(data: &u8) -> DataAttribute {
    match data {
        0x0 => DataAttribute::AttrBool,
        0x1 => DataAttribute::AttrUnknown,
        0x2 => DataAttribute::AttrVariableSizeInt,
        0x3 => DataAttribute::AttrUnknown2,
        0x4 => DataAttribute::AttrUnknown3,
        0x5 => DataAttribute::AttrUnknown4,
        0x6 => DataAttribute::AttrVariableSizeInt2,
        0x7 => DataAttribute::AttrVariableSizeIntMultiValue,
        0x8 => DataAttribute::AttrByte,
        0x9 => DataAttribute::AttrFloat32,
        0xa => DataAttribute::AttrFloat64,
        0xb => DataAttribute::AttrString,
        0xc => DataAttribute::AttrDate,
        0xe => DataAttribute::AttrBinary,
        0xf => DataAttribute::AttrList,
        _ => DataAttribute::Unknown,
    }
}

#[cfg(test)]
mod tests {
    use super::parse_properties_data;
    use crate::{
        artifacts::os::macos::spotlight::dbstr::{
            data::{get_attribute, parse_categories_data, parse_dbstr_data, DataAttribute},
            header::get_header,
            offsets::get_offsets,
        },
        filesystem::{files::read_file, metadata::glob_paths},
    };
    use std::path::PathBuf;

    #[test]
    fn test_parse_properties_data() {
        let mut test_location = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
        test_location.push("tests/test_data/macos/spotlight/bigsur/*.header");
        let headers = glob_paths(test_location.to_str().unwrap()).unwrap();
        for header in headers {
            // Only 1 contains the property data
            if !header.full_path.contains("1") {
                continue;
            }

            let data = read_file(&header.full_path).unwrap();
            let db_header = get_header(&data).unwrap();
            let offsets = header.full_path.replace("header", "offsets");
            let offset_data = read_file(&offsets).unwrap();
            let offsets_vec = get_offsets(&offset_data, &db_header.offset_entries).unwrap();

            let data = header.full_path.replace("header", "data");
            let prop_data = read_file(&data).unwrap();

            let (_, results) = parse_properties_data(&prop_data, &offsets_vec).unwrap();
            assert_eq!(results.len(), 109);
        }
    }

    #[test]
    fn test_get_attribute() {
        let test = 0xa;
        let attribute = get_attribute(&test);
        assert_eq!(attribute, DataAttribute::AttrFloat64);
    }

    #[test]
    fn test_parse_categories_data() {
        let mut test_location = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
        test_location.push("tests/test_data/macos/spotlight/bigsur/*.header");
        let headers = glob_paths(test_location.to_str().unwrap()).unwrap();
        for header in headers {
            // Only 2 contains the category data
            if !header.full_path.contains("2") {
                continue;
            }

            let data = read_file(&header.full_path).unwrap();
            let db_header = get_header(&data).unwrap();
            let offsets = header.full_path.replace("header", "offsets");
            let offset_data = read_file(&offsets).unwrap();
            let offsets_vec = get_offsets(&offset_data, &db_header.offset_entries).unwrap();

            let data = header.full_path.replace("header", "data");
            let prop_data = read_file(&data).unwrap();

            let (_, results) = parse_categories_data(&prop_data, &offsets_vec).unwrap();
            assert_eq!(results.len(), 4708);
        }
    }

    #[test]
    fn test_parse_dbstr_data() {
        let mut test_location = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
        test_location.push("tests/test_data/macos/spotlight/bigsur/*.header");
        let headers = glob_paths(test_location.to_str().unwrap()).unwrap();
        for header in headers {
            if !header.full_path.contains("5") && !header.full_path.contains("4") {
                continue;
            }

            let data = read_file(&header.full_path).unwrap();
            let db_header = get_header(&data).unwrap();
            let offsets = header.full_path.replace("header", "offsets");
            let offset_data = read_file(&offsets).unwrap();
            let offsets_vec = get_offsets(&offset_data, &db_header.offset_entries).unwrap();

            let data = header.full_path.replace("header", "data");
            let prop_data = read_file(&data).unwrap();

            let mut extra = false;
            if header.full_path.contains("5") {
                extra = true;
            }

            let (_, results) = parse_dbstr_data(&prop_data, &offsets_vec, &extra).unwrap();
            if header.full_path.contains("5") {
                assert_eq!(results.len(), 126);
                assert_eq!(results.get(&3).unwrap().len(), 41);
            }

            if header.full_path.contains("4") {
                assert_eq!(results.len(), 312);
                assert_eq!(results.get(&3).unwrap().len(), 1);
            }
        }
    }
}
