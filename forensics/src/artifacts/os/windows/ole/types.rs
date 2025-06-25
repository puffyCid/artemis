use crate::{
    artifacts::os::windows::shellitems::items::detect_shellitem,
    utils::{
        encoding::base64_encode_standard,
        nom_helper::{
            Endian, nom_signed_eight_bytes, nom_signed_four_bytes, nom_signed_two_bytes,
            nom_unsigned_eight_bytes, nom_unsigned_four_bytes, nom_unsigned_one_byte,
            nom_unsigned_sixteen_bytes, nom_unsigned_two_bytes,
        },
        strings::{extract_utf8_string, extract_utf16_string},
        time::{filetime_to_unixepoch, ole_automationtime_to_unixepoch, unixepoch_to_iso},
        uuid::format_guid_le_bytes,
    },
};
use common::windows::ShellItem;
use log::warn;
use nom::{
    bytes::complete::{take, take_while},
    number::complete::{le_f32, le_f64},
};
use serde_json::Value;
use std::{collections::HashMap, mem::size_of};

/// Parse different OLE Type definitions
pub(crate) fn parse_types<'a>(
    data: &'a [u8],
    ole_type: &u16,
    values: &mut HashMap<String, Value>,
    key: String,
) -> nom::IResult<&'a [u8], Vec<ShellItem>> {
    // List at https://github.com/libyal/libfole/blob/main/documentation/OLE%20definitions.asciidoc
    let (input, result) = match ole_type {
        0x48 => {
            let (input, id_data) = take(size_of::<u128>())(data)?;
            (input, Value::String(format_guid_le_bytes(id_data)))
        }
        0x0 | 0x1 => (data, Value::Null),
        0x2 => {
            let (input, result) = nom_signed_two_bytes(data, Endian::Le)?;
            (input, Value::Number(result.into()))
        }
        0x3 | 0xa | 0x16 => {
            let (input, result) = nom_signed_four_bytes(data, Endian::Le)?;
            (input, Value::Number(result.into()))
        }
        0x4 => {
            let (input, vt_data) = take(size_of::<u32>())(data)?;
            let (_, result) = le_f32(vt_data)?;
            (input, Value::String(format!("{result}")))
        }
        0x5 | 0x6 => {
            let (input, vt_data) = take(size_of::<u64>())(data)?;
            let (_, result) = le_f64(vt_data)?;
            (input, Value::String(format!("{result}")))
        }
        0x7 => {
            let (input, vt_data) = take(size_of::<u64>())(data)?;
            let (_, oletime) = le_f64(vt_data)?;
            (
                input,
                Value::String(unixepoch_to_iso(ole_automationtime_to_unixepoch(oletime))),
            )
        }
        0x8 => {
            let (input, size) = nom_unsigned_four_bytes(data, Endian::Le)?;
            let (input, binary_string) = take(size)(input)?;
            (input, Value::String(base64_encode_standard(binary_string)))
        }
        0xb => {
            let (input, result) = nom_unsigned_one_byte(data, Endian::Le)?;
            if result == 0 {
                (input, Value::Bool(false))
            } else {
                (input, Value::Bool(true))
            }
        }
        0x10 | 0x11 => {
            let (input, result) = nom_unsigned_one_byte(data, Endian::Le)?;
            (input, Value::Number(result.into()))
        }
        0xe => {
            let (input, result) = nom_unsigned_sixteen_bytes(data, Endian::Le)?;
            (input, Value::String(format!("{result}")))
        }
        0x12 => {
            let (input, result) = nom_unsigned_two_bytes(data, Endian::Le)?;
            (input, Value::Number(result.into()))
        }
        0x13 | 0x17 => {
            let (input, result) = nom_unsigned_four_bytes(data, Endian::Le)?;
            (input, Value::Number(result.into()))
        }
        0x14 => {
            let (input, result) = nom_signed_eight_bytes(data, Endian::Le)?;
            (input, Value::Number(result.into()))
        }
        0x15 => {
            let (input, result) = nom_unsigned_eight_bytes(data, Endian::Le)?;
            (input, Value::Number(result.into()))
        }
        0x1e => {
            let (input, string_data) = take_while(|b| b != 0)(data)?;
            let (input, _eof) = nom_unsigned_one_byte(input, Endian::Le)?;
            (input, Value::String(extract_utf8_string(string_data)))
        }
        0x40 => {
            let (input, filetime) = nom_unsigned_eight_bytes(data, Endian::Le)?;
            (
                input,
                Value::String(unixepoch_to_iso(filetime_to_unixepoch(filetime))),
            )
        }
        0x42 => {
            let (input, result) = parse_stream(data, values)?;
            return Ok((input, result));
        }
        0x1f => {
            let (input, string_size) = nom_unsigned_four_bytes(data, Endian::Le)?;
            let utf_adjust = 2;
            let (input, string_data) = take(string_size * utf_adjust)(input)?;

            (input, Value::String(extract_utf16_string(string_data)))
        }
        _ => {
            warn!("[olecf] Unknown/Unsupported ole type {ole_type}");
            return Ok((&[], Vec::new()));
        }
    };

    values.insert(key, result);

    // No shellitems if the ole_type is not 0x42 (stream)
    Ok((input, Vec::new()))
}

