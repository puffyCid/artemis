use super::items::ShellItem;
use crate::artifacts::os::windows::shellitems::items::ShellType::RootFolder;
use crate::utils::uuid::format_guid_le_bytes;
use nom::bytes::complete::take;
use std::mem::size_of;

/// Parse a `Root` `ShellItem` type. Contains a GUID and optional 0xbeefXXXX data. Currently only GUID is returned
pub(crate) fn parse_root(data: &[u8]) -> nom::IResult<&[u8], ShellItem> {
    let (input, _sort_index) = take(size_of::<u8>())(data)?;

    let (input, guid) = take(size_of::<u128>())(input)?;
    let guid_string = format_guid_le_bytes(guid);
    let root_item = ShellItem {
        value: guid_string,
        shell_type: RootFolder,
        created: 0,
        modified: 0,
        accessed: 0,
        mft_entry: 0,
        mft_sequence: 0,
        stores: Vec::new(),
    };

    // There may also be 0xbeef0026, 0xbeef0017, or 0xbeef0025 data
    Ok((input, root_item))
}

#[cfg(test)]
mod tests {
    use super::parse_root;
    use crate::artifacts::os::windows::shellitems::items::ShellType;

    #[test]
    fn test_parse_root() {
        let test_data = [
            128, 203, 133, 159, 103, 32, 2, 128, 64, 178, 155, 85, 64, 204, 5, 170, 182, 0, 0,
        ];

        let (_, result) = parse_root(&test_data).unwrap();
        assert_eq!(result.value, "679f85cb-0220-4080-b29b-5540cc05aab6");
        assert_eq!(result.shell_type, ShellType::RootFolder);
        assert_eq!(result.mft_sequence, 0);
        assert_eq!(result.mft_entry, 0);
        assert_eq!(result.created, 0);
        assert_eq!(result.modified, 0);
        assert_eq!(result.accessed, 0);
    }
}
