use super::wevt::ManifestTemplate;
use crate::utils::{
    nom_helper::{Endian, nom_unsigned_four_bytes, nom_unsigned_two_bytes},
    uuid::format_guid_le_bytes,
};
use nom::bytes::complete::take;
use std::collections::HashMap;

/// Parse WEVT header
pub(crate) fn parse_crimson(data: &[u8]) -> nom::IResult<&[u8], HashMap<String, ManifestTemplate>> {
    let (input, _sig) = nom_unsigned_four_bytes(data, Endian::Le)?;
    // Size is the entire file
    let (input, _size) = nom_unsigned_four_bytes(input, Endian::Le)?;
    let (input, _major_version) = nom_unsigned_two_bytes(input, Endian::Le)?;
    let (input, _minor_version) = nom_unsigned_two_bytes(input, Endian::Le)?;

    let (mut input, number_providers) = nom_unsigned_four_bytes(input, Endian::Le)?;

    let mut templates = HashMap::new();
    let mut count = 0;

    let guid_size: u8 = 16;
    while count < number_providers {
        let (remaining, guid_bytes) = take(guid_size)(input)?;
        let guid = format_guid_le_bytes(guid_bytes);
        let (remaining, offset) = nom_unsigned_four_bytes(remaining, Endian::Le)?;

        let template = ManifestTemplate {
            offset,
            element_offsets: Vec::new(),
            keywords: Vec::new(),
            channels: Vec::new(),
            maps: Vec::new(),
            opcodes: Vec::new(),
            levels: Vec::new(),
            tasks: Vec::new(),
            definitions: HashMap::new(),
        };

        templates.insert(guid, template);
        input = remaining;
        count += 1;
    }

    // HashMap is Provider GUID as key and Template info
    Ok((input, templates))
}

#[cfg(test)]
mod tests {
    use super::parse_crimson;

    #[test]
    fn test_parse_crimson() {
        let test = [
            67, 82, 73, 77, 208, 56, 0, 0, 5, 0, 1, 0, 1, 0, 0, 0, 108, 39, 153, 151, 4, 251, 232,
            71, 132, 94, 54, 148, 96, 69, 194, 24, 36, 0, 0, 0,
        ];

        let (_, result) = parse_crimson(&test).unwrap();
        assert_eq!(result.len(), 1);

        assert_eq!(
            result
                .get("9799276c-fb04-47e8-845e-36946045c218")
                .unwrap()
                .offset,
            36
        );
    }
}