/// Parse the OLE Stream aassociated with type 0x42
fn parse_stream<'a>(
    data: &'a [u8],
    values: &mut HashMap<String, Value>,
) -> nom::IResult<&'a [u8], Vec<ShellItem>> {
    let (input, value_size) = nom_unsigned_four_bytes(data, Endian::Le)?;
    let (input, value_name) = take(value_size)(input)?;

    let (input, _padding) = nom_unsigned_two_bytes(input, Endian::Le)?;
    let (input, stream_size) = nom_unsigned_four_bytes(input, Endian::Le)?;
    let (remaining_input, input) = take(stream_size)(input)?;

    let (input, guid_data) = take(size_of::<u128>())(input)?;
    let unknown_size: u8 = 38;

    let (mut input, _unknown) = take(unknown_size)(input)?;
    let mut shellitems_vec: Vec<ShellItem> = Vec::new();

    // ShellItems are back to back
    while !input.is_empty() {
        let (shell_input, item_size) = nom_unsigned_two_bytes(input, Endian::Le)?;
        let empty = 0;

        let adjust_size = 2;
        if item_size == empty || item_size < adjust_size {
            break;
        }

        // Size includes size itself
        let (item_remaining, shellitem_data) = take(item_size - adjust_size)(shell_input)?;
        let item_result = detect_shellitem(shellitem_data);
        let shellitem = match item_result {
            Ok((_, result)) => result,
            Err(_err) => {
                warn!("[ole] Could not parse shellitem");
                break;
            }
        };

        shellitems_vec.push(shellitem);
        input = item_remaining;
    }

    values.insert(
        String::from("other_values"),
        Value::Array(vec![
            Value::String(extract_utf16_string(value_name)),
            Value::String(format_guid_le_bytes(guid_data)),
        ]),
    );

    Ok((remaining_input, shellitems_vec))
}

#[cfg(test)]
mod tests {
    use std::collections::HashMap;

    use super::parse_stream;
    use crate::artifacts::os::windows::ole::types::parse_types;

    #[test]
    fn test_parse_types() {
        let test_data = [
            129, 48, 105, 195, 194, 204, 140, 77, 128, 223, 108, 13, 216, 242, 103, 9, 0, 0, 0, 0,
            0, 0, 0, 0, 0, 0, 0, 0,
        ];
        let mut values = HashMap::new();
        let _ = parse_types(&test_data, &0x48, &mut values, String::from("value")).unwrap();
        assert_eq!(
            values.get("value").unwrap(),
            "c3693081-ccc2-4d8c-80df-6c0dd8f26709"
        );
    }

    #[test]
    fn test_parse_stream() {
        let test_data = [
            30, 0, 0, 0, 112, 0, 114, 0, 111, 0, 112, 0, 52, 0, 50, 0, 57, 0, 52, 0, 57, 0, 54, 0,
            55, 0, 50, 0, 57, 0, 53, 0, 0, 0, 0, 0, 221, 1, 0, 0, 174, 165, 78, 56, 225, 173, 138,
            78, 138, 155, 123, 234, 120, 255, 241, 233, 6, 0, 0, 128, 0, 0, 0, 0, 1, 0, 0, 0, 2, 0,
            0, 128, 1, 0, 0, 0, 1, 0, 0, 0, 2, 0, 0, 0, 32, 0, 0, 0, 0, 0, 0, 0, 223, 0, 20, 0, 31,
            80, 224, 79, 208, 32, 234, 58, 105, 16, 162, 216, 8, 0, 43, 48, 48, 157, 25, 0, 47, 67,
            58, 92, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 86, 0, 49, 0, 0, 0, 0,
            0, 146, 82, 103, 139, 16, 0, 87, 105, 110, 100, 111, 119, 115, 0, 64, 0, 9, 0, 4, 0,
            239, 190, 135, 79, 119, 72, 171, 82, 196, 34, 46, 0, 0, 0, 204, 110, 1, 0, 0, 0, 3, 0,
            0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 211, 130, 49, 0, 87, 0, 105, 0, 110, 0, 100,
            0, 111, 0, 119, 0, 115, 0, 0, 0, 22, 0, 90, 0, 49, 0, 0, 0, 0, 0, 171, 82, 240, 34, 16,
            32, 80, 114, 101, 102, 101, 116, 99, 104, 0, 0, 66, 0, 9, 0, 4, 0, 239, 190, 36, 81,
            114, 25, 171, 82, 242, 34, 46, 0, 0, 0, 246, 84, 0, 0, 0, 0, 17, 0, 0, 0, 0, 0, 0, 0,
            0, 0, 0, 0, 0, 0, 0, 0, 98, 34, 97, 0, 80, 0, 114, 0, 101, 0, 102, 0, 101, 0, 116, 0,
            99, 0, 104, 0, 0, 0, 24, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0,
            0, 0, 1, 0, 0, 0, 1, 0, 0, 128, 1, 0, 0, 0, 4, 0, 105, 0, 116, 0, 101, 0, 109, 0, 0, 0,
            0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0,
            0, 0, 0, 0, 0, 0, 0, 0, 0, 30, 26, 222, 127, 49, 139, 165, 73, 147, 184, 107, 225, 76,
            250, 73, 67, 255, 255, 255, 255, 255, 255, 255, 255, 255, 255, 255, 255, 0, 0, 0, 0, 1,
            0, 0, 0, 26, 0, 83, 0, 101, 0, 97, 0, 114, 0, 99, 0, 104, 0, 32, 0, 82, 0, 101, 0, 115,
            0, 117, 0, 108, 0, 116, 0, 115, 0, 32, 0, 105, 0, 110, 0, 32, 0, 80, 0, 114, 0, 101, 0,
            102, 0, 101, 0, 116, 0, 99, 0, 104, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0,
            0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0,
        ];
        let mut values = HashMap::new();

        let (_, items) = parse_stream(&test_data, &mut values).unwrap();
        assert_eq!(
            values.get("other_values").unwrap().as_array().unwrap()[0],
            "prop4294967295"
        );
        assert_eq!(items.len(), 4);
    }
}
