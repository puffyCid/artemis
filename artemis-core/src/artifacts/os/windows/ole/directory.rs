use crate::utils::{
    nom_helper::{
        nom_signed_eight_bytes, nom_signed_four_bytes, nom_unsigned_eight_bytes,
        nom_unsigned_four_bytes, nom_unsigned_one_byte, nom_unsigned_two_bytes, Endian,
    },
    strings::extract_utf16_string,
    time::filetime_to_unixepoch,
    uuid::format_guid_le_bytes,
};
use nom::bytes::complete::take;

/// Using the SAT or SSAT Slot values, assemble the OLE data
pub(crate) fn assemble_ole_data<'a>(
    data: &'a [u8],
    slots: &[i32],
    start: u32,
    size: u32,
) -> nom::IResult<&'a [u8], Vec<u8>> {
    let mut ole_data = Vec::new();

    // Go to start of first OLE sector
    let (dir_start, _) = take(start * size)(data)?;
    if dir_start.len() < size as usize {
        ole_data.append(&mut dir_start.to_vec());
        return Ok((data, ole_data));
    }

    // Get data based on sector or stream size
    let (_, value) = take(size)(dir_start)?;

    ole_data.append(&mut value.to_vec());
    let mut slot_value = start;

    // Now use the slots to determine the OLE data
    // Loop until negative slot value is encountered
    while slots.len() > slot_value as usize {
        // start also represents first slot index
        let slot = slots[slot_value as usize];
        // Any negative value means we have reached end
        if slot < 0 {
            break;
        }

        // Use slot value to jump to next OLE sector
        let (dir_start, _) = take(slot as u32 * size)(data)?;

        if dir_start.len() < size as usize {
            // Get rest of stream data
            let (_, value) = take(dir_start.len())(dir_start)?;

            ole_data.append(&mut value.to_vec());
            break;
        }

        // Get data based on sector or stream size
        let (_, value) = take(size)(dir_start)?;
        // the slot value then points to the next slot
        slot_value = slot as u32;

        ole_data.append(&mut value.to_vec());
    }

    Ok((data, ole_data))
}

#[derive(Debug)]
pub(crate) struct OleDirectory {
    pub(crate) name: String,
    name_size: u16,
    pub(crate) directory_type: DirectoryType,
    directory_color: DirectoryColor,
    previous_id: i32,
    id: i32,
    next_id: i32,
    class_id: String,
    flags: u32,
    created: i64,
    modified: i64,
    pub(crate) sector_id: i32,
    pub(crate) directory_size: u32,
    reserved: u32,
}

#[derive(Debug, PartialEq)]
pub(crate) enum DirectoryType {
    Empty,
    Storage,
    Stream,
    LockBytes,
    Property,
    Root,
    Unknown,
}

#[derive(Debug)]
/// Refers to color of red-black tree `https://en.wikipedia.org/wiki/Red%E2%80%93black_tree`
pub(crate) enum DirectoryColor {
    Red,
    Black,
    Unknown,
}

