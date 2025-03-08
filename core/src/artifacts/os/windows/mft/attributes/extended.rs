use crate::utils::{
    encoding::base64_encode_standard,
    nom_helper::{Endian, nom_unsigned_four_bytes, nom_unsigned_one_byte, nom_unsigned_two_bytes},
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
    /// Parsed `ExtendInfo` attribute
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

    /// Parse `ExtendedAttribute` attribute
    pub(crate) fn parse_extended_attribute(data: &[u8]) -> nom::IResult<&[u8], ExtendedAttribute> {
        let (input, offset) = nom_unsigned_four_bytes(data, Endian::Le)?;
        let (input, flags) = nom_unsigned_one_byte(input, Endian::Le)?;
        let (input, name_size) = nom_unsigned_one_byte(input, Endian::Le)?;
        let (input, data_size) = nom_unsigned_two_bytes(input, Endian::Le)?;

        // Add one for end of string value (0)
        let (input, name_data) = take(name_size + 1)(input)?;

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

#[cfg(test)]
mod tests {
    use crate::artifacts::os::windows::mft::attributes::extended::ExtendedInfo;

    #[test]
    fn test_parse_extended_info() {
        let test = [222, 0, 0, 0, 232, 0, 0, 0];

        let (_, result) = ExtendedInfo::parse_extended_info(&test).unwrap();
        assert_eq!(result.entry_size, 222);
        assert_eq!(result.extend_count, 0);
        assert_eq!(result.size, 232);
    }

    #[test]
    fn test_parse_extended_attribute() {
        let test = [
            92, 0, 0, 0, 0, 15, 67, 0, 36, 67, 73, 46, 67, 65, 84, 65, 76, 79, 71, 72, 73, 78, 84,
            0, 1, 0, 63, 0, 80, 97, 99, 107, 97, 103, 101, 95, 49, 95, 102, 111, 114, 95, 75, 66,
            53, 48, 48, 52, 51, 52, 50, 126, 51, 49, 98, 102, 51, 56, 53, 54, 97, 100, 51, 54, 52,
            101, 51, 53, 126, 97, 109, 100, 54, 52, 126, 126, 49, 48, 46, 48, 46, 52, 52, 48, 48,
            46, 51, 46, 99, 97, 116, 0, 140, 0, 0, 0, 0, 22, 108, 0, 36, 75, 69, 82, 78, 69, 76,
            46, 80, 85, 82, 71, 69, 46, 69, 83, 66, 67, 65, 67, 72, 69, 0, 108, 0, 0, 0, 3, 0, 2,
            8, 208, 180, 58, 79, 218, 26, 216, 1, 0, 214, 248, 43, 155, 44, 215, 1, 66, 0, 0, 0,
            78, 0, 39, 1, 12, 128, 0, 0, 32, 21, 170, 67, 235, 31, 208, 49, 29, 6, 161, 124, 120,
            204, 99, 94, 63, 154, 228, 99, 111, 224, 128, 45, 109, 2, 240, 121, 82, 21, 172, 104,
            142, 39, 0, 12, 128, 0, 0, 32, 251, 35, 175, 94, 99, 10, 205, 86, 135, 228, 219, 68,
            150, 225, 205, 175, 182, 111, 134, 197, 70, 168, 253, 245, 21, 149, 125, 209, 100, 197,
            81, 127, 0,
        ];

        let (_, result) = ExtendedInfo::parse_extended_attribute(&test).unwrap();
        assert_eq!(result.flags, 0);
        assert_eq!(result.data_size, 67);
        assert_eq!(result.name, "$CI.CATALOGHINT");
        assert_eq!(
            result.data,
            "AQA/AFBhY2thZ2VfMV9mb3JfS0I1MDA0MzQyfjMxYmYzODU2YWQzNjRlMzV+YW1kNjR+fjEwLjAuNDQwMC4zLmNhdACMAAAAABZsACRLRVJORUwuUFVSR0UuRVNCQ0FDSEUAbAAAAAMAAgjQtDpP2hrYAQDW+CubLNcBQgAAAE4AJwEMgAAAIBWqQ+sf0DEdBqF8eMxjXj+a5GNv4IAtbQLweVIVrGiOJwAMgAAAIPsjr15jCs1Wh+TbRJbhza+2b4bFRqj99RWVfdFkxVF/AA=="
        );
        assert_eq!(result.name_size, 15);
    }
}
