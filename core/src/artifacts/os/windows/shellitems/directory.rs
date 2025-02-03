use super::beef::beef0004;
use crate::utils::time::{fattime_utc_to_unixepoch, unixepoch_to_iso};
use common::windows::{ShellItem, ShellType};
use nom::{
    bytes::complete::{take, take_until},
    combinator::peek,
    Parser,
};
use std::mem::size_of;

/// Parse a `Directory` `ShellItem`. The most common `ShellItem` type
pub(crate) fn parse_directory(data: &[u8]) -> nom::IResult<&[u8], ShellItem> {
    let (input, _unknown) = take(size_of::<u8>())(data)?;
    let (input, _file_size) = take(size_of::<u32>())(input)?;

    let (input, modified_data) = take(size_of::<u32>())(input)?;
    let (input, _attribute_flags) = take(size_of::<u16>())(input)?;

    // Primary name is either ASCII or UTF16. No size is given for name size. But the next directory `ShellItem` data is the signature 0xBEEF0004
    // We peek until we find the signature without nomming the input
    let (input, primary_name_start) = peek(take_until([4, 0, 239, 190].as_slice())).parse(input)?;

    // Next 4 bytes after the primary name is metadata related to the signature
    let adjust_size = 4;
    let primary_name_size = primary_name_start.len() - adjust_size;

    let (input, _primary_name_data) = take(primary_name_size)(input)?;

    let (input, mut directory_item) = beef0004::parse_beef(input, ShellType::Directory)?;
    directory_item.modified = unixepoch_to_iso(&fattime_utc_to_unixepoch(modified_data));

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
}
