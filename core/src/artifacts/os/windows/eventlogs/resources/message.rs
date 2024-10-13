use crate::utils::{
    nom_helper::{nom_unsigned_four_bytes, nom_unsigned_two_bytes, Endian},
    strings::{extract_ascii_utf16_string, extract_utf16_string, extract_utf8_string},
};
use nom::bytes::complete::take;
use std::collections::HashMap;

#[derive(Debug)]
pub(crate) struct MessageTable {
    pub(crate) id: u32,
    size: u16,
    flags: StringFlags,
    pub(crate) message: String,
}

struct Block {
    first_id: u32,
    last_id: u32,
    offset: u32,
}

#[derive(PartialEq, Debug)]
enum StringFlags {
    Ascii,
    Utf16,
}

pub(crate) fn parse_table(data: &[u8]) -> nom::IResult<&[u8], HashMap<u32, MessageTable>> {
    let (mut input, entry_count) = nom_unsigned_four_bytes(data, Endian::Le)?;
    let mut count = 0;

    let block_size: u8 = 12;
    let mut blocks = Vec::new();
    while count < entry_count {
        let (remaining, block_data) = take(block_size)(input)?;
        input = remaining;

        let (block_input, first_id) = nom_unsigned_four_bytes(block_data, Endian::Le)?;
        let (block_input, last_id) = nom_unsigned_four_bytes(block_input, Endian::Le)?;
        let (_, offset) = nom_unsigned_four_bytes(block_input, Endian::Le)?;

        let block = Block {
            first_id,
            last_id,
            offset,
        };
        blocks.push(block);
        count += 1;
    }

    let mut table = HashMap::new();
    for block in blocks {
        // Get the number of strings
        let string_count = block.last_id - block.first_id;
        count = 0;
        let (mut strings_start, _) = take(block.offset)(data)?;
        while count < string_count {
            let id = block.first_id + count;
            let (input, size) = nom_unsigned_two_bytes(strings_start, Endian::Le)?;
            let (input, flag) = nom_unsigned_two_bytes(input, Endian::Le)?;
            let flags = if flag == 0 || flag == 2 {
                StringFlags::Ascii
            } else {
                StringFlags::Utf16
            };

            let adjust = 4;

            // Should never happen
            if adjust > size {
                break;
            }

            let (input, string_data) = take(size - adjust)(input)?;
            strings_start = input;
            let message = if flags == StringFlags::Ascii {
                extract_utf8_string(string_data)
            } else {
                extract_ascii_utf16_string(string_data)
            };

            let message_table = MessageTable {
                id,
                size,
                flags,
                message,
            };
            table.insert(id, message_table);
            count += 1;
        }
    }

    Ok((&[], table))
}

#[cfg(test)]
mod tests {
    use super::parse_table;
    use crate::filesystem::files::read_file;
    use std::path::PathBuf;

    #[test]
    fn test_parse_table() {
        let mut test_location = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
        test_location.push("tests/test_data/windows/pe/resources/message_table.raw");

        let data = read_file(test_location.to_str().unwrap()).unwrap();
        let (_, result) = parse_table(&data).unwrap();
        assert_eq!(result.len(), 30);
    }
}
