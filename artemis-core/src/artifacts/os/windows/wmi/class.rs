use super::{
    index::IndexBody,
    instance::{InstanceRecord, InstanceValue},
};
use crate::{
    artifacts::os::windows::wmi::{namespaces::extract_namespace_data, wmi::hash_name},
    utils::{
        nom_helper::{
            nom_signed_eight_bytes, nom_signed_four_bytes, nom_signed_two_bytes,
            nom_unsigned_eight_bytes, nom_unsigned_four_bytes, nom_unsigned_one_byte,
            nom_unsigned_two_bytes, Endian,
        },
        strings::extract_utf8_string,
    },
};
use log::warn;
use nom::{
    bytes::complete::{take, take_while},
    error::ErrorKind,
    number::complete::{le_f32, le_f64, le_i8},
};
use serde_json::{json, Value};
use std::{collections::HashMap, mem::size_of};

#[derive(Debug, Clone)]
pub(crate) struct ClassInfo {
    pub(crate) super_class_name: String,
    pub(crate) class_name: String,
    pub(crate) qualifiers: Vec<Qualifier>,
    pub(crate) properties: Vec<Property>,
    pub(crate) instances: Vec<InstanceRecord>,
    pub(crate) class_hash: String,
    pub(crate) includes_parent_props: bool,
}

#[derive(Debug, Clone)]
pub(crate) struct Qualifier {
    pub(crate) name: String,
    pub(crate) value_data_type: CimType,
    pub(crate) data: Value,
}

#[derive(Debug, Clone)]
pub(crate) struct Property {
    pub(crate) name: String,
    pub(crate) property_data_type: CimType,
    pub(crate) property_index: u16,
    pub(crate) data_offset: u32,
    class_level: u32,
    qualifiers: Vec<Qualifier>,
    pub(crate) instance_value: InstanceValue,
}

#[derive(Debug, PartialEq, Clone)]
pub(crate) enum CimType {
    Sint16,
    Sint32,
    Real32,
    Real64,
    String,
    Bool,
    Object,
    Sint8,
    Uint8,
    Uint16,
    Uint32,
    Sint64,
    Uint64,
    Datetime,
    Reference,
    Char,
    ArrayString,
    ArraySint16,
    ArraySint8,
    ArraySint32,
    ArraySint64,
    ArrayReal32,
    ArrayReal64,
    ArrayBool,
    ArrayUint8,
    ArrayUint16,
    ArrayUint32,
    ArrayUint64,
    ArrayChar,
    ArrayObject,
    ByteRefString,
    ByteRefUint32,
    ByteRefUint16,
    ByteRefSint64,
    ByteRefSint32,
    ByteRefBool,
    ByteRefDatetime,
    ByteRefUint8,
    ByteRefReference,
    ByteRefObject,
    Unknown,
    None,
}

/// Parse class definition information
pub(crate) fn parse_class<'a>(data: &'a [u8], hash: &str) -> nom::IResult<&'a [u8], ClassInfo> {
    let (input, _unknown) = nom_unsigned_one_byte(data, Endian::Le)?;
    let (input, class_name_offset) = nom_unsigned_four_bytes(input, Endian::Le)?;
    let (input, default_size) = nom_unsigned_four_bytes(input, Endian::Le)?;
    let (input, super_class_name_size) = nom_unsigned_four_bytes(input, Endian::Le)?;

    let adjust_size = 4;
    // super class name size also includes the size itself. We already nom'd that away
    let (input, super_class_name_data) = take(super_class_name_size - adjust_size)(input)?;

    let (input, qual_data_size) = nom_unsigned_four_bytes(input, Endian::Le)?;
    let (input, qual_data) = take(qual_data_size - adjust_size)(input)?;
    let (input, number_props) = nom_unsigned_four_bytes(input, Endian::Le)?;

    let prop_size = 8;
    let (input, prop_data) = take(number_props * prop_size)(input)?;
    let (input, _default_values) = take(default_size)(input)?;

    let (input, prop_value_size) = nom_unsigned_four_bytes(input, Endian::Le)?;
    let adjust_msb: u32 = 0x80000000;

    if prop_value_size < adjust_msb {
        warn!("[wmi] Property value size is too small. Expected MSB to bet set. Got instead: {prop_value_size}");
        return Err(nom::Err::Failure(nom::error::Error::new(
            input,
            ErrorKind::Fail,
        )));
    }
    // Most significant bit is always set
    let (input, prop_value_data) = take(prop_value_size - adjust_msb)(input)?;

    let (_, class_name) = get_class_name(class_name_offset, prop_value_data)?;

    // If the Super Class name size is 4 then there is no name because the size value also includes the 4 byte size value itself.
    let super_class_name = if !super_class_name_data.is_empty() {
        let (_, name) = extract_cim_string(super_class_name_data)?;
        name
    } else {
        String::new()
    };

    let (_, qualifiers) = parse_qualifier(qual_data, prop_value_data)?;
    let (_, properties) = parse_property(prop_data, prop_value_data)?;

    let class_info = ClassInfo {
        super_class_name,
        class_name,
        qualifiers,
        properties,
        instances: Vec::new(),
        class_hash: hash.to_string(),
        includes_parent_props: false,
    };

    Ok((input, class_info))
}

