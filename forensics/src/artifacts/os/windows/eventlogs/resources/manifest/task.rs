use crate::utils::{
    nom_helper::{Endian, nom_signed_four_bytes, nom_unsigned_four_bytes},
    strings::extract_utf16_string,
    uuid::format_guid_le_bytes,
};
use nom::bytes::complete::take;
use serde::{Deserialize, Serialize};

#[derive(Debug, Serialize, Deserialize)]
pub(crate) struct Task {
    message_id: i32,
    /**Bitmask? */
    id: u32,
    guid: String,
    value: String,
}

/// Parse WEVT task info
pub(crate) fn parse_task<'a>(
    resource: &'a [u8],
    data: &'a [u8],
) -> nom::IResult<&'a [u8], Vec<Task>> {
    let (input, _sig) = nom_unsigned_four_bytes(data, Endian::Le)?;
    let (input, size) = nom_unsigned_four_bytes(input, Endian::Le)?;
    let empty = 0;
    if size == empty {
        return Ok((input, Vec::new()));
    }

    let (mut input, task_count) = nom_unsigned_four_bytes(input, Endian::Le)?;

    let mut count = 0;
    let mut tasks = Vec::new();
    while count < task_count {
        let (remaining, id) = nom_unsigned_four_bytes(input, Endian::Le)?;

        let (remaining, message_id) = nom_signed_four_bytes(remaining, Endian::Le)?;

        let guid_size: u8 = 16;
        let (remaining, guid_bytes) = take(guid_size)(remaining)?;
        let mui_id = format_guid_le_bytes(guid_bytes);
        let (remaining, offset) = nom_unsigned_four_bytes(remaining, Endian::Le)?;
        input = remaining;

        let (string_start, _) = take(offset)(resource)?;
        let (string_data, size) = nom_unsigned_four_bytes(string_start, Endian::Le)?;

        let adjust_size = 4;
        // Should not happen
        if adjust_size > size {
            break;
        }
        // Size includes size itself
        let (_, value_data) = take(size - adjust_size)(string_data)?;

        let value = extract_utf16_string(value_data);

        let task = Task {
            message_id,
            guid: mui_id,
            id,
            value,
        };

        tasks.push(task);
        count += 1;
    }

    Ok((input, tasks))
}

#[cfg(test)]
mod tests {
    use crate::{
        artifacts::os::windows::eventlogs::resources::manifest::task::parse_task,
        filesystem::files::read_file, utils::nom_helper::nom_data,
    };
    use std::path::PathBuf;

    #[test]
    fn test_parse_task() {
        let mut test_location = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
        test_location.push("tests/test_data/windows/pe/resources/wevt_template.raw");

        let data = read_file(test_location.to_str().unwrap()).unwrap();

        let start = 12764;
        let (table_start, _) = nom_data(&data, start).unwrap();
        let (_input, tasks) = parse_task(&data, table_start).unwrap();
        assert_eq!(tasks[0].value, "Class");
        assert_eq!(tasks[0].guid, "00000000-0000-0000-0000-000000000000");
    }
}
