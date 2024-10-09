use super::{channel::Channel, opcode::Opcode, xml::TemplateElement};
use crate::utils::{
    nom_helper::{nom_unsigned_four_bytes, nom_unsigned_two_bytes, Endian},
    uuid::format_guid_le_bytes,
};
use nom::bytes::complete::take;
use std::collections::HashMap;

#[derive(Debug)]
pub(crate) struct ManifestTemplate {
    pub(crate) xml: String,
    /**Offset to start of Provider */
    pub(crate) offset: u32,
    pub(crate) message_table_id: i32,
    pub(crate) element_offsets: Vec<u32>,
    pub(crate) channels: Vec<Channel>,
    pub(crate) keywords: Vec<String>,
    pub(crate) opcodes: Vec<Opcode>,
    pub(crate) templates: Vec<TemplateElement>,
}

pub(crate) fn parse_manifest(
    data: &[u8],
) -> nom::IResult<&[u8], HashMap<String, ManifestTemplate>> {
    let (input, sig) = nom_unsigned_four_bytes(data, Endian::Le)?;
    // Size is the entire file
    let (input, size) = nom_unsigned_four_bytes(input, Endian::Le)?;
    let (input, major_version) = nom_unsigned_two_bytes(input, Endian::Le)?;
    let (input, minor_version) = nom_unsigned_two_bytes(input, Endian::Le)?;

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
            xml: String::new(),
            message_table_id: 0,
            element_offsets: Vec::new(),
            keywords: Vec::new(),
            channels: Vec::new(),
            templates: Vec::new(),
            opcodes: Vec::new(),
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
    use super::parse_manifest;

    #[test]
    fn test_parse_manifest() {
        let test = [
            67, 82, 73, 77, 208, 56, 0, 0, 5, 0, 1, 0, 1, 0, 0, 0, 108, 39, 153, 151, 4, 251, 232,
            71, 132, 94, 54, 148, 96, 69, 194, 24, 36, 0, 0, 0,
        ];

        let (_, result) = parse_manifest(&test).unwrap();
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