/// Extract the Class name
fn get_class_name(offset: u32, data: &[u8]) -> nom::IResult<&[u8], String> {
    let (mut start_data, _) = take(offset)(data)?;
    if offset == 0 {
        start_data = data;
    }
    let (input, name) = extract_cim_string(start_data)?;

    Ok((input, name))
}

/// Parse the Qualifier data in the class
pub(crate) fn parse_qualifier<'a>(
    data: &'a [u8],
    value_data: &'a [u8],
) -> nom::IResult<&'a [u8], Vec<Qualifier>> {
    let min_size = 13;
    let mut qual_data = data;

    let mut quals = Vec::new();
    while qual_data.len() >= min_size {
        let (input, name_offset) = nom_unsigned_four_bytes(qual_data, Endian::Le)?;
        let (input, _unknown) = nom_unsigned_one_byte(input, Endian::Le)?;
        let (input, cim_data_type) = nom_unsigned_four_bytes(input, Endian::Le)?;

        let msb_set: u32 = 0x80000000;
        let name = if name_offset > msb_set {
            let index = name_offset - msb_set;
            get_predefine_name(&index)
        } else {
            let (_, value) = get_class_name(name_offset, value_data)?;
            value
        };

        let mut qual = Qualifier {
            name,
            value_data_type: get_cim_data_type(&cim_data_type),
            data: Value::Null,
        };

        let (input, value) = extract_cim_data(&qual.value_data_type, input, value_data)?;
        qual.data = value;

        qual_data = input;
        quals.push(qual);
    }

    Ok((qual_data, quals))
}

/// Parse the Property data in the class
fn parse_property<'a>(
    data: &'a [u8],
    value_data: &'a [u8],
) -> nom::IResult<&'a [u8], Vec<Property>> {
    let min_size = 8;
    let mut prop_data = data;

    let mut props = Vec::new();
    while prop_data.len() >= min_size {
        let (input, name_offset) = nom_unsigned_four_bytes(prop_data, Endian::Le)?;
        let msb_set: u32 = 0x80000000;
        let name = if name_offset > msb_set {
            let index = name_offset - msb_set;
            get_predefine_name(&index)
        } else {
            let (_, value) = get_class_name(name_offset, value_data)?;
            value
        };

        let (input, prop_definition_offset) = nom_unsigned_four_bytes(input, Endian::Le)?;
        prop_data = input;

        let (input, _) = take(prop_definition_offset)(value_data)?;

        let (input, prop_data_type) = nom_unsigned_four_bytes(input, Endian::Le)?;
        let (input, property_index) = nom_unsigned_two_bytes(input, Endian::Le)?;
        let (input, data_offset) = nom_unsigned_four_bytes(input, Endian::Le)?;
        let (input, class_level) = nom_unsigned_four_bytes(input, Endian::Le)?;
        let (input, qual_size) = nom_unsigned_four_bytes(input, Endian::Le)?;

        let adjust_size = 4;
        let qualifiers = if qual_size > adjust_size {
            let (_, qual_data) = take(qual_size - adjust_size)(input)?;
            let (_, qualifiers) = parse_qualifier(qual_data, value_data)?;
            qualifiers
        } else {
            Vec::new()
        };

        let prop = Property {
            name,
            property_data_type: get_cim_data_type(&prop_data_type),
            qualifiers,
            property_index,
            data_offset,
            class_level,
            instance_value: InstanceValue::Unknown,
        };
        props.push(prop);
    }

    Ok((prop_data, props))
}

/// Extract a CIM string from raw bytes
fn extract_cim_string(data: &[u8]) -> nom::IResult<&[u8], String> {
    let (input, string_type) = nom_unsigned_one_byte(data, Endian::Le)?;

    if string_type != 0 {
        warn!("[wmi] CIM String is using unknown encoding/raw bytes. Cannot extract.");
        return Ok((input, String::new()));
    }

    // CIM strings are ASCII with end of string character
    let (input, string_data) = take_while(|b| b != 0)(input)?;
    let value = extract_utf8_string(string_data);

    Ok((input, value))
}

