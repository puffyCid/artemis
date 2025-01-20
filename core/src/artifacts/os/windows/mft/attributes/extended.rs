use crate::utils::{
    encoding::base64_encode_standard,
    nom_helper::{nom_unsigned_four_bytes, nom_unsigned_one_byte, nom_unsigned_two_bytes, Endian},
    strings::extract_utf8_string,
};
use nom::bytes::complete::take;
use serde::Serialize;

#[derive(Debug, Serialize)]
pub(crate) struct ExtendedInfo {
    entry_size: u16,
    extend_count: u16,
    size: u32,
}

#[derive(Debug, Serialize)]
pub(crate) struct ExtendedAttribute {
    offset: u32,
    flags: u8,
    name_size: u8,
    data_size: u16,
    name: String,
    data: String,
}

impl ExtendedInfo {
    pub(crate) fn parse_extended_info(data: &[u8]) -> nom::IResult<&[u8], ExtendedInfo> {
        let (input, entry_size) = nom_unsigned_two_bytes(data, Endian::Le)?;
        let (input, extend_count) = nom_unsigned_two_bytes(input, Endian::Le)?;
        let (input, size) = nom_unsigned_four_bytes(input, Endian::Le)?;

        let info = ExtendedInfo {
            entry_size,
            extend_count,
            size,
        };

        Ok((input, info))
    }

    pub(crate) fn parse_extended_attribute(data: &[u8]) -> nom::IResult<&[u8], ExtendedAttribute> {
        let (input, offset) = nom_unsigned_four_bytes(data, Endian::Le)?;
        let (input, flags) = nom_unsigned_one_byte(input, Endian::Le)?;
        let (input, name_size) = nom_unsigned_one_byte(input, Endian::Le)?;
        let (input, data_size) = nom_unsigned_two_bytes(input, Endian::Le)?;

        // Add one for end of string value (0)
        let (input, name_data) = take(name_size + 1)(input)?;
        //let (input, value_data) = take(data_size)(input)?;

        let attrib = ExtendedAttribute {
            offset,
            flags,
            name_size,
            data_size,
            name: extract_utf8_string(name_data),
            data: base64_encode_standard(input),
        };

        Ok((&[], attrib))
    }
}