/// Parse OLE Directory data
pub(crate) fn parse_directory(data: &[u8]) -> nom::IResult<&[u8], Vec<OleDirectory>> {
    let min_size = 128;

    let mut input = data;
    let mut dir_entries = Vec::new();
    while input.len() >= min_size {
        let string_size: u8 = 64;
        let (remaining_input, string_data) = take(string_size)(input)?;
        let (remaining_input, name_size) = nom_unsigned_two_bytes(remaining_input, Endian::Le)?;
        let (remaining_input, type_data) = nom_unsigned_one_byte(remaining_input, Endian::Le)?;
        let (remaining_input, color_data) = nom_unsigned_one_byte(remaining_input, Endian::Le)?;

        let directory_color = if color_data == 0 {
            DirectoryColor::Red
        } else if color_data == 1 {
            DirectoryColor::Black
        } else {
            DirectoryColor::Unknown
        };

        let (remaining_input, previous_id) = nom_signed_four_bytes(remaining_input, Endian::Le)?;
        let (remaining_input, next_id) = nom_signed_four_bytes(remaining_input, Endian::Le)?;
        let (remaining_input, id) = nom_signed_four_bytes(remaining_input, Endian::Le)?;

        let class_size: u8 = 16;
        let (remaining_input, class_data) = take(class_size)(remaining_input)?;
        let (remaining_input, flags) = nom_unsigned_four_bytes(remaining_input, Endian::Le)?;

        let (remaining_input, created) = nom_unsigned_eight_bytes(remaining_input, Endian::Le)?;
        let (remaining_input, modified) = nom_unsigned_eight_bytes(remaining_input, Endian::Le)?;
        let (remaining_input, sector_id) = nom_signed_four_bytes(remaining_input, Endian::Le)?;

        let (remaining_input, directory_size) =
            nom_unsigned_four_bytes(remaining_input, Endian::Le)?;
        let (remaining_input, reserved) = nom_unsigned_four_bytes(remaining_input, Endian::Le)?;

        input = remaining_input;

        let directory = OleDirectory {
            name: extract_utf16_string(string_data),
            name_size,
            directory_type: parse_directory_type(&type_data),
            directory_color,
            previous_id,
            id,
            next_id,
            class_id: format_guid_le_bytes(class_data),
            flags,
            created: filetime_to_unixepoch(&created),
            modified: filetime_to_unixepoch(&modified),
            sector_id,
            directory_size,
            reserved,
        };

        dir_entries.push(directory);
    }
    Ok((input, dir_entries))
}

/// Determine OLE Directory type
fn parse_directory_type(dir_type: &u8) -> DirectoryType {
    match dir_type {
        0 => DirectoryType::Empty,
        1 => DirectoryType::Storage,
        2 => DirectoryType::Stream,
        3 => DirectoryType::LockBytes,
        4 => DirectoryType::Property,
        5 => DirectoryType::Root,
        _ => DirectoryType::Unknown,
    }
}

#[cfg(test)]
mod tests {
    use super::{assemble_ole_data, parse_directory};
    use crate::artifacts::os::windows::ole::directory::{parse_directory_type, DirectoryType};
    use crate::artifacts::os::windows::ole::header::OleHeader;
    use crate::artifacts::os::windows::ole::sat::assemble_sat_data;
    use crate::filesystem::files::read_file;
    use std::path::PathBuf;

    #[test]
    fn test_assemble_ole_data() {
        let mut test_location = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
        test_location.push(
            "tests/test_data/dfir/windows/jumplists/win7/1b4dd67f29cb1962.automaticDestinations-ms",
        );
        let data = read_file(&test_location.display().to_string()).unwrap();

        let (input, header) = OleHeader::parse_header(&data).unwrap();
        let size: u32 = 2;
        let (_, sat) = assemble_sat_data(
            input,
            &header.msat_sectors,
            size.pow(header.sector_size as u32),
        )
        .unwrap();

        let size: u32 = 2;
        let (_, result) = assemble_ole_data(
            input,
            &sat,
            header.sector_id_chain,
            size.pow(header.sector_size as u32),
        )
        .unwrap();

        assert_eq!(result.len(), 1024);
    }

    #[test]
    fn test_parse_directory() {
        let mut test_location = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
        test_location.push(
            "tests/test_data/dfir/windows/jumplists/win7/1b4dd67f29cb1962.automaticDestinations-ms",
        );
        let data = read_file(&test_location.display().to_string()).unwrap();

        let (input, header) = OleHeader::parse_header(&data).unwrap();
        let size: u32 = 2;
        let (_, sat) = assemble_sat_data(
            input,
            &header.msat_sectors,
            size.pow(header.sector_size as u32),
        )
        .unwrap();

        let size: u32 = 2;
        let (_, dir_data) = assemble_ole_data(
            input,
            &sat,
            header.sector_id_chain,
            size.pow(header.sector_size as u32),
        )
        .unwrap();

        let (_, result) = parse_directory(&dir_data).unwrap();
        assert_eq!(result.len(), 8);
        assert_eq!(result[0].created, -11644473600);
        assert_eq!(result[0].modified, 1452975805);
        assert_eq!(result[1].name, "1");
        assert_eq!(result[1].directory_size, 411);
    }

    #[test]
    fn test_parse_directory_type() {
        let test = 1;
        let result = parse_directory_type(&test);
        assert_eq!(result, DirectoryType::Storage);
    }
}