/// Get predefine string names
fn get_predefine_name(index: &u32) -> String {
    match index {
        1 => String::from("key"),
        3 => String::from("read"),
        4 => String::from("write"),
        5 => String::from("volatile"),
        6 => String::from("provider"),
        7 => String::from("dynamic"),
        10 => String::from("type"),
        _ => String::from("unknown"),
    }
}

/// Determine the CIM Type
fn get_cim_data_type(data_type: &u32) -> CimType {
    match data_type {
        0x0 => CimType::None,
        0x2 => CimType::Sint16,
        0x3 => CimType::Sint32,
        0x4 => CimType::Real32,
        0x5 => CimType::Real64,
        0x8 => CimType::String,
        0xb => CimType::Bool,
        0xd => CimType::Object,
        0x10 => CimType::Sint8,
        0x11 => CimType::Uint8,
        0x12 => CimType::Uint16,
        0x13 => CimType::Uint32,
        0x14 => CimType::Sint64,
        0x15 => CimType::Uint64,
        0x65 => CimType::Datetime,
        0x66 => CimType::Reference,
        0x67 => CimType::Char,
        0x2008 => CimType::ArrayString,
        0x2002 => CimType::ArraySint16,
        0x2003 => CimType::ArraySint32,
        0x2004 => CimType::ArrayReal32,
        0x2005 => CimType::ArrayReal64,
        0x200b => CimType::ArrayBool,
        0x2010 => CimType::ArraySint8,
        0x2011 => CimType::ArrayUint8,
        0x2012 => CimType::ArrayUint16,
        0x2013 => CimType::ArrayUint32,
        0x2014 => CimType::ArraySint64,
        0x2015 => CimType::ArrayUint64,
        0x2067 => CimType::ArrayChar,
        0x4003 => CimType::ByteRefSint32,
        0x4008 => CimType::ByteRefString,
        0x4012 => CimType::ByteRefUint16,
        0x4013 => CimType::ByteRefUint32,
        0x4015 => CimType::ByteRefSint64,
        0x400b => CimType::ByteRefBool,
        0x400d => CimType::ByteRefObject,
        0x4065 => CimType::ByteRefDatetime,
        0x4066 => CimType::ByteRefReference,
        0x4011 => CimType::ByteRefUint8,
        0x200d => CimType::ArrayObject,
        // Seen but unsure what they are
        0x6008 | 0x600d | 0x6012 => CimType::Unknown,
        _ => {
            println!("unknown cim type: {data_type}");
            CimType::Unknown
        }
    }
}

