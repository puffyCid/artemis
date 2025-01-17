use super::header::{AttributeHeader, AttributeType};
use crate::utils::{
    nom_helper::{
        nom_unsigned_eight_bytes, nom_unsigned_four_bytes, nom_unsigned_one_byte,
        nom_unsigned_two_bytes, Endian,
    },
    strings::extract_utf16_string,
};
use nom::bytes::complete::take;
use serde_json::Value;

#[derive(Debug)]
pub(crate) struct AttributeList {
    attribute_type: AttributeType,
    size: u16,
    name_size: u8,
    name_offset: u8,
    attribute_name: String,
    vcn: u64,
    parent_mft: u32,
    parent_sequence: u16,
    attribute_id: u16,
    attribute: Value,
}

impl AttributeList {
    pub(crate) fn parse_list(data: &[u8]) -> nom::IResult<&[u8], Vec<AttributeList>> {
        let mut remaining = data;
        let min_size = 26;
        while remaining.len() >= min_size {
            let (input, attribute_type) = nom_unsigned_four_bytes(remaining, Endian::Le)?;
            let (input, size) = nom_unsigned_two_bytes(input, Endian::Le)?;
            let (input, name_size) = nom_unsigned_one_byte(input, Endian::Le)?;
            let (input, name_offset) = nom_unsigned_one_byte(input, Endian::Le)?;
            let (input, vcn) = nom_unsigned_eight_bytes(input, Endian::Le)?;
            let (input, parent_mft) = nom_unsigned_four_bytes(input, Endian::Le)?;
            let (input, _padding) = nom_unsigned_two_bytes(input, Endian::Le)?;
            let (input, parent_sequence) = nom_unsigned_two_bytes(input, Endian::Le)?;
            let (input, attribute_id) = nom_unsigned_two_bytes(input, Endian::Le)?;

            let (input, name_data) = take(name_size * 2)(input)?;
            let attribute_name = extract_utf16_string(name_data);

            let padding_size: u8 = 6;
            let (input, _padding) = take(padding_size)(input)?;
            remaining = input;

            let list = AttributeList {
                attribute_type: AttributeHeader::get_type(&attribute_type),
                size,
                name_size,
                name_offset,
                attribute_name,
                vcn,
                parent_mft,
                parent_sequence,
                attribute_id,
                attribute: Value::Null,
            };

            println!("{list:?}");
        }
        panic!("lookup list entries in mft?");
        Ok((remaining, Vec::new()))
    }
}
