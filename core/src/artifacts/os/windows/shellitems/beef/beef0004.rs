use crate::utils::nom_helper::{nom_unsigned_two_bytes, Endian};
use crate::utils::{
    strings::{extract_ascii_utf16_string, extract_utf16_string},
    time::fattime_utc_to_unixepoch,
};
use byteorder::{LittleEndian, ReadBytesExt};
use common::windows::{ShellItem, ShellType};
use nom::Needed;
use nom::{
    bytes::complete::{take, take_until},
    combinator::peek,
};
use std::mem::size_of;

/// Parse a 0xbeef0004 block. Contains a file/directory name, FAT timestamps, and MFT metadata
pub(crate) fn parse_beef(data: &[u8], shell_type: ShellType) -> nom::IResult<&[u8], ShellItem> {
    let (input, sig_size) = nom_unsigned_two_bytes(data, Endian::Le)?;

    // Size includes size itself
    let adjust_size = 2;
    if sig_size < adjust_size {
        return Err(nom::Err::Incomplete(Needed::Unknown));
    }
    let (remaining_data, input) = take(sig_size - adjust_size)(input)?;
    let (input, version) = nom_unsigned_two_bytes(input, Endian::Le)?;
    let (input, _signature) = take(size_of::<u32>())(input)?;

    let (input, created_data) = take(size_of::<u32>())(input)?;
    let (input, accessed_data) = take(size_of::<u32>())(input)?;
    let (mut input, _unknown2) = take(size_of::<u16>())(input)?;

    let mut directory_item = ShellItem {
        value: String::new(),
        shell_type,
        created: fattime_utc_to_unixepoch(created_data),
        modified: 0,
        accessed: fattime_utc_to_unixepoch(accessed_data),
        mft_entry: 0,
        mft_sequence: 0,
        stores: Vec::new(),
    };

    let vista_version = 7;
    if version >= vista_version {
        let (vista_input, _unknown3) = take(size_of::<u16>())(input)?;
        let entry_size: u8 = 6;
        let (vista_input, mut entry_data) = take(entry_size)(vista_input)?;
        let (vista_input, mft_seq) = nom_unsigned_two_bytes(vista_input, Endian::Le)?;

        directory_item.mft_entry = entry_data.read_u48::<LittleEndian>().unwrap_or(0);
        directory_item.mft_sequence = mft_seq;

        let (vista_input, _unknown4) = take(size_of::<u64>())(vista_input)?;
        input = vista_input;
    }

    let mut long_size = 0;
    let xp_version = 3;
    if version >= xp_version {
        // Long string size is often zero (0)
        let (xp_input, size) = nom_unsigned_two_bytes(input, Endian::Le)?;
        long_size = size;
        input = xp_input;
    }

    let win10_version = 9;
    if version >= win10_version {
        let (win10_input, _unknown5) = take(size_of::<u32>())(input)?;
        input = win10_input;
    }

    let win7_version = 8;
    if version >= win7_version {
        let (win7_input, _unknown6) = take(size_of::<u32>())(input)?;
        input = win7_input;
    }

    if version >= xp_version {
        // Just like Primary name no size is given for the long name (UTF16)
        // Peek until we get end of string character
        let (_, long_string_end) = peek(take_until([0, 0, 0].as_slice()))(input)?;
        let adjust_string_size = 3;
        let name_size = long_string_end.len() + adjust_string_size; // Make sure to include end of string character
        let (long_name_input, name_data) = take(name_size)(input)?;
        let name = extract_utf16_string(name_data);
        directory_item.value = name;
        input = long_name_input;
    }

    let no_local_name = 0;
    if version >= xp_version && long_size != no_local_name {
        let _local_name = extract_ascii_utf16_string(input);
    }
    Ok((remaining_data, directory_item))
}

#[cfg(test)]
mod tests {
    use crate::artifacts::os::windows::shellitems::beef::beef0004::parse_beef;
    use common::windows::ShellType;

    #[test]
    fn test_parse_beef() {
        let test_data = [
            68, 0, 9, 0, 4, 0, 239, 190, 196, 78, 156, 189, 110, 82, 80, 173, 46, 0, 0, 0, 62, 128,
            5, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 68, 0, 111, 0,
            119, 0, 110, 0, 108, 0, 111, 0, 97, 0, 100, 0, 115, 0, 0, 0, 24, 0, 0, 0,
        ];
        let (_, result) = parse_beef(&test_data, ShellType::Directory).unwrap();
        assert_eq!(result.value, "Downloads");
        assert_eq!(result.shell_type, ShellType::Directory);
        assert_eq!(result.mft_sequence, 0);
        assert_eq!(result.mft_entry, 360510);
        assert_eq!(result.created, 1559691896);
        assert_eq!(result.modified, 0);
        assert_eq!(result.accessed, 1615758152);
    }
}