/// Extract CIM data based on the type
pub(crate) fn extract_cim_data<'a>(
    cim_type: &CimType,
    remaining_input: &'a [u8],
    data: &'a [u8],
) -> nom::IResult<&'a [u8], Value> {
    let cim_value;
    let remaining;
    match cim_type {
        CimType::String | CimType::Reference | CimType::Datetime => {
            let (input, qual_data_offset) = nom_unsigned_four_bytes(remaining_input, Endian::Le)?;
            let (_, value) = get_class_name(qual_data_offset, data)?;
            cim_value = Value::String(value);
            remaining = input;
        }
        CimType::Unknown => {
            println!("unknown. what??? see libyal");
            cim_value = Value::Null;
            remaining = &[];
        }
        CimType::ArrayString | CimType::ByteRefReference => {
            // Array value is offset to array length
            let (input, string_count_offset) =
                nom_unsigned_four_bytes(remaining_input, Endian::Le)?;
            remaining = input;

            // Offset from the property value data
            let (input, _) = take(string_count_offset)(data)?;
            let (mut string_data, count) = nom_unsigned_four_bytes(input, Endian::Le)?;

            if count as usize > data.len() {
                cim_value = Value::Null;
                return Ok((remaining, cim_value));
            }

            let mut strings = Vec::new();
            let mut string_count = 0;
            // Parse array of strings
            while string_count < count && string_data.len() > 4 {
                let (next_offset, offset) = nom_unsigned_four_bytes(string_data, Endian::Le)?;
                let (_, string_value) = get_class_name(offset, data)?;

                string_count += 1;
                strings.push(string_value);
                string_data = next_offset;
            }
            cim_value = json!(strings);
        }
        CimType::Object => {
            panic!("object. see CIM object libyal");
        }
        CimType::Bool => {
            let (input, value) = nom_signed_two_bytes(remaining_input, Endian::Le)?;
            cim_value = if value == -1 {
                Value::Bool(true)
            } else {
                Value::Bool(false)
            };
            remaining = input;
        }
        CimType::Sint16 => {
            let (input, value) = nom_signed_two_bytes(remaining_input, Endian::Le)?;
            cim_value = Value::Number(value.into());
            remaining = input;
        }
        CimType::Sint32 => {
            let (input, value) = nom_signed_four_bytes(remaining_input, Endian::Le)?;
            cim_value = Value::Number(value.into());
            remaining = input;
        }
        CimType::Real32 => {
            let (input, value_data) = take(size_of::<u32>())(remaining_input)?;
            let (_, value) = le_f32(value_data)?;
            cim_value = Value::String(value.to_string());
            remaining = input;
        }
        CimType::Real64 => {
            let (input, value_data) = take(size_of::<u64>())(remaining_input)?;
            let (_, value) = le_f64(value_data)?;
            cim_value = Value::String(value.to_string());
            remaining = input;
        }
        CimType::Uint8 => {
            let (input, value) = nom_unsigned_one_byte(remaining_input, Endian::Le)?;
            cim_value = Value::Number(value.into());
            remaining = input;
        }
        CimType::Sint64 => {
            let (input, value) = nom_signed_eight_bytes(remaining_input, Endian::Le)?;
            cim_value = Value::Number(value.into());
            remaining = input;
        }
        CimType::Sint8 => {
            let (input, value_data) = take(size_of::<u8>())(remaining_input)?;
            let (_, value) = le_i8(value_data)?;
            cim_value = Value::Number(value.into());
            remaining = input;
        }
        CimType::Uint64 => {
            let (input, value) = nom_unsigned_eight_bytes(remaining_input, Endian::Le)?;
            cim_value = Value::Number(value.into());
            remaining = input;
        }
        CimType::Uint16 => {
            let (input, value) = nom_unsigned_two_bytes(remaining_input, Endian::Le)?;
            cim_value = Value::Number(value.into());
            remaining = input;
        }
        CimType::Char => {
            let (input, value) = nom_unsigned_two_bytes(remaining_input, Endian::Le)?;
            let result = char::from_digit(value as u32, 10).unwrap_or_default();
            cim_value = Value::String(result.to_string());
            remaining = input;
        }
        CimType::ArrayChar => {
            // Array value is offset to array length
            let (input, string_count_offset) =
                nom_unsigned_four_bytes(remaining_input, Endian::Le)?;
            remaining = input;

            // Offset from the property value data
            let (input, _) = take(string_count_offset)(data)?;
            let (mut string_data, count) = nom_unsigned_four_bytes(input, Endian::Le)?;

            let mut strings = Vec::new();
            let mut string_count = 0;
            // Parse array of chars
            while string_count < count {
                let (next_offset, value) = nom_unsigned_two_bytes(string_data, Endian::Le)?;
                let string_value = char::from_digit(value as u32, 10)
                    .unwrap_or_default()
                    .to_string();

                string_count += 1;
                strings.push(string_value);
                string_data = next_offset;
            }
            cim_value = json!(strings);
        }
        CimType::ArrayObject => {
            // Array value is offset to array length
            let (input, string_count_offset) =
                nom_unsigned_four_bytes(remaining_input, Endian::Le)?;
            remaining = input;

            // Offset from the property value data
            let (input, _) = take(string_count_offset)(data)?;
            let (mut object_data, count) = nom_unsigned_four_bytes(input, Endian::Le)?;
            panic!("{object_data:?}");

            let mut objects: Vec<Value> = Vec::new();
            let mut object_count = 0;
        }
        CimType::Uint32 => {
            let (input, value) = nom_unsigned_four_bytes(remaining_input, Endian::Le)?;
            cim_value = Value::Number(value.into());
            remaining = input;
        }
        CimType::ArraySint32 => {
            // Array value is offset to array length
            let (input, string_count_offset) =
                nom_unsigned_four_bytes(remaining_input, Endian::Le)?;
            remaining = input;

            // Offset from the property value data
            let (input, _) = take(string_count_offset)(data)?;
            let (mut cim_data, count) = nom_unsigned_four_bytes(input, Endian::Le)?;

            let mut signed_ints = Vec::new();
            let mut int_count = 0;
            // Parse array of signed integers
            while int_count < count {
                let (next_offset, value) = nom_signed_four_bytes(cim_data, Endian::Le)?;
                int_count += 1;
                signed_ints.push(value);
                cim_data = next_offset;
            }
            cim_value = json!(signed_ints);
        }
        CimType::ArrayUint8 => {
            // Array value is offset to array length
            let (input, string_count_offset) =
                nom_unsigned_four_bytes(remaining_input, Endian::Le)?;
            remaining = input;

            // Offset from the property value data
            let (input, _) = take(string_count_offset)(data)?;
            let (mut cim_data, count) = nom_unsigned_four_bytes(input, Endian::Le)?;

            let mut signed_ints = Vec::new();
            let mut int_count = 0;
            // Parse array of unsigned integers
            while int_count < count {
                let (next_offset, value) = nom_unsigned_one_byte(cim_data, Endian::Le)?;
                int_count += 1;
                signed_ints.push(value);
                cim_data = next_offset;
            }
            cim_value = json!(signed_ints);
        }
        _ => {
            println!("{remaining_input:?}");
            panic!("odd: {cim_type:?}");
            warn!("[wmi] Unknown CIM Type: {cim_type:?}");
            let (input, _) = nom_unsigned_four_bytes(remaining_input, Endian::Le)?;
            remaining = input;
            cim_value = Value::Null;
            return Ok((remaining, cim_value));
        }
    }

    Ok((remaining, cim_value))
}

