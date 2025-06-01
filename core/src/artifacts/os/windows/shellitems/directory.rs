use super::beef::beef0004;
use crate::utils::{
    strings::{extract_ascii_utf16_string, extract_utf8_string},
    time::{fattime_utc_to_unixepoch, unixepoch_to_iso},
};
use common::windows::{ShellItem, ShellType};
use nom::{
    Parser,
    bytes::complete::{take, take_until},
    combinator::peek,
};
use std::mem::size_of;

/// Parse a `Directory ShellItem`. The most common `ShellItem` type
pub(crate) fn parse_directory(data: &[u8]) -> nom::IResult<&[u8], ShellItem> {
    let (input, _unknown) = take(size_of::<u8>())(data)?;
    let (input, _file_size) = take(size_of::<u32>())(input)?;

    let (input, modified_data) = take(size_of::<u32>())(input)?;
    let (input, _attribute_flags) = take(size_of::<u16>())(input)?;

    // Primary name is either ASCII or UTF16. No size is given for name size. But the next directory `ShellItem` data is the signature 0xBEEF0004
    // We peek until we find the signature without nomming the input
    if let Ok((input, mut directory_item)) = check_beef(input) {
        directory_item.modified = unixepoch_to_iso(&fattime_utc_to_unixepoch(modified_data));

        return Ok((input, directory_item));
    }

    // No beef0004 signature found. We should have end of string character then
    // Check UTF16 first
    if let Ok((input, mut directory_item)) = check_utf16(input) {
        directory_item.modified = unixepoch_to_iso(&fattime_utc_to_unixepoch(modified_data));
        // Skipping 8.3 filename string that is after the UTF16 string
        return Ok((input, directory_item));
    }

    let (input, string_data) = take_until([0].as_slice()).parse(input)?;
    let name = extract_utf8_string(string_data);
    let directory_item = ShellItem {
        value: name,
        shell_type: ShellType::Directory,
        created: String::from("1970-01-01T00:00:00Z"),
        modified: unixepoch_to_iso(&fattime_utc_to_unixepoch(modified_data)),
        accessed: String::from("1970-01-01T00:00:00Z"),
        mft_entry: 0,
        mft_sequence: 0,
        stores: Vec::new(),
    };

    Ok((input, directory_item))
}

/// Check for beef0004 signature. It may not exist sometimes
fn check_beef(data: &[u8]) -> nom::IResult<&[u8], ShellItem> {
    let (input, primary_name_start) = peek(take_until([4, 0, 239, 190].as_slice())).parse(data)?;

    // Next 4 bytes after the primary name is metadata related to the signature
    let adjust_size = 4;
    let primary_name_size = primary_name_start.len() - adjust_size;

    let (input, _primary_name_data) = take(primary_name_size)(input)?;
    let (input, directory_item) = beef0004::parse_beef(input, ShellType::Directory)?;

    Ok((input, directory_item))
}

/// Check if string is UTF16. Should have end of string characters
fn check_utf16(data: &[u8]) -> nom::IResult<&[u8], ShellItem> {
    let (input, string_data) = take_until([0, 0].as_slice()).parse(data)?;
    let name = extract_ascii_utf16_string(string_data);
    let directory_item = ShellItem {
        value: name,
        shell_type: ShellType::Directory,
        created: String::from("1970-01-01T00:00:00Z"),
        modified: String::from("1970-01-01T00:00:00Z"),
        accessed: String::from("1970-01-01T00:00:00Z"),
        mft_entry: 0,
        mft_sequence: 0,
        stores: Vec::new(),
    };

    Ok((input, directory_item))
}

#[cfg(test)]
mod tests {
    use super::parse_directory;
    use common::windows::ShellType;

    #[test]
    fn test_parse_directory_win10() {
        let test_data = [
            0, 0, 0, 0, 0, 123, 79, 195, 14, 16, 0, 82, 69, 71, 82, 73, 80, 126, 49, 46, 56, 45,
            77, 0, 0, 88, 0, 9, 0, 4, 0, 239, 190, 123, 79, 195, 14, 123, 79, 195, 14, 46, 0, 0, 0,
            225, 9, 0, 0, 0, 0, 15, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 38, 46, 73, 0, 82,
            0, 101, 0, 103, 0, 82, 0, 105, 0, 112, 0, 112, 0, 101, 0, 114, 0, 50, 0, 46, 0, 56, 0,
            45, 0, 109, 0, 97, 0, 115, 0, 116, 0, 101, 0, 114, 0, 0, 0, 28, 0, 0, 0,
        ];

        let (_, result) = parse_directory(&test_data).unwrap();
        assert_eq!(result.value, "RegRipper2.8-master");
        assert_eq!(result.shell_type, ShellType::Directory);
        assert_eq!(result.mft_sequence, 15);
        assert_eq!(result.mft_entry, 2529);
        assert_eq!(result.created, "2019-11-27T01:54:06.000Z");
        assert_eq!(result.modified, "2019-11-27T01:54:06.000Z");
        assert_eq!(result.accessed, "2019-11-27T01:54:06.000Z");
    }

    #[test]
    fn test_parse_directory_no_beef() {
        let test_data = [
            0, 0, 0, 0, 0, 0, 0, 0, 0, 16, 0, 87, 105, 110, 100, 111, 119, 115, 0,
        ];
        let (_, result) = parse_directory(&test_data).unwrap();
        assert_eq!(result.value, "Windows");
        assert_eq!(result.modified, "1970-01-01T00:00:00.000Z");
        assert_eq!(result.shell_type, ShellType::Directory);
    }

    #[test]
    fn test_parse_directory_no_beef_utf16() {
        let test_data = [
            0, 0, 0, 0, 0, 0, 0, 0, 0, 16, 0, 68, 0, 111, 0, 99, 0, 117, 0, 109, 0, 101, 0, 110, 0,
            116, 0, 115, 0, 0, 0, 68, 79, 67, 85, 77, 69, 126, 49, 0,
        ];

        let (_, result) = parse_directory(&test_data).unwrap();
        assert_eq!(result.value, "Documents");
        assert_eq!(result.modified, "1970-01-01T00:00:00.000Z");
        assert_eq!(result.shell_type, ShellType::Directory);
    }

    #[test]
    fn test_parse_directory_ascii_utf16() {
        let test_data = [
            0, 0, 0, 0, 0, 0, 0, 0, 0, 16, 0, 79, 110, 101, 68, 114, 105, 118, 101, 0, 0,
        ];

        let (_, result) = parse_directory(&test_data).unwrap();
        assert_eq!(result.value, "OneDrive");
        assert_eq!(result.modified, "1970-01-01T00:00:00.000Z");
        assert_eq!(result.shell_type, ShellType::Directory);
    }
}
