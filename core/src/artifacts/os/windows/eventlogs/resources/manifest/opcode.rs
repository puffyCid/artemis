use super::crimson::ManifestTemplate;
use crate::utils::{
    nom_helper::{nom_signed_four_bytes, nom_unsigned_four_bytes, Endian},
    strings::extract_utf16_string,
};
use nom::bytes::complete::take;

#[derive(Debug)]
pub(crate) struct Opcode {
    message_id: i32,
    /**Bitmask? */
    id: u32,
    value: String,
    /**Offset from start of the data */
    offset: u32,
}

pub(crate) fn parse_opcode<'a>(
    resource: &'a [u8],
    data: &'a [u8],
    template: &mut ManifestTemplate,
) -> nom::IResult<&'a [u8], ()> {
    let (input, sig) = nom_unsigned_four_bytes(data, Endian::Le)?;
    let (input, size) = nom_unsigned_four_bytes(input, Endian::Le)?;
    let empty = 0;
    if size == empty {
        return Ok((input, ()));
    }

    let (mut input, op_count) = nom_unsigned_four_bytes(input, Endian::Le)?;

    let mut count = 0;
    println!("{op_count}");
    while count < op_count {
        let (remaining, id) = nom_unsigned_four_bytes(input, Endian::Le)?;
        let (remaining, message_id) = nom_signed_four_bytes(remaining, Endian::Le)?;
        let (remaining, offset) = nom_unsigned_four_bytes(remaining, Endian::Le)?;

        input = remaining;
        count += 1;

        let (string_start, _) = take(offset)(resource)?;
        let (string_data, size) = nom_unsigned_four_bytes(string_start, Endian::Le)?;

        let adjust_size = 4;
        if adjust_size > size {
            // Should not happen
            continue;
        }
        // Size includes size itself
        let (_, value_data) = take(size - adjust_size)(string_data)?;

        let value = extract_utf16_string(value_data);

        let op = Opcode {
            message_id,
            id,
            value,
            offset,
        };

        template.opcodes.push(op);
    }

    Ok((input, ()))
}

#[cfg(test)]
mod tests {
    use crate::{
        artifacts::os::windows::eventlogs::resources::manifest::{
            crimson::parse_manifest, opcode::parse_opcode,
        },
        filesystem::files::read_file,
        utils::nom_helper::nom_data,
    };
    use std::path::PathBuf;

    #[test]
    fn test_parse_opcode() {
        let mut test_location = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
        test_location.push("tests/test_data/windows/pe/resources/wevt_template.raw");

        let data = read_file(test_location.to_str().unwrap()).unwrap();
        let (_, mut template) = parse_manifest(&data).unwrap();
        let manifest = template
            .get_mut("9799276c-fb04-47e8-845e-36946045c218")
            .unwrap();

        let start = 12368;
        let (table_start, _) = nom_data(&data, start).unwrap();
        let (_input, _) = parse_opcode(&data, table_start, manifest).unwrap();
        assert_eq!(manifest.opcodes.len(), 8);
        assert_eq!(manifest.opcodes[5].value, "Retry");
        assert_eq!(manifest.opcodes[5].message_id, 805306472);
        assert_eq!(manifest.opcodes[5].id, 6815744);
    }
}