/// Get a namespace containing the specified classname. The classname should be SHA256 hashed without "CD_" prefix
pub(crate) fn get_namespace_from_class(
    classname: &str,
    index_info: &HashMap<u32, IndexBody>,
) -> Vec<String> {
    let mut namespace_info = Vec::new();
    for entry in index_info.values() {
        for value in &entry.value_data {
            if value.contains(&format!("CD_{classname}").to_uppercase())
                || value.contains(&format!("CI_{classname}").to_uppercase())
            {
                namespace_info.append(&mut entry.value_data.clone());
                //namespace_info.push(value.clone());
            }
        }
    }
    namespace_info
}

pub(crate) fn get_parent_props<'a>(
    class_name: &str,
    indexes: &HashMap<u32, IndexBody>,
    objects: &[u8],
    pages: &[u32],
    cache_props: &mut HashMap<String, Vec<Property>>
) -> Vec<Property> {
    println!("getting parent props for: {class_name}");
    let hash = hash_name(class_name);
    let namespace = get_namespace_from_class(&hash, indexes);
    let namespace_data = extract_namespace_data(&vec![namespace], objects, pages, indexes, cache_props);
    let mut parent_props = Vec::new();
    // Now loop and get parent props extracted from namespace
    for entry in namespace_data {
        for classes_vec in entry.classes {
            for classes in classes_vec.values() {
                for class in classes {
                    // Only want props related to our Class name
                    if class.class_name == class_name {
                        parent_props = class.properties.clone();
                        // Check if there is another super class
                        if !class.super_class_name.is_empty() && class.super_class_name != class_name {
                            println!("getting more parents: {}", class.super_class_name);
                            let mut results =
                                get_parent_props(&class.super_class_name, indexes, objects, pages, cache_props);
                            parent_props.append(&mut results);
                            println!("total props: {}", parent_props.len());
                        }
                    }
                }
            }
        }
    }

    parent_props
}

#[cfg(test)]
mod tests {
    use super::{
        extract_cim_data, extract_cim_string, get_cim_data_type, get_class_name,
        get_predefine_name, parse_class, parse_qualifier,
    };
    use crate::{
        artifacts::os::windows::wmi::class::{parse_property, CimType},
        filesystem::files::read_file,
    };
    use serde_json::Value;
    use std::path::PathBuf;

    #[test]
    fn test_parse_class() {
        let mut test_location = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
        test_location.push("tests/test_data/windows/wmi/class_data.raw");

        let data = read_file(test_location.to_str().unwrap()).unwrap();

        let (_, results) = parse_class(&data, &"test").unwrap();
        assert_eq!(results.super_class_name, "MSFT_DOUsage");
        assert_eq!(results.class_name, "MSFT_DOUploadUsage");
        assert_eq!(results.qualifiers.len(), 4);
        assert_eq!(results.properties.len(), 3);

        assert_eq!(results.qualifiers[0].name, "Description");
        assert_eq!(results.qualifiers[0].value_data_type, CimType::String);
        assert_eq!(
            results.qualifiers[0].data,
            Value::String(String::from("25"))
        );

        assert_eq!(results.properties[1].name, "UploadRatePct");
        assert_eq!(results.properties[1].property_data_type, CimType::Uint8);
        assert_eq!(results.properties[1].property_index, 5);
        assert_eq!(results.properties[1].qualifiers.len(), 1);
    }

