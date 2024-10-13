use std::collections::HashMap;

use nom::bytes::complete::take;

use super::{table::parse_template, xml::TemplateElement};
use crate::utils::nom_helper::{
    nom_signed_four_bytes, nom_unsigned_eight_bytes, nom_unsigned_four_bytes,
    nom_unsigned_one_byte, nom_unsigned_two_bytes, Endian,
};

#[derive(Debug)]
pub(crate) struct Definition {
    pub(crate) id: u16,
    pub(crate) version: u8,
    pub(crate) channel: u8,
    pub(crate) level: u8,
    pub(crate) opcode: u8,
    pub(crate) task: u16,
    pub(crate) keywords: u64,
    pub(crate) message_id: u32,
    pub(crate) temp_offset: u32,
    pub(crate) template: Option<TemplateElement>,
    pub(crate) opcode_offset: u32,
    pub(crate) level_offset: u32,
    pub(crate) task_offset: u32,
}

/// Parse the manifest definition and then get the template format
pub(crate) fn parse_definition<'a>(
    resource: &'a [u8],
    data: &'a [u8],
) -> nom::IResult<&'a [u8], HashMap<String, Definition>> {
    let (input, _sig) = nom_unsigned_four_bytes(data, Endian::Le)?;
    let (input, size) = nom_unsigned_four_bytes(input, Endian::Le)?;
    let empty = 0;
    if size == empty {
        return Ok((input, HashMap::new()));
    }

    let (input, def_count) = nom_unsigned_four_bytes(input, Endian::Le)?;
    let (mut input, _unknown) = nom_unsigned_four_bytes(input, Endian::Le)?;

    let mut count = 0;
    let mut defs = HashMap::new();
    while count < def_count {
        let (remaining, id) = nom_unsigned_two_bytes(input, Endian::Le)?;
        let (remaining, version) = nom_unsigned_one_byte(remaining, Endian::Le)?;
        let (remaining, channel) = nom_unsigned_one_byte(remaining, Endian::Le)?;
        let (remaining, level) = nom_unsigned_one_byte(remaining, Endian::Le)?;
        let (remaining, opcode) = nom_unsigned_one_byte(remaining, Endian::Le)?;
        let (remaining, task) = nom_unsigned_two_bytes(remaining, Endian::Le)?;

        let (remaining, keywords) = nom_unsigned_eight_bytes(remaining, Endian::Le)?;
        let (remaining, message_id) = nom_unsigned_four_bytes(remaining, Endian::Le)?;

        let (remaining, temp_offset) = nom_unsigned_four_bytes(remaining, Endian::Le)?;
        let (remaining, opcode_offset) = nom_unsigned_four_bytes(remaining, Endian::Le)?;
        let (remaining, level_offset) = nom_unsigned_four_bytes(remaining, Endian::Le)?;
        let (remaining, task_offset) = nom_unsigned_four_bytes(remaining, Endian::Le)?;

        let (remaining, _unknown) = nom_unsigned_four_bytes(remaining, Endian::Le)?;
        let (remaining, _unknown) = nom_unsigned_four_bytes(remaining, Endian::Le)?;
        let (remaining, _unknown) = nom_unsigned_four_bytes(remaining, Endian::Le)?;

        input = remaining;
        count += 1;

        let mut def = Definition {
            id,
            version,
            channel,
            level,
            opcode,
            task,
            keywords,
            message_id,
            temp_offset,
            opcode_offset,
            level_offset,
            task_offset,
            template: None,
        };

        let no_temp = 0;
        if def.temp_offset == no_temp {
            // Use ID and version number as our unique key. This will map to event ID and version. Ex: EventID 4624 version 3.
            defs.insert(format!("{}_{}", def.id, def.version), def);
            continue;
        }

        let (temp_start, _) = take(def.temp_offset)(resource)?;
        // Get template manifest format. Can be used to assemble eventlog strings
        let (_, template) = parse_template(temp_start)?;
        def.template = Some(template);

        defs.insert(format!("{}_{}", def.id, def.version), def);
    }

    Ok((input, defs))
}

#[cfg(test)]
mod tests {
    use crate::{
        artifacts::os::windows::eventlogs::resources::manifest::defintion::parse_definition,
        filesystem::files::read_file, utils::nom_helper::nom_data,
    };
    use std::path::PathBuf;

    #[test]
    fn test_parse_definition() {
        let mut test_location = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
        test_location.push("tests/test_data/windows/pe/resources/wevt_template.raw");

        let data = read_file(test_location.to_str().unwrap()).unwrap();

        let start = 13656;
        let (table_start, _) = nom_data(&data, start).unwrap();
        let (_input, defs) = parse_definition(&data, table_start).unwrap();
        assert_eq!(defs.len(), 16);
        for value in defs.values() {
            if value.id == 21 {
                assert_eq!(value.id, 211);
                assert_eq!(value.message_id, 2952855763);
                assert_eq!(value.version, 1);
                assert_eq!(value.level, 4);
                assert_eq!(value.keywords, 9223372062624579584);
                assert_eq!(value.temp_offset, 11280);
                assert_eq!(value.template.as_ref().unwrap().elements.len(), 8);
            }
        }
    }
}
