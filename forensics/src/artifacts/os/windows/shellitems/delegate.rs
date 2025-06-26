use super::beef::beef0004;
use crate::utils::time::{fattime_utc_to_unixepoch, unixepoch_to_iso};
use crate::utils::uuid::format_guid_le_bytes;
use common::windows::{ShellItem, ShellType};
use nom::Parser;
use nom::{
    bytes::complete::{take, take_until},
    combinator::peek,
};
use std::mem::size_of;

#[derive(Debug)]
pub(crate) struct DelegateItem {
    pub(crate) value: String,
    pub(crate) shell_type: ShellType,
    pub(crate) created: String,  // FAT time
    pub(crate) modified: String, // FAT time
    pub(crate) accessed: String, // FAT time
    pub(crate) mft_entry: u64,
    pub(crate) mft_sequence: u16,
    pub(crate) _delegate_guid: String,
    pub(crate) _class_id: String,
}

/// Parse a `Delegate` `ShellItem` type and return a generic `ShellItem` structure
pub(crate) fn get_delegate_shellitem(data: &[u8]) -> nom::IResult<&[u8], ShellItem> {
    let (input, delegate_item) = parse_delegate(data)?;

    let item = ShellItem {
        value: delegate_item.value,
        shell_type: delegate_item.shell_type,
        created: delegate_item.created,
        modified: delegate_item.modified,
        accessed: delegate_item.accessed,
        mft_entry: delegate_item.mft_entry,
        mft_sequence: delegate_item.mft_sequence,
        stores: Vec::new(),
    };

    Ok((input, item))
}

/// Parse a `Delegate` `ShellItem` type and return a Delegate structure. Same as `ShellItem` structure with addition of GUID and ID
pub(crate) fn parse_delegate(data: &[u8]) -> nom::IResult<&[u8], DelegateItem> {
    let (input, _unknown) = take(size_of::<u8>())(data)?;
    let (input, _unknown_size) = take(size_of::<u16>())(input)?;
    let (input, _sig) = take(size_of::<u32>())(input)?;

    let (input, _shell_item_size) = take(size_of::<u16>())(input)?;
    let (input, _indicator) = take(size_of::<u8>())(input)?;
    let (input, _unknown2) = take(size_of::<u8>())(input)?;

    let (input, _file_size) = take(size_of::<u32>())(input)?;
    let (input, modified_data) = take(size_of::<u32>())(input)?;
    let (input, _attribute_flags) = take(size_of::<u16>())(input)?;

    // Primary name is either ASCII or UTF16. No size is given for name size. But the next directory shellitem data is the signature 0xBEEF0004
    // We peek until we find the signature without nomming the input
    let (input, primary_name_start) = peek(take_until([4, 0, 239, 190].as_slice())).parse(input)?;

    // Next 38 bytes after the primary name is unknown data, two GUIDs, and BEEF004 metadata
    let adjust_size = 38;
    let primary_name_size = primary_name_start.len() - adjust_size;

    let (input, _primary_name_data) = take(primary_name_size)(input)?;
    let (input, _unknown3) = take(size_of::<u16>())(input)?;

    let (input, guid_bytes) = take(size_of::<u128>())(input)?;
    let delegate_guid = format_guid_le_bytes(guid_bytes);

    let (input, guid_bytes) = take(size_of::<u128>())(input)?;
    let class_id = format_guid_le_bytes(guid_bytes);

    let (input, mut directory_item) = beef0004::parse_beef(input, ShellType::Delegate)?;
    directory_item.modified = unixepoch_to_iso(fattime_utc_to_unixepoch(modified_data));

    let delegate_item = DelegateItem {
        value: directory_item.value,
        shell_type: directory_item.shell_type,
        created: directory_item.created,
        modified: directory_item.modified,
        accessed: directory_item.accessed,
        mft_entry: directory_item.mft_entry,
        mft_sequence: directory_item.mft_sequence,
        _delegate_guid: delegate_guid,
        _class_id: class_id,
    };

    Ok((input, delegate_item))
}

#[cfg(test)]
mod tests {
    use crate::artifacts::os::windows::shellitems::delegate::{
        get_delegate_shellitem, parse_delegate,
    };
    use common::windows::ShellType;

    #[test]
    fn test_parse_delegate() {
        let test_data = [
            0, 28, 0, 67, 70, 83, 70, 22, 0, 49, 0, 0, 0, 0, 0, 85, 79, 20, 189, 16, 0, 115, 111,
            117, 114, 99, 101, 0, 0, 0, 0, 116, 26, 89, 94, 150, 223, 211, 72, 141, 103, 23, 51,
            188, 238, 40, 186, 197, 205, 250, 223, 159, 103, 86, 65, 137, 71, 197, 199, 107, 192,
            182, 127, 62, 0, 9, 0, 4, 0, 239, 190, 85, 79, 20, 189, 85, 79, 20, 189, 46, 0, 0, 0,
            58, 63, 4, 0, 0, 0, 12, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 137, 188, 35, 0,
            115, 0, 111, 0, 117, 0, 114, 0, 99, 0, 101, 0, 0, 0, 66, 0, 0, 0,
        ];

        let (remaining, result) = parse_delegate(&test_data).unwrap();
        assert_eq!(result.value, "source");
        assert_eq!(result.shell_type, ShellType::Delegate);
        assert_eq!(result.mft_sequence, 12);
        assert_eq!(result.mft_entry, 278330);
        assert_eq!(result.created, "2019-10-21T23:40:40.000Z");
        assert_eq!(result.modified, "2019-10-21T23:40:40.000Z");
        assert_eq!(result.accessed, "2019-10-21T23:40:40.000Z");
        assert_eq!(
            result._delegate_guid,
            "5e591a74-df96-48d3-8d67-1733bcee28ba"
        );
        assert_eq!(result._class_id, "dffacdc5-679f-4156-8947-c5c76bc0b67f");
        assert_eq!(remaining, [0, 0]);
    }

    #[test]
    fn test_get_delegate_shellitem() {
        let test_data = [
            0, 28, 0, 67, 70, 83, 70, 22, 0, 49, 0, 0, 0, 0, 0, 85, 79, 20, 189, 16, 0, 115, 111,
            117, 114, 99, 101, 0, 0, 0, 0, 116, 26, 89, 94, 150, 223, 211, 72, 141, 103, 23, 51,
            188, 238, 40, 186, 197, 205, 250, 223, 159, 103, 86, 65, 137, 71, 197, 199, 107, 192,
            182, 127, 62, 0, 9, 0, 4, 0, 239, 190, 85, 79, 20, 189, 85, 79, 20, 189, 46, 0, 0, 0,
            58, 63, 4, 0, 0, 0, 12, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 137, 188, 35, 0,
            115, 0, 111, 0, 117, 0, 114, 0, 99, 0, 101, 0, 0, 0, 66, 0, 0, 0,
        ];

        let (remaining, result) = get_delegate_shellitem(&test_data).unwrap();
        assert_eq!(result.value, "source");
        assert_eq!(result.shell_type, ShellType::Delegate);
        assert_eq!(result.mft_sequence, 12);
        assert_eq!(result.mft_entry, 278330);
        assert_eq!(result.created, "2019-10-21T23:40:40.000Z");
        assert_eq!(result.modified, "2019-10-21T23:40:40.000Z");
        assert_eq!(result.accessed, "2019-10-21T23:40:40.000Z");
        assert_eq!(remaining, [0, 0]);
    }
}