    #[test]
    fn test_get_class_name() {
        let data = [
            0, 77, 83, 70, 84, 95, 68, 79, 85, 112, 108, 111, 97, 100, 85, 115, 97, 103, 101, 0, 0,
            68, 101, 115, 99, 114, 105, 112, 116, 105, 111, 110, 0, 0, 50, 53, 0, 0, 68, 101, 108,
            105, 118, 101, 114, 121, 79, 112, 116, 105, 109, 105, 122, 97, 116, 105, 111, 110, 77,
            73, 80, 114, 111, 118, 0, 0, 67, 108, 97, 115, 115, 86, 101, 114, 115, 105, 111, 110,
            0, 0, 49, 46, 48, 46, 48, 0, 0, 77, 111, 110, 116, 104, 108, 121, 85, 112, 108, 111,
            97, 100, 82, 101, 115, 116, 114, 105, 99, 116, 105, 111, 110, 0, 17, 0, 0, 0, 7, 0, 30,
            0, 0, 0, 2, 0, 0, 0, 54, 0, 0, 0, 10, 0, 0, 128, 3, 8, 0, 0, 0, 180, 0, 0, 0, 187, 0,
            0, 0, 2, 8, 32, 0, 0, 197, 0, 0, 0, 222, 0, 0, 0, 2, 8, 32, 0, 0, 230, 0, 0, 0, 3, 0,
            0, 128, 2, 11, 0, 0, 0, 255, 255, 0, 117, 105, 110, 116, 56, 0, 0, 86, 97, 108, 117,
            101, 77, 97, 112, 0, 3, 0, 0, 0, 213, 0, 0, 0, 216, 0, 0, 0, 219, 0, 0, 0, 0, 48, 0, 0,
            49, 0, 0, 50, 0, 0, 86, 97, 108, 117, 101, 115, 0, 3, 0, 0, 0, 246, 0, 0, 0, 250, 0, 0,
            0, 254, 0, 0, 0, 0, 50, 50, 0, 0, 50, 51, 0, 0, 50, 52, 0, 0, 85, 112, 108, 111, 97,
            100, 82, 97, 116, 101, 80, 99, 116, 0, 17, 0, 0, 0, 5, 0, 25, 0, 0, 0, 2, 0, 0, 0, 28,
            0, 0, 0, 10, 0, 0, 128, 3, 8, 0, 0, 0, 59, 1, 0, 0, 3, 0, 0, 128, 2, 11, 0, 0, 0, 255,
            255, 0, 117, 105, 110, 116, 56, 0, 0, 85, 112, 108, 111, 97, 100, 115, 0, 19, 0, 0, 0,
            6, 0, 26, 0, 0, 0, 2, 0, 0, 0, 28, 0, 0, 0, 10, 0, 0, 128, 3, 8, 0, 0, 0, 117, 1, 0, 0,
            3, 0, 0, 128, 2, 11, 0, 0, 0, 255, 255, 0, 117, 105, 110, 116, 51, 50, 0,
        ];

        let offset = 0;
        let (_, value) = get_class_name(offset, &data).unwrap();
        assert_eq!(value, "MSFT_DOUploadUsage")
    }

    #[test]
    fn test_get_qualifier() {
        let data = [
            20, 0, 0, 0, 2, 8, 0, 0, 0, 33, 0, 0, 0, 7, 0, 0, 128, 1, 11, 0, 0, 0, 255, 255, 6, 0,
            0, 128, 1, 8, 0, 0, 0, 37, 0, 0, 0, 65, 0, 0, 0, 0, 8, 0, 0, 0, 79, 0, 0, 0,
        ];
        let value_data = [
            0, 77, 83, 70, 84, 95, 68, 79, 85, 112, 108, 111, 97, 100, 85, 115, 97, 103, 101, 0, 0,
            68, 101, 115, 99, 114, 105, 112, 116, 105, 111, 110, 0, 0, 50, 53, 0, 0, 68, 101, 108,
            105, 118, 101, 114, 121, 79, 112, 116, 105, 109, 105, 122, 97, 116, 105, 111, 110, 77,
            73, 80, 114, 111, 118, 0, 0, 67, 108, 97, 115, 115, 86, 101, 114, 115, 105, 111, 110,
            0, 0, 49, 46, 48, 46, 48, 0, 0, 77, 111, 110, 116, 104, 108, 121, 85, 112, 108, 111,
            97, 100, 82, 101, 115, 116, 114, 105, 99, 116, 105, 111, 110, 0, 17, 0, 0, 0, 7, 0, 30,
            0, 0, 0, 2, 0, 0, 0, 54, 0, 0, 0, 10, 0, 0, 128, 3, 8, 0, 0, 0, 180, 0, 0, 0, 187, 0,
            0, 0, 2, 8, 32, 0, 0, 197, 0, 0, 0, 222, 0, 0, 0, 2, 8, 32, 0, 0, 230, 0, 0, 0, 3, 0,
            0, 128, 2, 11, 0, 0, 0, 255, 255, 0, 117, 105, 110, 116, 56, 0, 0, 86, 97, 108, 117,
            101, 77, 97, 112, 0, 3, 0, 0, 0, 213, 0, 0, 0, 216, 0, 0, 0, 219, 0, 0, 0, 0, 48, 0, 0,
            49, 0, 0, 50, 0, 0, 86, 97, 108, 117, 101, 115, 0, 3, 0, 0, 0, 246, 0, 0, 0, 250, 0, 0,
            0, 254, 0, 0, 0, 0, 50, 50, 0, 0, 50, 51, 0, 0, 50, 52, 0, 0, 85, 112, 108, 111, 97,
            100, 82, 97, 116, 101, 80, 99, 116, 0, 17, 0, 0, 0, 5, 0, 25, 0, 0, 0, 2, 0, 0, 0, 28,
            0, 0, 0, 10, 0, 0, 128, 3, 8, 0, 0, 0, 59, 1, 0, 0, 3, 0, 0, 128, 2, 11, 0, 0, 0, 255,
            255, 0, 117, 105, 110, 116, 56, 0, 0, 85, 112, 108, 111, 97, 100, 115, 0, 19, 0, 0, 0,
            6, 0, 26, 0, 0, 0, 2, 0, 0, 0, 28, 0, 0, 0, 10, 0, 0, 128, 3, 8, 0, 0, 0, 117, 1, 0, 0,
            3, 0, 0, 128, 2, 11, 0, 0, 0, 255, 255, 0, 117, 105, 110, 116, 51, 50, 0,
        ];

        let (_, quals) = parse_qualifier(&data, &value_data).unwrap();
        assert_eq!(quals.len(), 4);
    }

