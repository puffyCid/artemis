use crate::utils::nom_helper::{
    nom_signed_four_bytes, nom_unsigned_eight_bytes, nom_unsigned_four_bytes,
    nom_unsigned_one_byte, nom_unsigned_two_bytes, Endian,
};

#[derive(Debug)]
pub(crate) struct Definition {
    id: u16,
    version: u8,
    channel: u8,
    level: u8,
    opcode: u8,
    task: u16,
    keywords: u64,
    message_id: u32,
    temp_offset: u32,
    opcode_offset: u32,
    level_offset: u32,
    task_offset: u32,
}

pub(crate) fn parse_definition(data: &[u8]) -> nom::IResult<&[u8], Vec<Definition>> {
    let (input, _sig) = nom_unsigned_four_bytes(data, Endian::Le)?;
    let (input, size) = nom_unsigned_four_bytes(input, Endian::Le)?;
    let empty = 0;
    if size == empty {
        return Ok((input, Vec::new()));
    }

    let (input, def_count) = nom_unsigned_four_bytes(input, Endian::Le)?;
    let (mut input, _unknown) = nom_unsigned_four_bytes(input, Endian::Le)?;

    let mut count = 0;
    let mut defs = Vec::new();
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

        let def = Definition {
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
        };

        defs.push(def);
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
        let (_input, defs) = parse_definition(table_start).unwrap();
        assert_eq!(defs.len(), 16);
        assert_eq!(defs[11].id, 211);
        assert_eq!(defs[11].message_id, 2952855763);
        assert_eq!(defs[11].version, 1);
        assert_eq!(defs[11].level, 4);
        assert_eq!(defs[11].keywords, 9223372062624579584);
        assert_eq!(defs[11].temp_offset, 11280);
    }
}
