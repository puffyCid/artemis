use crate::utils::nom_helper::{
    nom_unsigned_eight_bytes, nom_unsigned_four_bytes, nom_unsigned_one_byte,
    nom_unsigned_two_bytes, Endian,
};
use crate::utils::strings::{extract_ascii_utf16_string, extract_utf16_string};
use crate::utils::time::filetime_to_unixepoch;
use common::windows::ShellItem;
use common::windows::ShellType::Uri;
use nom::bytes::complete::{take, take_while};
use std::mem::size_of;

/// Parse a `URI` `ShellItem`. Often related to browsing FTP servers with Explorer
pub(crate) fn parse_uri(data: &[u8]) -> nom::IResult<&[u8], ShellItem> {
    let (input, flag) = nom_unsigned_one_byte(data, Endian::Le)?;
    let (input, size) = nom_unsigned_two_bytes(input, Endian::Le)?;

    let mut uri_item = ShellItem {
        value: String::new(),
        shell_type: Uri,
        created: 0,
        modified: 0,
        accessed: 0,
        mft_entry: 0,
        mft_sequence: 0,
        stores: Vec::new(),
    };
    let empty_uri = 0;

    if size == empty_uri {
        let has_uri = 0x80;
        if flag == has_uri {
            let (uri_start, _) = take_while(|b| b == 0)(input)?;
            uri_item.value = extract_utf16_string(uri_start);
        }
        return Ok((input, uri_item));
    }

    let (input, _unknown) = take(size_of::<u32>())(input)?;
    let (input, _unknown2) = take(size_of::<u32>())(input)?;

    let (input, access) = nom_unsigned_eight_bytes(input, Endian::Le)?;
    uri_item.accessed = filetime_to_unixepoch(&access);

    let (input, _unknown3) = take(size_of::<u32>())(input)?;

    let unknown_size: u8 = 12;
    let (input, _unknown4) = take(unknown_size)(input)?;
    let (input, _unknown5) = take(size_of::<u32>())(input)?;

    let (input, data_size) = nom_unsigned_four_bytes(input, Endian::Le)?;
    let (input, string_data) = take(data_size)(input)?;
    uri_item.value = extract_ascii_utf16_string(string_data);

    let (input, data_size) = nom_unsigned_four_bytes(input, Endian::Le)?;

    let (input, string_data) = take(data_size)(input)?;
    let _username = extract_ascii_utf16_string(string_data);

    let (input, data_size) = nom_unsigned_four_bytes(input, Endian::Le)?;

    let (input, string_data) = take(data_size)(input)?;
    let _password = extract_ascii_utf16_string(string_data);

    Ok((input, uri_item))
}

#[cfg(test)]
mod tests {
    use super::parse_uri;
    use common::windows::ShellType;

    #[test]
    fn test_parse_uri() {
        let test_data = [
            3, 100, 0, 3, 39, 0, 0, 4, 0, 0, 0, 179, 12, 170, 178, 225, 1, 215, 1, 0, 0, 0, 0, 0,
            0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 21, 0, 0, 0, 16, 0, 0, 0, 102, 116, 112, 46, 100, 108,
            112, 116, 101, 115, 116, 46, 99, 111, 109, 0, 8, 0, 0, 0, 100, 108, 112, 117, 115, 101,
            114, 0, 28, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0,
            0, 0, 0, 0, 0, 0, 102, 116, 112, 0, 0, 0,
        ];

        let (_, result) = parse_uri(&test_data).unwrap();
        assert_eq!(result.value, "ftp.dlptest.com");
        assert_eq!(result.shell_type, ShellType::Uri);
        assert_eq!(result.mft_sequence, 0);
        assert_eq!(result.mft_entry, 0);
        assert_eq!(result.created, 0);
        assert_eq!(result.modified, 0);
        assert_eq!(result.accessed, 1613204690);
    }

    #[test]
    fn test_parse_uri_string() {
        let test = [
            128, 0, 0, 0, 0, 119, 0, 105, 0, 110, 0, 100, 0, 111, 0, 119, 0, 115, 0, 100, 0, 101,
            0, 102, 0, 101, 0, 110, 0, 100, 0, 101, 0, 114, 0, 58, 0, 47, 0, 47, 0, 116, 0, 104, 0,
            114, 0, 101, 0, 97, 0, 116, 0, 47, 0, 0, 0, 0, 0, 0, 0,
        ];
        let (_, result) = parse_uri(&test).unwrap();
        assert_eq!(result.value, "windowsdefender://threat/");
        assert_eq!(result.shell_type, ShellType::Uri);
        assert_eq!(result.mft_sequence, 0);
        assert_eq!(result.mft_entry, 0);
        assert_eq!(result.created, 0);
        assert_eq!(result.modified, 0);
        assert_eq!(result.accessed, 0);
    }
}