    #[test]
    fn test_parse_property() {
        let data = [
            86, 0, 0, 0, 112, 0, 0, 0, 2, 1, 0, 0, 17, 1, 0, 0, 66, 1, 0, 0, 75, 1, 0, 0,
        ];
        let value_data = [
            0, 77, 83, 70, 84, 95, 68, 79, 85, 112, 108, 111, 97, 100, 85, 115, 97, 103, 101, 0, 0,
            68, 101, 115, 99, 114, 105, 112, 116, 105, 111, 110, 0, 0, 50, 53, 0, 0, 68, 101, 108,
            105, 118, 101, 114, 121, 79, 112, 116, 105, 109, 105, 122, 97, 116, 105, 111, 110, 77,
            73, 80, 114, 111, 118, 0, 0, 67, 108, 97, 115, 115, 86, 101, 114, 115, 105, 111, 110,
            0, 0, 49, 46, 48, 46, 48, 0, 0, 77, 111, 110, 116, 104, 108, 121, 85, 112, 108, 111,
            97, 100, 82, 101, 115, 116, 114, 105, 99, 116, 105, 111, 110, 0, 17, 0, 0, 0, 7, 0, 30,
            0, 0, 0, 2, 0, 0, 0, 54, 0, 0, 0, 10, 0, 0, 128, 3, 8, 0, 0, 0, 180, 0, 0, 0, 187, 0,
            0, 0, 2, 8, 32, 0, 0, 197, 0, 0, 0, 222, 0, 0, 0, 2, 8, 32, 0, 0, 230, 0, 0, 0, 3, 0,
            0, 128, 2, 11, 0, 0, 0, 255, 255, 0, 117, 105, 110, 116, 56, 0, 0, 86, 97, 108, 117,
            101, 77, 97, 112, 0, 3, 0, 0, 0, 213, 0, 0, 0, 216, 0, 0, 0, 219, 0, 0, 0, 0, 48, 0, 0,
            49, 0, 0, 50, 0, 0, 86, 97, 108, 117, 101, 115, 0, 3, 0, 0, 0, 246, 0, 0, 0, 250, 0, 0,
            0, 254, 0, 0, 0, 0, 50, 50, 0, 0, 50, 51, 0, 0, 50, 52, 0, 0, 85, 112, 108, 111, 97,
            100, 82, 97, 116, 101, 80, 99, 116, 0, 17, 0, 0, 0, 5, 0, 25, 0, 0, 0, 2, 0, 0, 0, 28,
            0, 0, 0, 10, 0, 0, 128, 3, 8, 0, 0, 0, 59, 1, 0, 0, 3, 0, 0, 128, 2, 11, 0, 0, 0, 255,
            255, 0, 117, 105, 110, 116, 56, 0, 0, 85, 112, 108, 111, 97, 100, 115, 0, 19, 0, 0, 0,
            6, 0, 26, 0, 0, 0, 2, 0, 0, 0, 28, 0, 0, 0, 10, 0, 0, 128, 3, 8, 0, 0, 0, 117, 1, 0, 0,
            3, 0, 0, 128, 2, 11, 0, 0, 0, 255, 255, 0, 117, 105, 110, 116, 51, 50, 0,
        ];

        let (_, quals) = parse_property(&data, &value_data).unwrap();
        assert_eq!(quals.len(), 3);
    }

    #[test]
    fn test_get_predefine_name() {
        let test = 4;
        let result = get_predefine_name(&test);
        assert_eq!(result, "write");
    }

    #[test]
    fn test_get_cim_data_type() {
        let test = 0x15;
        let result = get_cim_data_type(&test);
        assert_eq!(result, CimType::Uint64);
    }

    #[test]
    fn test_extract_cim_data() {
        let remaining_input = [
            33, 0, 0, 0, 7, 0, 0, 128, 1, 11, 0, 0, 0, 255, 255, 6, 0, 0, 128, 1, 8, 0, 0, 0, 37,
            0, 0, 0, 65, 0, 0, 0, 0, 8, 0, 0, 0, 79, 0, 0, 0,
        ];
        let data = [
            0, 77, 83, 70, 84, 95, 68, 79, 85, 112, 108, 111, 97, 100, 85, 115, 97, 103, 101, 0, 0,
            68, 101, 115, 99, 114, 105, 112, 116, 105, 111, 110, 0, 0, 50, 53, 0, 0, 68, 101, 108,
            105, 118, 101, 114, 121, 79, 112, 116, 105, 109, 105, 122, 97, 116, 105, 111, 110, 77,
            73, 80, 114, 111, 118, 0, 0, 67, 108, 97, 115, 115, 86, 101, 114, 115, 105, 111, 110,
            0, 0, 49, 46, 48, 46, 48, 0, 0, 77, 111, 110, 116, 104, 108, 121, 85, 112, 108, 111,
            97, 100, 82, 101, 115, 116, 114, 105, 99, 116, 105, 111, 110, 0, 17, 0, 0, 0, 7, 0, 30,
            0, 0, 0, 2, 0, 0, 0, 54, 0, 0, 0, 10, 0, 0, 128, 3, 8, 0, 0, 0, 180, 0, 0, 0, 187, 0,
            0, 0, 2, 8, 32, 0, 0, 197, 0, 0, 0, 222, 0, 0, 0, 2, 8, 32, 0, 0, 230, 0, 0, 0, 3, 0,
            0, 128, 2, 11, 0, 0, 0, 255, 255, 0, 117, 105, 110, 116, 56, 0, 0, 86, 97, 108, 117,
            101, 77, 97, 112, 0, 3, 0, 0, 0, 213, 0, 0, 0, 216, 0, 0, 0, 219, 0, 0, 0, 0, 48, 0, 0,
            49, 0, 0, 50, 0, 0, 86, 97, 108, 117, 101, 115, 0, 3, 0, 0, 0, 246, 0, 0, 0, 250, 0, 0,
            0, 254, 0, 0, 0, 0, 50, 50, 0, 0, 50, 51, 0, 0, 50, 52, 0, 0, 85, 112, 108, 111, 97,
            100, 82, 97, 116, 101, 80, 99, 116, 0, 17, 0, 0, 0, 5, 0, 25, 0, 0, 0, 2, 0, 0, 0, 28,
            0, 0, 0, 10, 0, 0, 128, 3, 8, 0, 0, 0, 59, 1, 0, 0, 3, 0, 0, 128, 2, 11, 0, 0, 0, 255,
            255, 0, 117, 105, 110, 116, 56, 0, 0, 85, 112, 108, 111, 97, 100, 115, 0, 19, 0, 0, 0,
            6, 0, 26, 0, 0, 0, 2, 0, 0, 0, 28, 0, 0, 0, 10, 0, 0, 128, 3, 8, 0, 0, 0, 117, 1, 0, 0,
            3, 0, 0, 128, 2, 11, 0, 0, 0, 255, 255, 0, 117, 105, 110, 116, 51, 50, 0,
        ];

        let (_, result) = extract_cim_data(&CimType::String, &remaining_input, &data).unwrap();
        assert_eq!(result, Value::String(String::from("25")));
    }

    #[test]
    fn test_extract_cim_string() {
        let data = [
            0, 77, 83, 70, 84, 95, 68, 79, 85, 112, 108, 111, 97, 100, 85, 115, 97, 103, 101, 0,
        ];

        let (_, result) = extract_cim_string(&data).unwrap();
        assert_eq!(result, "MSFT_DOUploadUsage");
    }
}
